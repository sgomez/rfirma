//! Lee de DRM, del módulo del kernel de NVIDIA y del entorno los gráficos de esta sesión para `--debug-info`.

use std::path::Path;

use crate::desktop::adapters::webkit_renderer::the_variable_in_force;
use crate::desktop::application::debug_report::{GlExtension, Gpu, Graphics};
use crate::desktop::domain::channel::Channel;

const DRM_DIRECTORY: &str = "/sys/class/drm";
const NVIDIA_MODULE_VERSION: &str = "/sys/module/nvidia/version";
const NVIDIA_DRIVER: &str = "nvidia";

/// Los gráficos de esta sesión; fuera de Linux, sin datos.
pub fn this_session_graphics(channel: Channel) -> Graphics {
    if std::env::consts::OS != "linux" {
        return Graphics::default();
    }
    let gpus = gpus_in(Path::new(DRM_DIRECTORY));
    let gl_extension = (channel == Channel::Flatpak)
        .then(|| gl_extension_for(&gpus))
        .flatten();
    Graphics {
        gpus,
        renderer: the_variable_in_force(|name| std::env::var(name).ok().filter(|v| !v.is_empty())),
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
