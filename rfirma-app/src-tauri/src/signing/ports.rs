//! Puertos del contexto de firma: el puente, el hilo que lo aloja y lo que el ciclo le pide al token.

use std::path::Path;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
pub use crate::identity::domain::holder::PromptedHolder;
pub use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::{BridgeError, PostSignRequest, PreSignRequest, PreSignature};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::Language;

/// El puente nativo visto desde el ciclo: prefirma y postfirma, y ninguna entrada que firme (ADR-0001).
pub trait Bridge {
    /// Prefirma del formato que se pide: los atributos que el token firmará y el sello de sesión.
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError>;

    /// Postfirma: el documento firmado a partir de una prefirma ya sellada.
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

/// Solicitud interactiva de credenciales (PIN o contraseña de almacén).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretPromptRequest {
    /// Etiqueta o nombre descriptivo del token o almacén.
    pub token_label: String,
    /// Titular del certificado para el que se pide el secreto, si el DER lo dice.
    pub holder: Option<PromptedHolder>,
    /// Idioma preferido para los textos del diálogo.
    pub language: Language,
    /// Indica si se trata de un reintento tras un PIN erróneo.
    pub incorrect_pin: bool,
    /// Intentos restantes si el token o módulo los comunica.
    pub attempts_left: Option<u32>,
}

/// Fallo o interrupción en la solicitud interactiva de credenciales.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretPromptError {
    /// La persona usuaria canceló el diálogo o pulsó Escape.
    Cancelled,
    /// Fallo al desplegar la interfaz gráfica o error del prompter.
    Failed(String),
}

impl std::fmt::Display for SecretPromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("solicitud de PIN cancelada por la persona usuaria"),
            Self::Failed(reason) => write!(f, "fallo en el diálogo de PIN: {reason}"),
        }
    }
}

impl std::error::Error for SecretPromptError {}

/// Puerto de diálogo interactivo para la solicitud de credenciales seguras.
pub trait SecretPrompter: Send + Sync {
    /// Presenta el diálogo interactivo para solicitar el secreto al usuario.
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError>;
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

    /// Firma `data` con la clave privada que acompaña al certificado, con el algoritmo pedido.
    fn sign(
        &self,
        reference: &CertificateRef,
        pin: &str,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError>;

    /// Firma `data` con la clave privada que acompaña al certificado y el secreto protegido.
    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        let pin_str = secret.as_str().map_err(|_| {
            TokenError::new(
                crate::identity::domain::error::Situation::IncorrectPin,
                "el secreto no es UTF-8 valido",
            )
        })?;
        self.sign(reference, pin_str, algorithm, data)
    }
}

/// El documento que se va a firmar, leído de donde esté.
pub trait DocumentBytes {
    /// El contenido del documento en la ruta indicada.
    fn read(&self, path: &Path) -> Result<Vec<u8>, String>;
}
