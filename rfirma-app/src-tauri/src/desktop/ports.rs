//! Puertos del contexto de escritorio: el registro de manejadores, la memoria de la versión publicada, su instalador y lo que alcanza la línea de órdenes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::desktop::domain::error::DesktopError;
use crate::desktop::domain::handlers::UrlHandler;
use crate::desktop::domain::installation::InstallFailure;
use crate::desktop::domain::sign_arguments::Algorithm;
use crate::desktop::domain::version_check::VersionCheck;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::SecretName;
use crate::identity::domain::store::StoreClass;
use crate::memory_error::MemoryError;
use crate::signing::domain::bridge::{BridgeError, Format, SignatureOperation};
use crate::site::domain::protocol::SiteFilter;

/// Quién atiende un esquema según el escritorio, y cómo se elige (ADR-0015).
pub trait HandlerRegistry {
    /// Los manejadores registrados para el esquema, o nada si el sandbox no deja preguntar.
    fn registered_for(&self, scheme: &str) -> Option<Vec<UrlHandler>>;

    /// El manejador que la persona tiene elegido para el esquema, si eligió alguno.
    fn current_default_for(&self, scheme: &str) -> Option<String>;

    /// Deja ese manejador como elegido para el esquema.
    fn choose_for(&self, scheme: &str, handler: &str) -> Result<(), DesktopError>;

    /// Quita la clave del esquema, sin escribir ningún otro manejador en su lugar.
    fn remove_for(&self, scheme: &str) -> Result<(), DesktopError>;
}

/// La última comprobación de versión, recordada entre sesiones y exenta de los interruptores (ADR-0010).
pub trait VersionMemory {
    /// La última comprobación guardada, si la hay.
    fn last_version_check(&self) -> Option<VersionCheck>;

    /// Guarda la comprobación recién hecha.
    fn remember_version_check(&self, check: VersionCheck) -> Result<(), MemoryError>;
}

/// Descarga, verifica e instala la versión anunciada; fuera de Windows, no está disponible (ADR-0035).
pub trait UpdateInstaller {
    /// La versión que anuncia el feed de instalación, o nada si no hay una mayor.
    fn announced(&self) -> Result<Option<String>, InstallFailure>;

    /// Descarga la versión anunciada, verifica su firma, cierra la aplicación y ejecuta el instalador.
    fn install(&self) -> Result<(), InstallFailure>;
}

/// Entrega un fichero al proceso de escritorio, por la instancia única si ya está abierto.
pub trait DesktopHandover {
    /// Deja el fichero en la ventana de rFirma, sin esperar a que termine de abrirse.
    fn hand_over(&self, file: &Path) -> Result<(), String>;
}

/// Los almacenes de certificados que ve la línea de órdenes.
pub trait CertificateStores {
    /// Los certificados firmables de todos los almacenes, o por qué ninguno se ha podido abrir.
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError>;

    /// El módulo PKCS#11 ya descubierto que es, canonizada, la biblioteca que se nombra.
    fn discovered_module(&self, library: &str) -> Option<PathBuf>;

    /// La clase del almacén del que sale ese certificado, que la referencia sola no sabe si es el de rFirma.
    fn class_of(&self, reference: &CertificateRef) -> StoreClass {
        reference.store().class()
    }
}

/// El motor de filtros de la sede, aplicado a los certificados de la línea de órdenes.
pub trait CertificateFilter {
    /// Los certificados vigentes que cumplen el filtro, o por qué el motor no ha contestado.
    fn accepted(
        &self,
        filter: &SiteFilter,
        certificates: Vec<TokenCertificate>,
    ) -> Result<Vec<TokenCertificate>, String>;
}

/// El secreto que la orden pide en la terminal, y si es porque el anterior no valía.
pub struct AskedSecret<'a> {
    pub name: SecretName,
    pub alias: &'a str,
    pub incorrect: bool,
}

/// La terminal desde la que se lanza la orden.
pub trait Terminal {
    /// Si hay una persona al otro lado que puede contestar a lo que se le pregunte.
    fn is_interactive(&self) -> bool;

    /// El secreto tecleado sin eco en la TTY, o por qué no se ha tecleado.
    fn secret(&self, asked: &AskedSecret<'_>) -> Result<ProtectedSecret, String>;

    /// La posición del certificado elegido en la lista, que empieza en `preselected`, o por qué no se ha elegido.
    fn chosen(&self, offered: &[OfferedCertificate], preselected: usize) -> Result<usize, String>;
}

/// Un certificado como lo enseña la lista de `-certtui`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferedCertificate {
    pub holder: String,
    pub issuer: String,
    pub expires: String,
    pub store: String,
}

/// Los descriptores que abre quien lanza la orden, de donde sale el PIN de `-password-fd`.
pub trait SecretDescriptor {
    /// El secreto que hay en ese descriptor, o por qué no se ha podido leer.
    fn read(&self, descriptor: u32) -> Result<ProtectedSecret, String>;
}

/// Los ficheros que la orden lee y escribe.
pub trait CommandLineFiles {
    /// Los bytes del fichero, o por qué no se han podido leer.
    fn read(&self, path: &Path) -> Result<Vec<u8>, String>;

    /// Deja los bytes en esa ruta, sobrescribiendo lo que hubiera.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;
}

/// El validador del original, como lo usa su orden `verify`.
pub trait SignatureVerifier {
    /// Un texto por resultado de validez de las firmas del documento, con el validador de ese formato.
    fn results_of(&self, document: &[u8], format: Format) -> Result<Vec<String>, BridgeError>;
}

/// Lo que la orden pide firmar y con qué.
pub struct CommandLineSigning<'a> {
    pub input: &'a Path,
    pub certificate: &'a TokenCertificate,
    pub format: Format,
    pub operation: SignatureOperation,
    pub algorithm: Algorithm,
    pub terminal: &'a dyn Terminal,
    pub parameters: &'a BTreeMap<String, String>,
    pub document_length: usize,
    pub password_fd: Option<u32>,
    pub descriptor: &'a dyn SecretDescriptor,
}

/// La firma por el camino de la sede, sin ventana ni AppHandle.
pub trait DocumentSigner {
    /// El documento firmado, o por qué no ha salido.
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String>;

    /// Apunta el certificado con el que se acaba de firmar, si la memoria lo permite.
    fn remember(&self, certificate: &TokenCertificate);

    /// El certificado recordado, si la memoria guarda alguno.
    fn remembered(&self) -> Option<CertificateRef>;
}
