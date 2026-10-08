//! Adaptadores de `identity`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

pub mod failures;
pub mod folder;
#[cfg(target_os = "linux")]
pub mod keyring;
#[cfg(target_os = "linux")]
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

/// El vigilante de lectores de esta plataforma.
#[cfg(target_os = "linux")]
pub use pcsc::PcscReaderWatch as DesktopReaderWatch;
/// El vigilante de lectores de esta plataforma.
#[cfg(not(target_os = "linux"))]
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
