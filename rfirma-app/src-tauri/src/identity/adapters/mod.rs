//! Adaptadores de `identity`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

pub mod failures;
pub mod folder;
#[cfg(target_os = "linux")]
pub mod keyring;
#[cfg(windows)]
pub mod pending_windows_credential_manager;
pub mod pkcs11;
pub mod tauri;
pub mod views;
#[cfg(windows)]
pub mod windows_store;

/// El llavero del escritorio de esta plataforma.
#[cfg(target_os = "linux")]
pub use keyring::RealKeyring as DesktopKeyring;
/// El llavero del escritorio de esta plataforma.
#[cfg(windows)]
pub use pending_windows_credential_manager::PendingWindowsCredentialManager as DesktopKeyring;

/// El token de esta plataforma.
#[cfg(target_os = "linux")]
pub use pkcs11::RealToken as DesktopToken;
/// El token de esta plataforma.
#[cfg(windows)]
pub use windows_store::WindowsToken as DesktopToken;

/// Los almacenes de certificados de esta plataforma.
#[cfg(target_os = "linux")]
pub use pkcs11::stores::from_environment as desktop_stores;
/// Los almacenes de certificados de esta plataforma.
#[cfg(windows)]
pub use windows_store::from_environment as desktop_stores;
