//! Los dobles de los puertos de `site`: las ranuras de la CA local en memoria, los servlets del servidor intermedio y los certificados tal como los ve un trámite.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use crate::identity::application::certificates::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::ports::CertificateMemory;
use crate::signing::domain::bridge::{BridgeError, Format, SignatureVerdict};
use crate::site::domain::batch::{BatchFormat, TriphaseData};
use crate::site::domain::batch_error::{BatchError, Situation as BatchSituation};
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::protocol::{DataSource, Refusal, SiteOperation};
use crate::site::domain::relay_error::{RelayError, Situation as RelaySituation};
use crate::site::domain::signing::SigningRefusal;
use crate::site::domain::tls_error::{Situation as TlsSituation, TlsError};
use crate::site::ports::{
    BatchServices, Certificates, LocalCaSlots, Servlets, TokenSigning, ValidationEngine,
};

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

/// Una llamada recibida por los servlets del lote remoto en memoria.
pub(crate) enum ReceivedBatchCall {
    /// Lo que llegó a `presign`.
    Presign {
        url: String,
        format: BatchFormat,
        lote_base64: String,
        certs: Vec<Vec<u8>>,
    },
    /// Lo que llegó a `postsign`.
    Postsign {
        url: String,
        format: BatchFormat,
        lote_base64: String,
        certs: Vec<Vec<u8>>,
        tridata: TriphaseData,
    },
}

/// Los servlets del lote remoto en memoria: guardan lo recibido y devuelven lo configurado.
#[derive(Default)]
pub(crate) struct InMemoryBatchServices {
    presign_response: Mutex<Option<Vec<u8>>>,
    postsign_response: Mutex<Option<Vec<u8>>>,
    received: Mutex<Vec<ReceivedBatchCall>>,
    unreachable: bool,
}

impl InMemoryBatchServices {
    /// Unos servlets que responden lo dado a cada verbo.
    pub(crate) fn answering(presign: Vec<u8>, postsign: Vec<u8>) -> Self {
        Self {
            presign_response: Mutex::new(Some(presign)),
            postsign_response: Mutex::new(Some(postsign)),
            ..Self::default()
        }
    }

    /// Unos servlets que prefirman y luego no devuelven postfirma ninguna.
    pub(crate) fn only_presigning(presign: Vec<u8>) -> Self {
        Self {
            presign_response: Mutex::new(Some(presign)),
            ..Self::default()
        }
    }

    /// Unos servlets que nunca responden, como si la sede no tuviera red.
    pub(crate) fn unreachable() -> Self {
        Self {
            unreachable: true,
            ..Self::default()
        }
    }

    /// Las llamadas recibidas, en el orden en que llegaron.
    pub(crate) fn received(&self) -> std::sync::MutexGuard<'_, Vec<ReceivedBatchCall>> {
        crate::lock(&self.received)
    }

    fn reaching(&self, unreachable: BatchSituation) -> Result<(), BatchError> {
        if self.unreachable {
            return Err(BatchError::new(unreachable, "este servlet no responde"));
        }
        Ok(())
    }
}

impl BatchServices for InMemoryBatchServices {
    fn presign(
        &self,
        url: &str,
        format: BatchFormat,
        lote_base64: &str,
        certs: &[Vec<u8>],
    ) -> Result<Vec<u8>, BatchError> {
        self.reaching(BatchSituation::PresignerUnreachable)?;
        crate::lock(&self.received).push(ReceivedBatchCall::Presign {
            url: url.to_owned(),
            format,
            lote_base64: lote_base64.to_owned(),
            certs: certs.to_vec(),
        });
        crate::lock(&self.presign_response).clone().ok_or_else(|| {
            BatchError::new(
                BatchSituation::InvalidPresignResponse,
                "sin respuesta configurada",
            )
        })
    }

    fn postsign(
        &self,
        url: &str,
        format: BatchFormat,
        lote_base64: &str,
        certs: &[Vec<u8>],
        tridata: &TriphaseData,
    ) -> Result<Vec<u8>, BatchError> {
        self.reaching(BatchSituation::PostsignerUnreachable)?;
        crate::lock(&self.received).push(ReceivedBatchCall::Postsign {
            url: url.to_owned(),
            format,
            lote_base64: lote_base64.to_owned(),
            certs: certs.to_vec(),
            tridata: tridata.clone(),
        });
        crate::lock(&self.postsign_response).clone().ok_or_else(|| {
            BatchError::new(
                BatchSituation::InvalidPostsignResponse,
                "sin respuesta configurada",
            )
        })
    }
}

