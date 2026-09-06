//! Puertos del contexto de firma: el puente, el hilo que lo aloja y lo que el ciclo le pide al token.

use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::{BridgeError, PostSignRequest, PreSignRequest, PreSignature};
use crate::signing::domain::isolate_gone::IsolateGone;

/// El puente nativo visto desde el ciclo: prefirma y postfirma, y ninguna entrada que firme (ADR-0001).
pub trait Bridge {
    /// Prefirma PAdES: los atributos que el token firmará y el sello de sesión.
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError>;

    /// Postfirma PAdES: el PDF firmado a partir de una prefirma ya sellada.
    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError>;
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

    /// Firma `data` con la clave privada que acompaña al certificado.
    fn sign(
        &self,
        reference: &CertificateRef,
        pin: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError>;
}
