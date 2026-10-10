//! Rellena el `DebugReport` de `--debug-info` y su `ReportOwner` leyendo el entorno de este proceso, con las rutas crudas; no formatea ni anonimiza.

use std::path::PathBuf;

use crate::desktop::adapters::graphics_info::this_session_graphics;
use crate::desktop::adapters::installation::{this_glibc, this_installation, this_webview};
use crate::desktop::adapters::registry::this_desktop;
use crate::desktop::application::debug_report::{
    DebugReport, LinuxEnvironment, NativeLibrary, NativeLibraryStatus, ProtocolHandlerStatus,
    ReportOwner,
};
use crate::desktop::domain::channel::Channel;
use crate::signing::adapters::ffi::{locate, NativeBridge};
use crate::signing::domain::bridge::{BridgeError, LIBRARY_DIRECTORY_VARIABLE};
use crate::PKCS11_MODULE_VARIABLE;

const SCHEME: &str = "afirma";
const UNKNOWN: &str = "desconocido";

/// El informe de este proceso, con la librería nativa realmente cargada.
pub fn this_process_report() -> DebugReport {
    let channel = Channel::detected();
    DebugReport {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        installation: this_installation(channel),
        glibc: this_glibc(channel),
        webview: this_webview(channel),
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        linux: (std::env::consts::OS == "linux").then(|| linux_environment(channel)),
        graphics: this_session_graphics(channel),
        locale: locale(),
        protocol_handler: protocol_handler(channel),
        native_library: native_library(),
        library_directory_override: defined_path(LIBRARY_DIRECTORY_VARIABLE),
        pkcs11_module_override: defined_path(PKCS11_MODULE_VARIABLE),
    }
}

/// El home y el nombre de quien ejecuta este proceso.
pub fn this_process_owner() -> ReportOwner {
    ReportOwner {
        home: first_defined(&["HOME", "USERPROFILE"])
            .map(PathBuf::from)
            .unwrap_or_default(),
        user_name: first_defined(&["USER", "USERNAME", "LOGNAME"]).unwrap_or_default(),
    }
}

fn first_defined(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
}

fn defined_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn linux_environment(channel: Channel) -> LinuxEnvironment {
    LinuxEnvironment {
        distribution: distribution(channel),
        desktop: environment_value("XDG_CURRENT_DESKTOP"),
        session: session_type(),
    }
}

fn environment_value(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| UNKNOWN.to_owned())
}

fn session_type() -> String {
    match std::env::var("XDG_SESSION_TYPE") {
        Ok(kind) if !kind.is_empty() => kind,
        _ if std::env::var_os("WAYLAND_DISPLAY").is_some() => "wayland".to_owned(),
        _ if std::env::var_os("DISPLAY").is_some() => "x11".to_owned(),
        _ => UNKNOWN.to_owned(),
    }
}

fn distribution(channel: Channel) -> String {
    let host_first = [
        "/run/host/os-release",
        "/etc/os-release",
        "/usr/lib/os-release",
    ];
    let candidates = if channel == Channel::Flatpak {
        &host_first[..]
    } else {
        &host_first[1..]
    };
    candidates
        .iter()
        .find_map(|file| std::fs::read_to_string(file).ok())
        .and_then(|content| pretty_name_in(&content))
        .unwrap_or_else(|| UNKNOWN.to_owned())
}

fn pretty_name_in(os_release: &str) -> Option<String> {
    os_release
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|value| value.trim().trim_matches('"').to_owned())
        .filter(|value| !value.is_empty())
}

fn locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| UNKNOWN.to_owned())
}

fn protocol_handler(channel: Channel) -> ProtocolHandlerStatus {
    if channel == Channel::Flatpak {
        return ProtocolHandlerStatus::NotQueryableFromTheSandbox;
    }
    let desktop = this_desktop();
    let handler = desktop.current_default_for(SCHEME).or_else(|| {
        if channel == Channel::Windows {
            return None;
        }
        let registered = desktop.registered_for(SCHEME)?;
        (!registered.is_empty()).then(|| {
            registered
                .iter()
                .map(|handler| without_directories(&handler.id))
                .collect::<Vec<_>>()
                .join(", ")
        })
    });
    match handler {
        Some(id) => ProtocolHandlerStatus::Registered(without_directories(&id)),
        None => ProtocolHandlerStatus::NoneRegistered,
    }
}

fn without_directories(identifier: &str) -> String {
    identifier
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(identifier)
        .to_owned()
}

fn native_library() -> NativeLibrary {
    let located = std::env::current_exe().ok().and_then(|executable| {
        let directory = executable.parent()?.to_path_buf();
        locate(&|name| std::env::var_os(name), &directory).ok()
    });
    let Some(path) = located else {
        return NativeLibrary {
            status: NativeLibraryStatus::NotFound,
            path: None,
        };
    };
    let status = match NativeBridge::open_at(&path) {
        Ok(_) => NativeLibraryStatus::Loaded,
        Err(BridgeError::MissingSymbol { .. }) => NativeLibraryStatus::IncompatibleSymbols,
        Err(_) => NativeLibraryStatus::NotLoadable,
    };
    NativeLibrary {
        status,
        path: Some(path),
    }
}
