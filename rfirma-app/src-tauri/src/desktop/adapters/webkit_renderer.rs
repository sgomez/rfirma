//! Si WebKitGTK debe componer sin la GPU en esta sesión; no fija nada en el entorno ni elige nada más del webview.

/// La variable de WebKitGTK que apaga la composición acelerada.
pub const COMPOSITING_SWITCH: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";

const RENDERER_CHOICES: [&str; 4] = [
    COMPOSITING_SWITCH,
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
    "__NV_DISABLE_EXPLICIT_SYNC",
];

/// Si la sesión es X11 y el entorno no elige ya el renderizador.
pub fn compositing_must_be_turned_off(variable: impl Fn(&str) -> Option<String>) -> bool {
    if RENDERER_CHOICES.iter().any(|name| variable(name).is_some()) {
        return false;
    }
    let forced = variable("GDK_BACKEND")
        .and_then(|backends| backends.split(',').next().map(str::trim).map(String::from))
        .filter(|backend| !backend.is_empty() && backend != "*");
    match forced {
        Some(backend) => backend == "x11",
        None => variable("WAYLAND_DISPLAY").is_none_or(|display| display.is_empty()),
    }
}

#[cfg(test)]
mod tests;
