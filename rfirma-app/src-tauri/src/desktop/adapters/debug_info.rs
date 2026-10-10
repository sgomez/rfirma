//! Rellena el `DebugReport` de `--debug-info` y su `ReportOwner` leyendo el entorno de este proceso, con las rutas crudas; no formatea ni anonimiza.

use std::path::{Path, PathBuf};
use std::time::Duration;

use openssl::asn1::Asn1Time;
use x509_cert::der::DateTime;

use crate::desktop::adapters::graphics_info::this_session_graphics;
use crate::desktop::adapters::installation::{this_glibc, this_installation, this_webview};
use crate::desktop::adapters::paths::Paths;
use crate::desktop::adapters::registry::this_desktop;
use crate::desktop::application::debug_report::{
    DebugReport, DiscardReason, LinuxEnvironment, ModuleStatus, NativeLibrary, NativeLibraryStatus,
    NssProfile, NssProfileState, NssStores, PcscStatus, Pkcs11Module, Pkcs11Modules,
    ProtocolHandlerStatus, ReportOwner, WindowsStores,
};
use crate::desktop::domain::channel::Channel;
use crate::identity::adapters as identity_adapters;
use crate::identity::adapters::pkcs11::p11kit;
use crate::identity::adapters::pkcs11::probe::{probe, ModuleProbe};
use crate::identity::adapters::pkcs11::stores::{
    has_certificate_database, ignored_nss_profiles, modules_from_environment, nss_profiles,
};
use crate::identity::domain::store::{Store, StoreClass};
use crate::signing::adapters::ffi::{locate, NativeBridge};
use crate::signing::domain::bridge::{BridgeError, LIBRARY_DIRECTORY_VARIABLE};
use crate::site::adapters::local_ca_trusted_in_each;
use crate::site::adapters::tls::LocalCaStore;
use crate::site::ports::LocalCaSlots;
use crate::PKCS11_MODULE_VARIABLE;

const SCHEME: &str = "afirma";
const BUNDLED_PCSC_LITE_VERSION: &str = "/app/share/rfirma/pcsc-lite-version";
const UNKNOWN: &str = "desconocido";
const MODULE_PROBE_LIMIT: Duration = Duration::from_secs(5);

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
        pcsc: pcsc_status(),
        bundled_pcsc_lite: bundled_pcsc_lite(channel),
        pkcs11_modules: pkcs11_modules(),
        nss_stores: nss_stores(),
        windows_stores: windows_stores(channel),
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

fn pcsc_status() -> Option<PcscStatus> {
    identity_adapters::SPEAKS_PCSC.then(|| match identity_adapters::survey_readers() {
        Some(readers) => PcscStatus::Responding(readers),
        None => PcscStatus::NotResponding,
    })
}

fn pkcs11_modules() -> Option<Pkcs11Modules> {
    if let Some(library) = defined_path(PKCS11_MODULE_VARIABLE) {
        return Some(Pkcs11Modules::Overridden(Pkcs11Module {
            name: library
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status: module_status(Some(&library), None),
            library: Some(library),
            registration: None,
        }));
    }
    (std::env::consts::OS == "linux").then(|| {
        Pkcs11Modules::Discovered(
            modules_from_environment()
                .into_iter()
                .map(|module| Pkcs11Module {
                    status: module_status(module.library.as_deref(), module.discard),
                    name: module.name,
                    library: module.library,
                    registration: module.registration,
                })
                .collect(),
        )
    })
}

fn module_status(library: Option<&Path>, discard: Option<p11kit::DiscardReason>) -> ModuleStatus {
    match (discard, library) {
        (Some(reason), _) => ModuleStatus::Discarded(discard_reason(reason)),
        (None, None) => ModuleStatus::DoesNotLoad,
        (None, Some(library)) => match probe(library, MODULE_PROBE_LIMIT) {
            ModuleProbe::Loads(info) => ModuleStatus::Loads {
                manufacturer: info.manufacturer,
                version: info.version,
            },
            ModuleProbe::DoesNotLoad => ModuleStatus::DoesNotLoad,
            ModuleProbe::NotResponding => ModuleStatus::NotResponding(MODULE_PROBE_LIMIT),
        },
    }
}

