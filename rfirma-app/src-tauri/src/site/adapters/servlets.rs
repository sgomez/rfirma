//! Adaptador `reqwest` sobre los servlets del servidor intermedio; la forma de la URL no se comprueba aquí.

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
        execute_outside_tokio(|| Self {
            client: reqwest::blocking::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .expect("el cliente HTTP se construye con parametros validos"),
        })
    }
}

impl Servlets for RelayServlets {
    fn retrieve(&self, service_url: &str, id: &str) -> Result<String, RelayError> {
        let url = validated_servlet_url(service_url)?;
        let id = id.to_owned();
        let client = self.client.clone();
        execute_outside_tokio(move || {
            let params = operation_params("get", &id, None);
            client
                .post(url)
                .form(&params)
                .send()
                .map_err(unreachable)?
                .error_for_status()
                .map_err(unreachable)?
                .text()
                .map_err(unreachable)
        })
    }

    fn store(&self, service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        let url = validated_servlet_url(service_url)?;
        let id = id.to_owned();
        let data = data.to_owned();
        let client = self.client.clone();
        execute_outside_tokio(move || {
            let params = operation_params("put", &id, Some(&data));
            client
                .post(url)
                .form(&params)
                .send()
                .map_err(unreachable)?
                .error_for_status()
                .map_err(rejected)?;
            Ok(())
        })
    }

    fn wait(&self, service_url: &str, id: &str) -> Result<(), RelayError> {
        self.store(service_url, id, WAIT_MARKER)
    }
}

fn execute_outside_tokio<T: Send + 'static>(action: impl FnOnce() -> T + Send + 'static) -> T {
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::Builder::new()
            .name("relay-servlet-io".into())
            .spawn(action)
            .expect("el hilo para la llamada HTTP del servidor intermedio se crea")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    } else {
        action()
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

/// La URL del servlet ya leída; su forma la comprobó el dominio al leer la invocación.
fn validated_servlet_url(service_url: &str) -> Result<reqwest::Url, RelayError> {
    reqwest::Url::parse(service_url)
        .map_err(|error| RelayError::new(Situation::ServletUnreachable, error.to_string()))
}

#[cfg(test)]
mod tests;