/// El token en memoria del lote: cuenta los secretos y los intentos de firma, y guarda lo que firmó.
#[derive(Default)]
pub(crate) struct InMemoryTokenSigning {
    secrets_asked: Mutex<usize>,
    attempts: Mutex<usize>,
    signed: Mutex<Vec<(String, Vec<u8>)>>,
    refusing: Option<SigningRefusal>,
}

impl InMemoryTokenSigning {
    /// Un token que niega toda firma con esa negativa ya traducida.
    pub(crate) fn refusing(refusal: SigningRefusal) -> Self {
        Self {
            refusing: Some(refusal),
            ..Self::default()
        }
    }

    /// Cuántas veces se le ha pedido el secreto.
    pub(crate) fn secrets_asked(&self) -> usize {
        *crate::lock(&self.secrets_asked)
    }

    /// Cuántas veces se le ha pedido firmar, con o sin éxito.
    pub(crate) fn signing_attempts(&self) -> usize {
        *crate::lock(&self.attempts)
    }

    /// El algoritmo y los bytes de cada firma, en el orden en que se pidieron.
    pub(crate) fn signed(&self) -> Vec<(String, Vec<u8>)> {
        crate::lock(&self.signed).clone()
    }
}

impl TokenSigning for InMemoryTokenSigning {
    fn secret_of(&self, _certificate: &TokenCertificate) -> Result<StoreSecret, SigningRefusal> {
        *crate::lock(&self.secrets_asked) += 1;
        match &self.refusing {
            Some(refusal) => Err(refusal.clone()),
            None => Ok(StoreSecret::TypedOnScreen),
        }
    }

    fn sign(
        &self,
        _certificate: &TokenCertificate,
        _secret: &str,
        algorithm: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        *crate::lock(&self.attempts) += 1;
        if let Some(refusal) = &self.refusing {
            return Err(refusal.clone());
        }
        crate::lock(&self.signed).push((algorithm.to_owned(), data.to_vec()));
        Ok([b"PK1:".as_slice(), data].concat())
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

/// El origen de datos que nunca baja nada: el `dat` de la grada A viene siempre en la URL.
struct NoDownloads;

impl DataSource for NoDownloads {
    fn download(&self, _url: &str) -> Result<Vec<u8>, String> {
        panic!("ninguna prueba de la grada A baja nada de la red")
    }
}

/// La lectura de la operación sin descargas.
pub fn read_operation(
    url: &crate::site::domain::protocol::AfirmaUrl,
) -> Result<SiteOperation, Refusal> {
    crate::site::domain::protocol::read_operation(url, &NoDownloads)
}

/// El validador al que no se llega a preguntar porque la sede no pidió `checkSignatures`.
pub struct NotAsked;

impl ValidationEngine for NotAsked {
    fn verdict_of(
        &self,
        _document_b64: &str,
        _format: Format,
    ) -> Result<SignatureVerdict, BridgeError> {
        panic!("sin checkSignatures el tramite no pregunta por las firmas previas")
    }
}

/// El validador de firmas doblado: contesta lo que se le diga y apunta lo que le preguntaron.
pub struct AValidator {
    asked: Mutex<Vec<String>>,
    verdict: Result<SignatureVerdict, ()>,
}

impl AValidator {
    /// El validador que siempre contesta el mismo veredicto.
    pub fn saying(verdict: SignatureVerdict) -> Self {
        Self {
            asked: Mutex::new(Vec::new()),
            verdict: Ok(verdict),
        }
    }

    /// El validador que no llega a dar veredicto porque el puente falla.
    pub fn that_breaks() -> Self {
        Self {
            asked: Mutex::new(Vec::new()),
            verdict: Err(()),
        }
    }

    /// Los documentos en Base64 por los que se preguntó, en orden.
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("el candado").clone()
    }
}

impl ValidationEngine for AValidator {
    fn verdict_of(
        &self,
        document_b64: &str,
        _format: Format,
    ) -> Result<SignatureVerdict, BridgeError> {
        self.asked
            .lock()
            .expect("el candado")
            .push(document_b64.to_owned());
        self.verdict
            .clone()
            .map_err(|()| BridgeError::Failed("el validador no arranca".to_owned()))
    }
}
