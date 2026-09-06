//! Consulta HTTP de la última publicación oficial en GitHub (ADR-0015).

use std::time::Duration;

/// Extremo de la API de GitHub para la última versión publicada.
pub const LATEST_RELEASE_ENDPOINT: &str =
    "https://api.github.com/repos/sgomez/rfirma/releases/latest";

/// Tiempo máximo de espera para la respuesta del servidor.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Obtiene el cuerpo de respuesta de la última publicación si está disponible.
pub fn latest_release() -> Option<String> {
    std::thread::spawn(ask_github).join().ok()?
}

fn ask_github() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("rfirma/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let response = client
        .get(LATEST_RELEASE_ENDPOINT)
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
