//! Consulta HTTP de la última publicación oficial en GitHub (ADR-0015).

use std::time::Duration;

use crate::desktop::domain::channel::Channel;

/// Extremo de la API de GitHub para la última versión publicada.
pub const LATEST_RELEASE_ENDPOINT: &str =
    "https://api.github.com/repos/sgomez/rfirma/releases/latest";

/// `latest.json` estable del servidor propio, el feed del canal de Windows.
pub const WINDOWS_FEED_ENDPOINT: &str = "https://rfirma.sgomez.me/windows/latest.json";

/// Tiempo máximo de espera para la respuesta del servidor.
const TIMEOUT: Duration = Duration::from_secs(4);

/// Obtiene el cuerpo del feed del canal si está disponible.
pub fn latest_release(channel: Channel) -> Option<String> {
    let endpoint = endpoint_for(channel);
    std::thread::spawn(move || ask(endpoint)).join().ok()?
}

/// Dónde publica su última versión cada canal.
fn endpoint_for(channel: Channel) -> &'static str {
    match channel {
        Channel::Windows => WINDOWS_FEED_ENDPOINT,
        Channel::Native | Channel::Flatpak => LATEST_RELEASE_ENDPOINT,
    }
}

fn ask(endpoint: &'static str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("rfirma/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let response = client
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .send()
        .ok()?
        .error_for_status()
        .ok()?;

    let body = response.bytes().ok()?;
    String::from_utf8(body.to_vec()).ok()
}

#[cfg(test)]
mod tests;