fn discard_reason(reason: p11kit::DiscardReason) -> DiscardReason {
    match reason {
        p11kit::DiscardReason::DisabledInRfirma => DiscardReason::DisabledInRfirma,
        p11kit::DiscardReason::EnabledOnlyElsewhere => DiscardReason::EnabledOnlyElsewhere,
        p11kit::DiscardReason::TrustPolicy => DiscardReason::TrustPolicy,
        p11kit::DiscardReason::MissingModule => DiscardReason::MissingModule,
    }
}

fn bundled_pcsc_lite(channel: Channel) -> Option<String> {
    if channel != Channel::Flatpak {
        return None;
    }
    std::fs::read_to_string(BUNDLED_PCSC_LITE_VERSION)
        .ok()
        .map(|version| version.trim().to_owned())
        .filter(|version| !version.is_empty())
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

fn windows_stores(channel: Channel) -> Option<WindowsStores> {
    if channel != Channel::Windows {
        return None;
    }
    let local_ca = Paths::from_environment()
        .ok()
        .map(|paths| LocalCaStore::of(&paths));
    let profiles = crate::site::adapters::trust_profiles();
    let trusted = local_ca.as_ref().is_some_and(|store| {
        local_ca_trusted_in_each(store, &profiles)
            .first()
            .copied()
            .unwrap_or(false)
    });
    let local_channel = if trusted {
        NssProfileState::TrustsLocalChannel {
            until: local_ca.as_ref().and_then(local_ca_expiry),
        }
    } else {
        NssProfileState::DoesNotTrustLocalChannel
    };
    Some(WindowsStores {
        local_channel,
        minidrivers: crate::desktop::adapters::minidrivers::registered_minidrivers(),
    })
}

fn nss_stores() -> Option<NssStores> {
    if std::env::consts::OS != "linux" {
        return None;
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let paths = Paths::from_environment().ok();
    let local_ca = paths.as_ref().map(LocalCaStore::of);
    let found = nss_profiles(&home);
    let trust = local_ca.as_ref().map_or_else(
        || vec![false; found.len()],
        |store| local_ca_trusted_in_each(store, &found),
    );
    let until = local_ca.as_ref().and_then(local_ca_expiry);
    let trusting = found.into_iter().zip(trust).map(|(directory, trusted)| {
        let state = if trusted {
            NssProfileState::TrustsLocalChannel {
                until: until.clone(),
            }
        } else {
            NssProfileState::DoesNotTrustLocalChannel
        };
        nss_profile(directory, state)
    });
    let ignored = ignored_nss_profiles(&home).into_iter().map(|directory| {
        nss_profile(
            directory,
            NssProfileState::IgnoredWithoutCertificateDatabase,
        )
    });
    Some(NssStores {
        profiles: trusting.chain(ignored).collect(),
        rfirma_store_installed: paths
            .is_some_and(|paths| has_certificate_database(&paths.installed_certificates_dir())),
    })
}

fn local_ca_expiry(store: &LocalCaStore) -> Option<String> {
    let serving = store.serving().ok()??;
    let epoch = Asn1Time::from_unix(0).ok()?;
    let elapsed = epoch.diff(serving.certificate().not_after()).ok()?;
    let seconds = u64::try_from(i64::from(elapsed.days) * 86_400 + i64::from(elapsed.secs)).ok()?;
    let date = DateTime::from_unix_duration(Duration::from_secs(seconds)).ok()?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month(),
        date.day()
    ))
}

fn nss_profile(directory: PathBuf, state: NssProfileState) -> NssProfile {
    NssProfile {
        browser: browser_of(&directory),
        directory,
        state,
    }
}

fn browser_of(directory: &Path) -> String {
    let name = match Store::nss(PathBuf::new(), directory).class() {
        StoreClass::Firefox => "Firefox",
        StoreClass::Chrome => "Chrome",
        StoreClass::Nssdb | StoreClass::Card | StoreClass::Installed | StoreClass::Windows => {
            "NSS del sistema"
        }
    };
    let packaging = if directory
        .components()
        .any(|part| part.as_os_str() == "snap")
    {
        " snap"
    } else if directory
        .components()
        .any(|part| part.as_os_str() == "org.mozilla.firefox")
    {
        " flatpak"
    } else {
        ""
    };
    format!("{name}{packaging}")
}

#[cfg(test)]
mod tests;
