//! Pregunta al sistema cómo se instaló este ejecutable: paquete de dpkg o de rpm, flatpak, instalador de Windows, macOS o compilación de desarrollo.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::desktop::application::debug_report::{
    FlatpakInstallation, Installation, InstallerScope, WebView,
};
use crate::desktop::domain::channel::Channel;

const SANDBOX_MARKER: &str = "/.flatpak-info";
const UNKNOWN: &str = "desconocido";

/// Cómo se instaló el ejecutable de este proceso.
pub fn this_installation(channel: Channel) -> Installation {
    match channel {
        Channel::Flatpak => Installation::Flatpak(flatpak_installation()),
        Channel::Windows => Installation::WindowsInstaller(installer_scope()),
        Channel::Native if cfg!(target_os = "macos") => Installation::MacOs,
        Channel::Native => native_installation(),
    }
}

/// La versión de glibc, solo donde rFirma corre sobre la del sistema.
pub fn this_glibc(channel: Channel) -> Option<String> {
    if channel != Channel::Native {
        return None;
    }
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // SAFETY: devuelve un puntero a una cadena estática de glibc, nunca nulo.
        let version = unsafe { std::ffi::CStr::from_ptr(libc::gnu_get_libc_version()) };
        Some(version.to_string_lossy().into_owned())
    }
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    None
}

/// El motor web propio del canal: WebKitGTK en Linux nativo (en flatpak lo da el runtime) y WebView2 en Windows.
pub fn this_webview(channel: Channel) -> Option<WebView> {
    let name = match channel {
        Channel::Native if cfg!(target_os = "linux") => "WebKitGTK",
        Channel::Windows => "WebView2",
        _ => return None,
    };
    let version = tauri::webview_version().ok()?;
    Some(WebView {
        name: name.to_owned(),
        version,
    })
}

fn native_installation() -> Installation {
    let Ok(executable) = std::env::current_exe() else {
        return development_build();
    };
    if owned_by(Command::new("dpkg").arg("-S"), &executable) {
        Installation::Deb
    } else if owned_by(Command::new("rpm").arg("-qf"), &executable) {
        Installation::Rpm
    } else {
        development_build()
    }
}

fn owned_by(query: &mut Command, executable: &Path) -> bool {
    query
        .arg(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn development_build() -> Installation {
    Installation::Development {
        commit: option_env!("RFIRMA_GIT_COMMIT")
            .unwrap_or(UNKNOWN)
            .to_owned(),
    }
}

fn flatpak_installation() -> FlatpakInstallation {
    let info = std::fs::read_to_string(SANDBOX_MARKER).unwrap_or_default();
    FlatpakInstallation {
        remote: flatpak_value(&info, "Instance", "origin"),
        branch: flatpak_value(&info, "Instance", "branch"),
        runtime: flatpak_value(&info, "Application", "runtime")
            .strip_prefix("runtime/")
            .map(str::to_owned)
            .unwrap_or_else(|| UNKNOWN.to_owned()),
    }
}

fn flatpak_value(info: &str, section: &str, key: &str) -> String {
    let header = format!("[{section}]");
    info.lines()
        .skip_while(|line| line.trim() != header)
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| UNKNOWN.to_owned())
}

fn installer_scope() -> InstallerScope {
    let in_program_files = std::env::current_exe().is_ok_and(|executable| {
        ["ProgramFiles", "ProgramW6432"]
            .iter()
            .filter_map(std::env::var_os)
            .any(|root| executable.starts_with(root))
    });
    if in_program_files {
        InstallerScope::PerMachine
    } else {
        InstallerScope::PerUser
    }
}
