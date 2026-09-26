//! Puertos del contexto de identidad: el token, el almacén de los `.p12` instalados, el certificado
//! recordado, el diálogo interactivo que pide un secreto y el llavero del PIN del Almacén de rFirma
//! (ADR-0001, ADR-0014, ADR-0034).

use std::fmt;
use std::path::Path;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::holder::PromptedHolder;
use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{SecretName, StoreSecret};
use crate::identity::domain::store::Store;
use crate::memory_error::MemoryError;
use crate::signing::domain::Language;

/// El token visto desde los casos de uso: lista, dice cómo pide el secreto, firma e importa un `.p12` (ADR-0001).
pub trait Token {
    /// Los certificados firmables de un almacén.
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError>;

    /// Todos los certificados de un almacén, también los que no firman: con ellos se completa una cadena.
    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError>;

    /// Cómo hay que pedirle el secreto al almacén del certificado.
    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError>;

    /// Comprueba que la ranura del certificado ofrece el mecanismo del algoritmo, sin pedir el secreto.
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
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError>;

    /// Importa un `.p12` a un almacén NSS nuevo en ese directorio y devuelve el almacén.
    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
    ) -> Result<Store, TokenError>;

    /// Los certificados de todos los almacenes: falla solo si ninguno se ha podido abrir.
    fn list_across(&self, stores: &[Store]) -> Result<Vec<TokenCertificate>, TokenError> {
        if stores.is_empty() {
            return Err(TokenError::new(
                Situation::ModuleNotFound,
                "no hay ningun modulo PKCS#11 donde buscar certificados",
            ));
        }

        let mut found = Vec::new();
        let mut any_loaded = false;
        let mut refused: Option<TokenError> = None;

        for store in stores {
            match self.list(store) {
                Ok(certificates) => {
                    any_loaded = true;
                    found.extend(certificates);
                }
                Err(error) => refused = refused.or(Some(error)),
            }
        }

        match refused {
            Some(error) if !any_loaded => Err(error),
            _ => Ok(found),
        }
    }
}

/// La carpeta donde vive cada `.p12` instalado, con sus permisos (ADR-0011).
pub trait InstalledFolder {
    /// Crea la carpeta del almacén recién instalado.
    fn make(&self, directory: &Path) -> Result<(), String>;

    /// Deja la ruta legible solo por su dueño.
    fn restrict_to_owner(&self, path: &Path);

    /// Borra la carpeta del almacén y todo lo que hubiera dentro.
    fn remove(&self, directory: &Path) -> Result<(), String>;
}

/// El certificado con el que se firmó la última vez, recordado entre sesiones (ADR-0010).
pub trait CertificateMemory {
    /// El certificado recordado, si lo hay.
    fn remembered_certificate(&self) -> Option<CertificateRef>;

    /// Apunta el certificado con el que se acaba de firmar, según permitan los interruptores.
    fn remember_the_certificate(&self, reference: &CertificateRef) -> Result<(), MemoryError>;

    /// Olvida el certificado recordado.
    fn forget_the_certificate(&self) -> Result<(), MemoryError>;
}

/// El PIN del Almacén de rFirma en el llavero del escritorio (ADR-0034).
pub trait Keyring {
    /// El PIN si el llavero ya lo tiene, sin crear nada.
    fn pin(&self) -> Result<ProtectedSecret, KeyringError>;

    /// Genera un PIN nuevo y lo guarda en el llavero.
    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError>;

    /// El PIN del almacén: lo crea si el llavero todavía no lo tiene.
    fn get_or_create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        match self.pin() {
            Err(KeyringError::PinMissing) => self.create_pin(),
            other => other,
        }
    }
}

/// Solicitud interactiva de credenciales (PIN o contraseña de almacén).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretPromptRequest {
    /// Cómo se llama el secreto que se pide.
    pub secret: SecretName,
    /// Titular del certificado para el que se pide el secreto, si el DER lo dice.
    pub holder: Option<PromptedHolder>,
    /// Idioma preferido para los textos del diálogo.
    pub language: Language,
    /// Indica si se trata de un reintento tras un secreto erróneo.
    pub incorrect_secret: bool,
}

/// Fallo o interrupción en la solicitud interactiva de credenciales.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretPromptError {
    /// La persona usuaria canceló el diálogo o pulsó Escape.
    Cancelled,
    /// Fallo al desplegar la interfaz gráfica o error del prompter.
    Failed(String),
}

impl fmt::Display for SecretPromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "solicitud de secreto cancelada por la persona usuaria"),
            Self::Failed(reason) => write!(f, "no se pudo pedir el secreto: {reason}"),
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

/// Fallo al pedir el secreto hasta que se acepta: el diálogo mismo, o un intento sin remedio.
#[derive(Debug)]
pub enum PromptedError<E> {
    /// La solicitud interactiva del secreto fue cancelada o falló.
    Prompt(SecretPromptError),
    /// El intento rechazó el secreto y `rejected` dijo que no merecía la pena reintentarlo.
    Attempt(E),
}

/// Pide el secreto hasta que `attempt` lo acepta; `rejected` decide si el rechazo merece reintentarlo (ADR-0001, ADR-0014).
pub fn prompted_until_accepted<T, E>(
    prompter: &dyn SecretPrompter,
    mut request: SecretPromptRequest,
    mut attempt: impl FnMut(&ProtectedSecret) -> Result<T, E>,
    rejected: impl Fn(&E) -> bool,
) -> Result<(ProtectedSecret, T), PromptedError<E>> {
    loop {
        let secret = prompter
            .prompt_secret(&request)
            .map_err(PromptedError::Prompt)?;
        match attempt(&secret) {
            Ok(done) => return Ok((secret, done)),
            Err(error) if rejected(&error) => {
                request.incorrect_secret = true;
            }
            Err(other) => return Err(PromptedError::Attempt(other)),
        }
    }
}

#[cfg(test)]
mod tests;
