//! Rellena el `DebugReport` de `--debug-info` leyendo el entorno de este proceso; no formatea nada y no deja pasar rutas.

use crate::desktop::adapters::registry::this_desktop;
use crate::desktop::application::debug_report::{
    DebugReport, LinuxEnvironment, NativeLibraryStatus, ProtocolHandlerStatus,
};
use crate::desktop::domain::channel::Channel;
use crate::signing::adapters::ffi::NativeBridge;
use crate::signing::domain::bridge::BridgeError;

const SCHEME: &str = "afirma";
const UNKNOWN: &str = "desconocido";

/// El informe de este proceso, con la librería nativa realmente cargada.
pub fn this_process_report() -> DebugReport {
    let channel = Channel::detected();
    DebugReport {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        channel: channel.label().to_owned(),
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        linux: (std::env::consts::OS == "linux").then(|| linux_environment(channel)),
        locale: locale(),
        protocol_handler: protocol_handler(channel),
        native_library: native_library(),
    }
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

fn native_library() -> NativeLibraryStatus {
    match NativeBridge::open() {
        Ok(_) => NativeLibraryStatus::Loaded,
        Err(BridgeError::NotFound(_)) => NativeLibraryStatus::NotFound,
        Err(BridgeError::MissingSymbol { .. }) => NativeLibraryStatus::IncompatibleSymbols,
        Err(_) => NativeLibraryStatus::NotLoadable,
    }
}
