//! Los dobles de los puertos de `site`: las ranuras de la CA local en memoria y los certificados tal como los ve un trámite.

use std::path::Path;
use std::sync::Mutex;

use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::NoMemory;
use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::tls_error::{Situation as TlsSituation, TlsError};
use crate::site::ports::{Certificates, LocalCaSlots};

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

/// Los certificados de la persona tal como los ve un trámite: los dados, con sus asas ya acuñadas.
pub(crate) struct Directory<'a> {
    pub(crate) certificates: Vec<TokenCertificate>,
    pub(crate) listed: &'a ListedCertificates,
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
            &NoMemory,
        )
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        crate::identity::application::certificates::usable_certificate(found, handle, self.listed)
    }
}
