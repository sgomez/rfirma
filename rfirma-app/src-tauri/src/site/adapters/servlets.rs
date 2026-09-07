//! Adaptador `reqwest` sobre los servlets del servidor intermedio (1.9.2, `UrlParameters.java:351-379`).

use std::time::Duration;

use crate::site::domain::relay_error::{RelayError, Situation};
use crate::site::ports::Servlets;

const OPERATION_VERSION: &str = "1_0";
const TIMEOUT: Duration = Duration::from_secs(30);

/// Literal de espera activa, byte a byte con el original (1.9.2, `ActiveWaitingThread.java:14`).
const WAIT_MARKER: &str = "#WAIT";

/// El servidor intermedio de producción, sobre `reqwest::blocking` con la validación TLS del sistema.
pub struct RelayServlets {
    client: reqwest::blocking::Client,
}

impl Default for RelayServlets {
    fn default() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .expect("el cliente HTTP se construye con parametros validos"),
        }
    }
}

impl Servlets for RelayServlets {
    fn retrieve(&self, service_url: &str, id: &str) -> Result<String, RelayError> {
        let url = validated_servlet_url(service_url)?;
        let params = operation_params("get", id, None);
        self.client
            .post(url)
            .form(&params)
            .send()
            .map_err(unreachable)?
            .error_for_status()
            .map_err(unreachable)?
            .text()
            .map_err(unreachable)
    }

    fn store(&self, service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        let url = validated_servlet_url(service_url)?;
        let params = operation_params("put", id, Some(data));
        self.client
            .post(url)
            .form(&params)
            .send()
            .map_err(unreachable)?
            .error_for_status()
            .map_err(rejected)?;
        Ok(())
    }

    fn wait(&self, service_url: &str, id: &str) -> Result<(), RelayError> {
        self.store(service_url, id, WAIT_MARKER)
    }
}

/// Compone el cuerpo del servlet como `IntermediateServerUtil`: `op`, `v` y `id`, con `dat` cuando lo hay.
///
/// Va en el cuerpo `application/x-www-form-urlencoded` de un POST, nunca en la *query string*
/// (1.9.2, `UrlHttpManagerImpl.java:188-277`): `dat` lleva datos de decenas o cientos de KB y
/// cualquier contenedor de servlets corta antes la URL.
fn operation_params<'a>(
    operation: &'a str,
    id: &'a str,
    data: Option<&'a str>,
) -> Vec<(&'a str, &'a str)> {
    let mut params = vec![("op", operation), ("v", OPERATION_VERSION), ("id", id)];
    if let Some(data) = data {
        params.push(("dat", data));
    }
    params
}

fn unreachable(error: reqwest::Error) -> RelayError {
    RelayError::new(Situation::ServletUnreachable, error.to_string())
}

fn rejected(error: reqwest::Error) -> RelayError {
    RelayError::new(Situation::UploadRejected, error.to_string())
}

/// Valida la URL del servlet como el original: `http`/`https`, host no local y sin parámetros.
fn validated_servlet_url(service_url: &str) -> Result<reqwest::Url, RelayError> {
    let url = reqwest::Url::parse(service_url)
        .map_err(|error| RelayError::new(Situation::ServletUnreachable, error.to_string()))?;

    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(RelayError::new(
            Situation::ServletUnreachable,
            format!("protocolo no soportado para el servlet: {}", url.scheme()),
        ));
    }

    if matches!(url.host_str(), Some("localhost") | Some("127.0.0.1")) {
        return Err(RelayError::new(
            Situation::ServletUnreachable,
            "el host de la URL del servlet es local",
        ));
    }

    if service_url.contains('?') || service_url.contains('=') {
        return Err(RelayError::new(
            Situation::ServletUnreachable,
            "la URL del servlet no admite parametros propios",
        ));
    }

    Ok(url)
}

#[cfg(test)]
mod tests;
