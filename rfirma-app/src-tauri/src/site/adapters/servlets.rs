//! Adaptador `reqwest` sobre los servlets del servidor intermedio (1.9.2, `UrlParameters.java:351-379`).

use std::time::Duration;

use crate::site::domain::relay_error::{RelayError, Situation};
use crate::site::ports::Servlets;

const OPERATION_VERSION: &str = "1_0";
const TIMEOUT: Duration = Duration::from_secs(30);

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
        let url = url_with_operation(service_url, "get", id, None)?;
        self.client
            .post(url)
            .send()
            .map_err(unreachable)?
            .error_for_status()
            .map_err(unreachable)?
            .text()
            .map_err(unreachable)
    }

    fn store(&self, service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        let url = url_with_operation(service_url, "put", id, Some(data))?;
        self.client
            .post(url)
            .send()
            .map_err(unreachable)?
            .error_for_status()
            .map_err(rejected)?;
        Ok(())
    }

    fn wait(&self, service_url: &str, id: &str) -> Result<(), RelayError> {
        self.store(service_url, id, "#WAIT")
    }
}

/// Compone la URL del servlet como `IntermediateServerUtil`: `op`, `v` y `id`, con `dat` cuando lo hay.
fn url_with_operation(
    service_url: &str,
    operation: &str,
    id: &str,
    data: Option<&str>,
) -> Result<reqwest::Url, RelayError> {
    let mut url = validated_servlet_url(service_url)?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("op", operation)
            .append_pair("v", OPERATION_VERSION)
            .append_pair("id", id);
        if let Some(data) = data {
            pairs.append_pair("dat", data);
        }
    }
    Ok(url)
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
