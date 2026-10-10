//! Qué le quita rFirma a WebKitGTK en esta sesión (ADR-0007): solo la decisión, que fija `titlebar.rs`; no elige nada más del webview.

use crate::desktop::application::debug_report::{RendererOrigin, RendererVariable};

/// La variable de WebKitGTK que apaga la composición acelerada.
pub const COMPOSITING_SWITCH: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";

/// La variable de WebKitGTK que apaga el renderizador DMA-BUF.
pub const DMABUF_SWITCH: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

const RENDERER_CHOICES: [&str; 4] = [
    COMPOSITING_SWITCH,
    DMABUF_SWITCH,
    "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
    "__NV_DISABLE_EXPLICIT_SYNC",
];

/// La variable que rige el renderizador en esta sesión: la del entorno si ya eligió, o la que fijará rFirma.
pub fn the_variable_in_force(
    variable: impl Fn(&str) -> Option<String>,
) -> Option<RendererVariable> {
    if let Some((name, value)) = RENDERER_CHOICES
        .iter()
        .find_map(|name| variable(name).map(|value| (*name, value)))
    {
        return Some(RendererVariable {
            name: name.to_owned(),
            value,
            origin: RendererOrigin::FromTheEnvironment,
        });
    }
    the_switch_for_this_session(variable).map(|name| RendererVariable {
        name: name.to_owned(),
        value: "1".to_owned(),
        origin: RendererOrigin::SetByRfirma,
    })
}

/// La variable que hay que fijar en esta sesión, o ninguna si el entorno ya elige el renderizador.
pub fn the_switch_for_this_session(
    variable: impl Fn(&str) -> Option<String>,
) -> Option<&'static str> {
    if RENDERER_CHOICES.iter().any(|name| variable(name).is_some()) {
        return None;
    }
    let forced = variable("GDK_BACKEND")
        .and_then(|backends| backends.split(',').next().map(str::trim).map(String::from))
        .filter(|backend| !backend.is_empty() && backend != "*");
    let on_x11 = match forced {
        Some(backend) => backend == "x11",
        None => variable("WAYLAND_DISPLAY").is_none_or(|display| display.is_empty()),
    };
    Some(if on_x11 {
        COMPOSITING_SWITCH
    } else {
        DMABUF_SWITCH
    })
}

#[cfg(test)]
mod tests;
