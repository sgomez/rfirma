//! Los gráficos de esta sesión para `--debug-info`: DRM, NVIDIA y el entorno en Linux; solo las tarjetas en Windows y macOS.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::desktop::adapters::webkit_renderer::the_variable_in_force;
use crate::desktop::application::debug_report::{GlExtension, Gpu, Graphics};
use crate::desktop::domain::channel::Channel;

#[cfg(windows)]
mod windows_adapters;
#[cfg(windows)]
use windows_adapters::display_adapters;

const DRM_DIRECTORY: &str = "/sys/class/drm";
const NVIDIA_MODULE_VERSION: &str = "/sys/module/nvidia/version";
const NVIDIA_DRIVER: &str = "nvidia";

/// Un adaptador de pantalla tal como lo da de alta el registro de Windows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayAdapter {
    /// `DriverDesc`.
    pub description: String,
    /// `DriverVersion`.
    pub driver_version: Option<String>,
    /// `MatchingDeviceId`, con el `ven_` del fabricante si es PCI.
    pub hardware_id: Option<String>,
    /// `ProviderName`, el fabricante del controlador.
    pub provider: Option<String>,
}

/// Los gráficos de esta sesión; en Windows y macOS, solo las tarjetas.
pub fn this_session_graphics(channel: Channel) -> Graphics {
    match std::env::consts::OS {
        "linux" => linux_graphics(channel),
        "windows" => Graphics {
            gpus: display_adapters().into_iter().map(gpu_of_adapter).collect(),
            ..Graphics::default()
        },
        "macos" => Graphics {
            gpus: system_profiler_output()
                .map(|json| gpus_from_system_profiler(&json))
                .unwrap_or_default(),
            ..Graphics::default()
        },
        _ => Graphics::default(),
    }
}

#[cfg(not(windows))]
fn display_adapters() -> Vec<DisplayAdapter> {
    Vec::new()
}

/// La tarjeta de un adaptador del registro: fabricante por su `ven_`, o por quien firma el controlador.
pub fn gpu_of_adapter(adapter: DisplayAdapter) -> Gpu {
    let vendor = adapter
        .hardware_id
        .as_deref()
        .and_then(pci_vendor_id)
        .map(|id| vendor_name(&id))
        .or(adapter.provider)
        .unwrap_or_default();
    Gpu {
        vendor,
        driver: adapter.description,
        driver_version: adapter.driver_version,
    }
}

fn pci_vendor_id(hardware_id: &str) -> Option<String> {
    let lowercase = hardware_id.to_ascii_lowercase();
    let start = lowercase.find("ven_")? + "ven_".len();
    let id = lowercase.get(start..start + 4)?;
    id.bytes()
        .all(|byte| byte.is_ascii_hexdigit())
        .then(|| format!("0x{id}"))
}

fn system_profiler_output() -> Option<String> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Las tarjetas que lista `system_profiler SPDisplaysDataType -json`, sin versión de controlador.
pub fn gpus_from_system_profiler(json: &str) -> Vec<Gpu> {
    let Ok(document) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    document["SPDisplaysDataType"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|display| {
            let model = text_of(display, "sppci_model").or_else(|| text_of(display, "_name"))?;
            let vendor = text_of(display, "sppci_vendor")
                .or_else(|| text_of(display, "spdisplays_vendor"))
                .map(|vendor| macos_vendor_name(&vendor))
                .unwrap_or_default();
            Some(Gpu {
                vendor,
                driver: model,
                driver_version: None,
            })
        })
        .collect()
}

fn text_of(display: &Value, field: &str) -> Option<String> {
    display[field]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn macos_vendor_name(raw: &str) -> String {
    let name = raw.strip_prefix("sppci_vendor_").unwrap_or(raw);
    if let Some(id) = name
        .split(['(', ')'])
        .map(str::trim)
        .find(|part| part.starts_with("0x"))
    {
        return vendor_name(&id.to_ascii_lowercase());
    }
    match name.to_ascii_lowercase().as_str() {
        "apple" => "Apple".to_owned(),
        "intel" => "Intel".to_owned(),
        "amd" | "ati" => "AMD".to_owned(),
        "nvidia" => "NVIDIA".to_owned(),
        _ => name.to_owned(),
    }
}

fn linux_graphics(channel: Channel) -> Graphics {
    let gpus = gpus_in(Path::new(DRM_DIRECTORY));
    let gl_extension = (channel == Channel::Flatpak)
        .then(|| gl_extension_for(&gpus))
        .flatten();
    Graphics {
        gpus,
        renderer: the_variable_in_force(|name| std::env::var(name).ok()),
        display_backend: std::env::var("GDK_BACKEND")
            .ok()
            .filter(|value| !value.is_empty()),
        gl_extension,
    }
}

fn gpus_in(drm: &Path) -> Vec<Gpu> {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut cards: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| is_a_card(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.path().join("device"))
        .collect();
    cards.sort();
    cards.iter().filter_map(|device| gpu_at(device)).collect()
}

fn is_a_card(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn gpu_at(device: &Path) -> Option<Gpu> {
    let vendor_id = std::fs::read_to_string(device.join("vendor")).ok()?;
    let driver = std::fs::read_link(device.join("driver"))
        .ok()?
        .file_name()?
        .to_string_lossy()
        .into_owned();
    let driver_version = (driver == NVIDIA_DRIVER)
        .then(nvidia_driver_version)
        .flatten();
    Some(Gpu {
        vendor: vendor_name(vendor_id.trim()),
        driver,
        driver_version,
    })
}

fn vendor_name(id: &str) -> String {
    match id {
        "0x10de" => "NVIDIA".to_owned(),
        "0x8086" => "Intel".to_owned(),
        "0x1002" => "AMD".to_owned(),
        other => other.to_owned(),
    }
}

fn nvidia_driver_version() -> Option<String> {
    std::fs::read_to_string(NVIDIA_MODULE_VERSION)
        .ok()
        .map(|version| version.trim().to_owned())
        .filter(|version| !version.is_empty())
}

fn gl_extension_for(gpus: &[Gpu]) -> Option<GlExtension> {
    let driver_version = gpus.iter().find_map(|gpu| gpu.driver_version.clone())?;
    let installed = gl_extension_is_mounted(&driver_version);
    Some(GlExtension {
        driver_version,
        installed,
    })
}

fn gl_extension_is_mounted(driver_version: &str) -> bool {
    let name = format!("nvidia-{}", driver_version.replace('.', "-"));
    std::fs::read_dir("/usr/lib")
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|architecture| architecture.path().join("GL").join(&name).is_dir())
}

#[cfg(test)]
mod tests;
