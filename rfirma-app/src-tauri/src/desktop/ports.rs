//! Puertos del contexto de escritorio: el registro de manejadores, la memoria de la versión publicada, su instalador y lo que alcanza la línea de órdenes.

use std::path::{Path, PathBuf};

use crate::desktop::domain::error::DesktopError;
use crate::desktop::domain::handlers::UrlHandler;
use crate::desktop::domain::installation::InstallFailure;
use crate::desktop::domain::sign_arguments::Algorithm;
use crate::desktop::domain::version_check::VersionCheck;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::TokenError;
use crate::memory_error::MemoryError;
use crate::signing::domain::bridge::{BridgeError, Format};

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
}

/// La terminal desde la que se lanza la orden.
pub trait Terminal {
    /// Si hay una persona al otro lado que puede contestar a lo que se le pregunte.
    fn is_interactive(&self) -> bool;
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
    pub algorithm: Algorithm,
}

/// La firma por el camino de la sede, sin ventana ni AppHandle.
pub trait DocumentSigner {
    /// El documento firmado, o por qué no ha salido.
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String>;

    /// Apunta el certificado con el que se acaba de firmar, si la memoria lo permite.
    fn remember(&self, certificate: &TokenCertificate);
}
