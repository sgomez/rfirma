//! **El puerto de red**, del lado de fuera: lo único de rFirma que abre una
//! conexión (ID-171, ID-182).
//!
//! Aquí no se decide nada. Este módulo pregunta a GitHub por la última
//! publicación y devuelve **el cuerpo tal cual**, o nada. Qué versión anuncia
//! ese cuerpo, si es más nueva que la que se está ejecutando y cada cuánto se
//! vuelve a preguntar son decisiones, y viven en
//! [`crate::app::version`] (ID-182). Por eso la firma no menciona ni JSON ni
//! `Version`: quien la dobla en las pruebas devuelve una cadena.
//!
//! **Se le pregunta a GitHub y no a `rfirma.sgomez.me`** (ID-178): un
//! `latest.json` propio sería un sitio más —derivado— donde vive la versión.
//! Y **no hay autoactualización** (ID-177): esto trae un número para enseñar
//! una franja, no un artefacto para instalar. Sin clave minisign, sin
//! `latest.json` de Tauri y sin capability del *updater*.
//!
//! # Sin red, silencio
//!
//! Cualquier tropiezo —sin red, DNS que no resuelve, GitHub que contesta 500,
//! respuesta que no llega a tiempo— es `None`. No hay error que enseñar: nadie
//! ha pedido esto, se pregunta sola al arrancar, y un aviso de que no se pudo
//! preguntar sería ruido por algo que al usuario no le importa.

use std::time::Duration;

/// A quién se le pregunta. La API de GitHub excluye de `/releases/latest` las
/// *prereleases*, así que una etiqueta `-rc.N` no llega hasta aquí.
pub const LATEST_RELEASE_ENDPOINT: &str =
    "https://api.github.com/repos/sgomez/rfirma/releases/latest";

/// Lo que se espera como mucho por la respuesta entera.
///
/// Corto a propósito: esto corre al arrancar y nadie lo está esperando. Si
/// GitHub tarda más, la franja no aparece esta vez y se vuelve a preguntar en
/// la siguiente sesión.
const TIMEOUT: Duration = Duration::from_secs(10);

/// El cuerpo de la respuesta de GitHub, o nada.
///
/// # Por qué un hilo propio
///
/// El cliente **bloqueante** de `reqwest` monta su propio runtime, y hacerlo
/// desde dentro de otro es un pánico (`Cannot start a runtime from within a
/// runtime`). La orden que llama a esto es `#[tauri::command(async)]`, así que
/// puede correr ya dentro del runtime de Tauri: el hilo suelto hace que dé
/// igual desde dónde se llame. Es un hilo por arranque, no por segundo.
pub fn latest_release() -> Option<String> {
    std::thread::spawn(ask_github).join().ok()?
}

/// La petición, ya en un hilo sin runtime alrededor.
fn ask_github() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        // La API de GitHub rechaza con 403 lo que no se presenta.
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

    // `bytes` y no `text`: la respuesta es UTF-8 por contrato de la API, y
    // `text` arrastraría la detección de juego de caracteres de `reqwest`
    // —una bandera más y un crate más— para adivinar lo que ya se sabe.
    let body = response.bytes().ok()?;
    String::from_utf8(body.to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::LATEST_RELEASE_ENDPOINT;

    /// **Grada A**: ninguna prueba de este módulo abre un socket (TD-39). Lo
    /// único que se puede comprobar sin red es a quién dice que pregunta.
    #[test]
    fn it_asks_github_for_the_latest_release_and_nobody_else() {
        assert_eq!(
            LATEST_RELEASE_ENDPOINT, "https://api.github.com/repos/sgomez/rfirma/releases/latest",
            "se le pregunta a GitHub, no a rfirma.sgomez.me (ID-178)"
        );
    }
}
