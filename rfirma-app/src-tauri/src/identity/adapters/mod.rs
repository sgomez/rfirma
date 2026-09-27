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

/// El llavero del escritorio de esta plataforma.
#[cfg(target_os = "linux")]
pub use keyring::RealKeyring as DesktopKeyring;
/// El llavero del escritorio de esta plataforma.
#[cfg(windows)]
pub use pending_windows_credential_manager::PendingWindowsCredentialManager as DesktopKeyring;
