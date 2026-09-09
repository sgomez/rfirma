//! El cliente HTTP que baja el `dat` que la sede manda como URL, sobre `reqwest::blocking`.

use std::time::Duration;

use crate::site::domain::protocol::DataSource;

const TIMEOUT: Duration = Duration::from_secs(30);

/// El origen de datos de producción: un GET con la validación TLS del sistema.
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpDataSource;

impl DataSource for HttpDataSource {
    fn download(&self, url: &str) -> Result<Vec<u8>, String> {
        let url = url.to_owned();
        execute_outside_tokio(move || {
            reqwest::blocking::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .map_err(|error| error.to_string())?
                .get(url)
                .send()
                .map_err(|error| error.to_string())?
                .error_for_status()
                .map_err(|error| error.to_string())?
                .bytes()
                .map(|bytes| bytes.to_vec())
                .map_err(|error| error.to_string())
        })
    }
}

fn execute_outside_tokio<T: Send + 'static>(action: impl FnOnce() -> T + Send + 'static) -> T {
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::Builder::new()
            .name("dat-download-io".into())
            .spawn(action)
            .expect("el hilo para la descarga del 'dat' se crea")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    } else {
        action()
    }
}

#[cfg(test)]
mod tests;
