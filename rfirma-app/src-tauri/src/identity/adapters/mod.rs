//! Adaptadores de `identity`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod dnie;
pub mod failures;
pub mod folder;
#[cfg(target_os = "linux")]
pub mod keyring;
#[cfg(target_os = "macos")]
pub mod macos_store;
#[cfg(any(target_os = "linux", windows, target_os = "macos"))]
pub mod pcsc;
#[cfg(target_os = "macos")]
pub mod pending_macos_keychain;
pub mod pkcs11;
pub mod readers;
pub mod tauri;
pub mod views;
#[cfg(windows)]
pub mod windows_credential_manager;
#[cfg(windows)]
pub mod windows_store;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod with_the_dnie;

/// El llavero del escritorio de esta plataforma.
#[cfg(target_os = "linux")]
pub use keyring::RealKeyring as DesktopKeyring;
/// El llavero del escritorio de esta plataforma.
#[cfg(target_os = "macos")]
pub use pending_macos_keychain::PendingMacosKeychain as DesktopKeyring;
/// El llavero del escritorio de esta plataforma.
#[cfg(windows)]
pub use windows_credential_manager::WindowsCredentialManager as DesktopKeyring;

/// Una consulta puntual a los lectores; `None`, si PC/SC no responde.
#[cfg(any(target_os = "linux", windows, target_os = "macos"))]
pub use pcsc::survey_readers;
/// El vigilante de lectores de esta plataforma.
#[cfg(any(target_os = "linux", windows, target_os = "macos"))]
pub use pcsc::PcscReaderWatch as DesktopReaderWatch;
/// Una consulta puntual a los lectores; sin PC/SC en esta plataforma, nunca hay respuesta.
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
pub fn survey_readers() -> Option<Vec<crate::identity::domain::readers::Reader>> {
    None
}
/// Si esta plataforma habla con PC/SC.
pub const SPEAKS_PCSC: bool = cfg!(any(target_os = "linux", windows, target_os = "macos"));

/// El vigilante de lectores de esta plataforma.
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
pub use readers::UnavailableReaderWatch as DesktopReaderWatch;

/// El token de esta plataforma.
#[cfg(target_os = "macos")]
pub type DesktopToken = with_the_dnie::WithTheDnie<macos_store::MacosToken>;
/// El token de esta plataforma.
#[cfg(target_os = "linux")]
pub use pkcs11::RealToken as DesktopToken;
/// El token de esta plataforma.
#[cfg(windows)]
pub use windows_store::WindowsToken as DesktopToken;

/// El token de esta plataforma, con el lector del DNIe en macOS.
#[cfg(target_os = "macos")]
pub fn desktop_token() -> DesktopToken {
    DesktopToken::default()
}
/// El token de esta plataforma.
#[cfg(any(target_os = "linux", windows))]
pub fn desktop_token() -> DesktopToken {
    DesktopToken
}

/// Los almacenes de certificados de esta plataforma.
#[cfg(target_os = "macos")]
pub fn desktop_stores() -> Vec<crate::identity::domain::store::Store> {
    with_the_dnie::and_the_dnie_store(macos_store::from_environment())
}
/// Los almacenes de certificados de esta plataforma.
#[cfg(target_os = "linux")]
pub use pkcs11::stores::from_environment as desktop_stores;
/// Los almacenes de certificados de esta plataforma.
#[cfg(windows)]
pub use windows_store::from_environment as desktop_stores;

/// El DNIe que ve el lector de esta plataforma; solo Linux y macOS lo leen con rdnie.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn this_dnie() -> crate::identity::domain::seen_dnie::DnieInTheReader {
    dnie::seen_dnie(&dnie::PcscCard)
}
/// El DNIe que ve el lector de esta plataforma; solo Linux y macOS lo leen con rdnie.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn this_dnie() -> crate::identity::domain::seen_dnie::DnieInTheReader {
    crate::identity::domain::seen_dnie::DnieInTheReader::Absent
}

/// La versión de rdnie compilada; sin ella en Windows.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub const RDNIE_VERSION: &str = dnie::RDNIE_VERSION;
/// La versión de rdnie compilada; sin ella en Windows.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub const RDNIE_VERSION: &str = "no incluido";
