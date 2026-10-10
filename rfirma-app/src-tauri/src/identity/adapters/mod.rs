//! Adaptadores de `identity`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

pub mod failures;
pub mod folder;
#[cfg(target_os = "linux")]
pub mod keyring;
#[cfg(any(target_os = "linux", windows))]
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
#[cfg(any(target_os = "linux", windows))]
pub use pcsc::survey_readers;
/// El vigilante de lectores de esta plataforma.
#[cfg(any(target_os = "linux", windows))]
pub use pcsc::PcscReaderWatch as DesktopReaderWatch;
/// Una consulta puntual a los lectores; sin PC/SC en esta plataforma, nunca hay respuesta.
#[cfg(not(any(target_os = "linux", windows)))]
pub fn survey_readers() -> Option<Vec<crate::identity::domain::readers::Reader>> {
    None
}
/// Si esta plataforma habla con PC/SC.
pub const SPEAKS_PCSC: bool = cfg!(any(target_os = "linux", windows));

/// El vigilante de lectores de esta plataforma.
#[cfg(not(any(target_os = "linux", windows)))]
pub use readers::UnavailableReaderWatch as DesktopReaderWatch;

/// El token de esta plataforma.
#[cfg(unix)]
pub use pkcs11::RealToken as DesktopToken;
/// El token de esta plataforma.
#[cfg(windows)]
pub use windows_store::WindowsToken as DesktopToken;

/// Los almacenes de certificados de esta plataforma.
#[cfg(unix)]
pub use pkcs11::stores::from_environment as desktop_stores;
/// Los almacenes de certificados de esta plataforma.
#[cfg(windows)]
pub use windows_store::from_environment as desktop_stores;
