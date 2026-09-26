//! Puertos del contexto de firma: el puente, el hilo que lo aloja y lo que el ciclo le pide al token.

use std::path::Path;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
pub use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::{BridgeError, PostSignRequest, PreSignRequest, PreSignature};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::previous_signatures::PreviousSignaturesReport;

/// El puente nativo visto desde el ciclo: prefirma y postfirma, y ninguna entrada que firme (ADR-0001).
pub trait Bridge {
    /// Prefirma del formato que se pide: los atributos que el token firmará y el sello de sesión.
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError>;

    /// Postfirma: el documento firmado a partir de una prefirma ya sellada.
    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError>;
}

/// Las firmas que ya trae un PDF, leídas con el recorrido de firmantes del original.
pub trait PreviousSignaturesEngine {
    /// El informe de firmas previas del PDF de entrada, en Base64.
    fn previous_signatures(
        &self,
        document_b64: &str,
    ) -> Result<PreviousSignaturesReport, BridgeError>;
}

/// El hilo dueño del puente: corre una tarea con el puente delante y devuelve lo que salió (ADR-0003).
pub trait IsolateHost {
    /// Corre la tarea en el hilo del puente; `Err` fuera si el hilo murió, `Err` dentro si el puente no abre.
    fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce(&dyn Bridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone>;
}

/// Lo que el ciclo le pide al token: cómo pide el secreto y la firma de unos bytes; nunca la clave (ADR-0001).
pub trait Signer {
    /// Cómo hay que pedirle el secreto al almacén del certificado.
    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError>;

    /// Comprueba que el token ofrece el mecanismo del algoritmo, antes de pedir el secreto.
    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError>;

    /// Si el almacén del certificado acepta el secreto, sin firmar nada.
    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), TokenError>;

    /// Firma `data` con la clave privada que acompaña al certificado y el secreto protegido (ADR-0001).
    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError>;
}

/// El documento que se va a firmar, leído de donde esté.
pub trait DocumentBytes {
    /// El contenido del documento en la ruta indicada.
    fn read(&self, path: &Path) -> Result<Vec<u8>, String>;
}
