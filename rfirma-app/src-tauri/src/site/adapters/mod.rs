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
#[cfg(windows)]
pub mod windows_root;

use std::path::Path;

#[cfg(target_os = "macos")]
use crate::site::domain::trust_error::{Situation, TrustError};
use crate::site::ports::TrustStores;

/// El almacén raíz del usuario de Windows, visto como un perfil más de los almacenes de confianza.
pub const SYSTEM_ROOT_STORE: &str = "cryptoapi:CurrentUser/Root";

/// Si el perfil es el almacén raíz del usuario de Windows y no un perfil NSS.
pub fn is_the_system_root_store(profile: &Path) -> bool {
    profile.as_os_str() == SYSTEM_ROOT_STORE
}

/// Los almacenes de confianza de esta plataforma.
#[cfg(target_os = "linux")]
pub fn desktop_trust_stores() -> Box<dyn TrustStores + Send + Sync> {
    Box::new(nss::NssTrustStores::new(
        crate::identity::adapters::pkcs11::RealNssHost,
    ))
}

/// Los almacenes de confianza de esta plataforma.
#[cfg(windows)]
pub fn desktop_trust_stores() -> Box<dyn TrustStores + Send + Sync> {
    Box::new(windows_root::WindowsUserStores)
}

/// Los perfiles NSS de esta persona, o ninguno si no se sabe cuál es su `HOME`.
#[cfg(target_os = "linux")]
pub fn trust_profiles() -> Vec<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| crate::identity::adapters::pkcs11::stores::nss_profiles(&home))
        .unwrap_or_default()
}

/// Los almacenes de confianza de esta persona.
#[cfg(windows)]
pub use windows_root::trust_profiles;

/// Los almacenes de confianza de esta plataforma.
#[cfg(target_os = "macos")]
pub fn desktop_trust_stores() -> Box<dyn TrustStores + Send + Sync> {
    Box::new(PendingMacosKeychainTrust)
}

/// Ningún almacén de confianza: sin llavero de macOS no hay dónde instalar la CA local.
#[cfg(target_os = "macos")]
pub fn trust_profiles() -> Vec<std::path::PathBuf> {
    Vec::new()
}

/// La confianza en la CA local sobre el llavero de macOS, que aún no existe: toda escritura falla.
#[cfg(target_os = "macos")]
pub struct PendingMacosKeychainTrust;

#[cfg(target_os = "macos")]
impl TrustStores for PendingMacosKeychainTrust {
    fn install(
        &self,
        _profile: &Path,
        _certificate_der: &[u8],
        _nickname: &str,
    ) -> Result<(), TrustError> {
        Err(pending_macos_keychain_trust(Situation::TrustNotWritten))
    }

    fn trust_of(
        &self,
        _profile: &Path,
        _certificate_der: &[u8],
    ) -> Result<Option<u32>, TrustError> {
        Ok(None)
    }

    fn withdraw(&self, _profile: &Path, _certificate_der: &[u8]) -> Result<(), TrustError> {
        Err(pending_macos_keychain_trust(Situation::TrustNotWithdrawn))
    }
}

#[cfg(target_os = "macos")]
fn pending_macos_keychain_trust(situation: Situation) -> TrustError {
    TrustError::new(
        situation,
        "el llavero de macOS aún no está disponible para la CA local",
    )
}

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

/// El detalle de una petición que no llegó: dice que el certificado del servidor no es de confianza cuando fue eso (ADR-0039).
fn send_failure(error: &reqwest::Error) -> String {
    if !is_untrusted_certificate(error) {
        return error.to_string();
    }
    let server = error
        .url()
        .and_then(|url| url.host_str())
        .unwrap_or("desconocido");
    format!("el certificado del servidor {server} no es de confianza: {error}")
}

/// Los códigos con que Schannel rechaza una cadena que no llega a una raíz de confianza, que en
/// Windows ocupan el lugar del `certificate verify failed` de OpenSSL (ADR-0035).
const SCHANNEL_UNTRUSTED_CHAIN: [&str; 4] = [
    "Os { code: -2146893019,", // SEC_E_UNTRUSTED_ROOT
    "Os { code: -2146762487,", // CERT_E_UNTRUSTEDROOT
    "Os { code: -2146762486,", // CERT_E_CHAINING
    "Os { code: -2146869244,", // TRUST_E_CERT_SIGNATURE
];

/// Los códigos con que Security.framework rechaza esa misma cadena en macOS (ADR-0040).
const SECURITY_FRAMEWORK_UNTRUSTED_CHAIN: [&str; 5] = [
    "Error { code: -25318,", // errSecCreateChainFailed
    "Error { code: -67843,", // errSecNotTrusted
    "Error { code: -9807,",  // errSSLXCertChainInvalid
    "Error { code: -9812,",  // errSSLUnknownRootCert
    "Error { code: -9813,",  // errSSLNoRootCert
];

fn is_untrusted_certificate(error: &reqwest::Error) -> bool {
    let mut cause: Option<&dyn std::error::Error> = Some(error);
    while let Some(current) = cause {
        if current.to_string().contains("certificate verify failed") {
            return true;
        }
        let debug = format!("{current:?}");
        if SCHANNEL_UNTRUSTED_CHAIN
            .iter()
            .chain(&SECURITY_FRAMEWORK_UNTRUSTED_CHAIN)
            .any(|code| debug.contains(code))
        {
            return true;
        }
        cause = current.source();
    }
    false
}

fn is_rejection(status: reqwest::StatusCode) -> bool {
    status.is_client_error() || status.is_server_error()
}
