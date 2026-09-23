//! Adaptador `reqwest` sobre el servidor trifásico que la sede nombra en `serverUrl`; no es el de los servlets del lote.

use std::time::Duration;

use crate::site::domain::triphase_server::{Situation, TriphaseServerError};
use crate::site::ports::TriphaseServer;

const TIMEOUT: Duration = Duration::from_secs(30);

/// El servidor trifásico de producción, sobre `reqwest::blocking` con la validación TLS del sistema.
pub struct HttpTriphaseServer {
    client: reqwest::blocking::Client,
}

impl Default for HttpTriphaseServer {
    fn default() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .expect("el cliente HTTP se construye con parametros validos"),
        }
    }
}

impl TriphaseServer for HttpTriphaseServer {
    fn post(
        &self,
        server_url: &str,
        form: &[(&'static str, String)],
    ) -> Result<Vec<u8>, TriphaseServerError> {
        let url = reqwest::Url::parse(server_url).map_err(|error| {
            TriphaseServerError::new(Situation::ServerUrlMissing, error.to_string())
        })?;
        let client = self.client.clone();
        let form = form.to_vec();
        outside_tokio(move || {
            client
                .post(url)
                .form(&form)
                .send()
                .map_err(|error| unreachable(&error))?
                .error_for_status()
                .map_err(|error| unreachable(&error))?
                .bytes()
                .map(|bytes| bytes.to_vec())
                .map_err(|error| unreachable(&error))
        })
    }
}

fn unreachable(error: &reqwest::Error) -> TriphaseServerError {
    TriphaseServerError::new(Situation::ServerUnreachable, error.to_string())
}

fn outside_tokio<T: Send + 'static>(action: impl FnOnce() -> T + Send + 'static) -> T {
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::Builder::new()
            .name("triphase-server-io".into())
            .spawn(action)
            .expect("el hilo para la llamada HTTP al servidor trifasico se crea")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    } else {
        action()
    }
}

#[cfg(test)]
mod tests;
