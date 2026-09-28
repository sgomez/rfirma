//! Adaptadores de `site`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri.

pub mod batch_services;
pub mod channel;
pub mod codec;
pub mod codec_relay;
pub mod codec_v1;
pub mod codec_v3;
pub mod cookies;
pub mod data_download;
pub mod desk;
pub mod frontier;
#[cfg(test)]
mod header_probe;
pub mod nss;
pub mod relay;
pub mod scratch;
pub mod service;
pub mod servlets;
pub mod tauri;
pub mod tls;
pub mod trace;
pub mod transport;
pub mod triphase_server;
pub mod views;
pub mod window;

const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// El cliente de los servicios de la sede: límite al conectar, ninguno a la respuesta (ADR-0037) y las cookies de la operación (ADR-0038).
fn service_client(cookies: std::sync::Arc<cookies::OperationCookies>) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .cookie_provider(cookies)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(None)
        .build()
        .expect("el cliente HTTP se construye con parametros validos")
}

/// La línea de estado de una respuesta de error seguida de su cuerpo, como el `HttpError` del original.
fn rejection_detail(response: reqwest::blocking::Response) -> String {
    let status = response.status();
    match response.text() {
        Ok(body) if !body.trim().is_empty() => format!("{status}: {}", body.trim()),
        _ => status.to_string(),
    }
}

fn is_rejection(status: reqwest::StatusCode) -> bool {
    status.is_client_error() || status.is_server_error()
}
