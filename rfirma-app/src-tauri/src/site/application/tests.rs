//! Los dobles de los puertos de `site`: las ranuras de la CA local en memoria, los servlets del servidor intermedio y los certificados tal como los ve un trámite.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use crate::identity::application::certificates::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::ports::CertificateMemory;
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::relay_error::{RelayError, Situation as RelaySituation};
use crate::site::domain::tls_error::{Situation as TlsSituation, TlsError};
use crate::site::ports::{Certificates, LocalCaSlots, Servlets};

/// Las dos ranuras de la CA local en memoria, escribibles o no.
#[derive(Default)]
pub(crate) struct InMemoryCaSlots {
    serving: Mutex<Option<LocalCa>>,
    next: Mutex<Option<LocalCa>>,
    unwritable: bool,
}

impl InMemoryCaSlots {
    /// Unas ranuras en las que no se puede escribir, como un disco de solo lectura.
    pub(crate) fn unwritable() -> Self {
        Self {
            unwritable: true,
            ..Self::default()
        }
    }

    fn writing(&self) -> Result<(), TlsError> {
        if self.unwritable {
            return Err(TlsError::new(
                TlsSituation::MaterialUnwritable,
                "estas ranuras no dejan escribir",
            ));
        }
        Ok(())
    }
}

impl LocalCaSlots for InMemoryCaSlots {
    fn serving(&self) -> Result<Option<LocalCa>, TlsError> {
        Ok(crate::lock(&self.serving).clone())
    }

    fn write_serving(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.writing()?;
        *crate::lock(&self.serving) = Some(ca.clone());
        Ok(())
    }

    fn next(&self) -> Result<Option<LocalCa>, TlsError> {
        Ok(crate::lock(&self.next).clone())
    }

    fn write_next(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.writing()?;
        *crate::lock(&self.next) = Some(ca.clone());
        Ok(())
    }

    fn promote_next(&self) -> Result<Option<LocalCa>, TlsError> {
        self.writing()?;
        let promoted = crate::lock(&self.next).take();
        if let Some(ca) = &promoted {
            *crate::lock(&self.serving) = Some(ca.clone());
        }
        Ok(promoted)
    }

    fn forget_next(&self) -> Result<(), TlsError> {
        self.writing()?;
        *crate::lock(&self.next) = None;
        Ok(())
    }
}

/// Los servlets del servidor intermedio en memoria: lo guardado por identificador, y las esperas pedidas.
#[derive(Default)]
pub(crate) struct InMemoryServlets {
    stored: Mutex<BTreeMap<String, String>>,
    waited: Mutex<Vec<String>>,
    unreachable: bool,
}

impl InMemoryServlets {
    /// Unos servlets que nunca responden, como si la sede no tuviera red.
    pub(crate) fn unreachable() -> Self {
        Self {
            unreachable: true,
            ..Self::default()
        }
    }

    /// Los identificadores por los que se pidió esperar, en el orden en que se pidieron.
    pub(crate) fn waited_ids(&self) -> Vec<String> {
        crate::lock(&self.waited).clone()
    }

    fn reaching(&self) -> Result<(), RelayError> {
        if self.unreachable {
            return Err(RelayError::new(
                RelaySituation::ServletUnreachable,
                "este servlet no responde",
            ));
        }
        Ok(())
    }
}

impl Servlets for InMemoryServlets {
    fn retrieve(&self, _service_url: &str, id: &str) -> Result<String, RelayError> {
        self.reaching()?;
        crate::lock(&self.stored).get(id).cloned().ok_or_else(|| {
            RelayError::new(
                RelaySituation::ServletUnreachable,
                "no hay datos guardados para ese identificador",
            )
        })
    }

    fn store(&self, _service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        self.reaching()?;
        crate::lock(&self.stored).insert(id.to_owned(), data.to_owned());
        Ok(())
    }

    fn wait(&self, _service_url: &str, id: &str) -> Result<(), RelayError> {
        self.reaching()?;
        crate::lock(&self.waited).push(id.to_owned());
        Ok(())
    }
}

/// Los certificados de la persona tal como los ve un trámite: los dados, con sus asas ya acuñadas.
pub(crate) struct Directory<'a> {
    pub(crate) certificates: Vec<TokenCertificate>,
    pub(crate) listed: &'a ListedCertificates,
    pub(crate) memory: &'a dyn CertificateMemory,
}

impl Certificates for Directory<'_> {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.certificates.clone())
    }

    fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        crate::identity::application::certificates::rows_of(
            found,
            Path::new("/no/hay/instalados"),
            self.listed,
            self.memory,
        )
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        crate::identity::application::certificates::usable_certificate(found, handle, self.listed)
    }

    fn remembered(&self) -> Option<CertificateRef> {
        self.memory.remembered_certificate()
    }

    fn remember(&self, chosen: &CertificateRef) {
        crate::identity::application::certificates::remember_the_certificate(self.memory, chosen);
    }

    fn forget_the_remembered(&self) {
        crate::identity::application::certificates::forget_the_certificate(self.memory);
    }
}
