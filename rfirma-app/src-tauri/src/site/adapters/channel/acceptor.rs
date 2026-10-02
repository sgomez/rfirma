//! Saludo TLS del servidor local: `native-tls` en Linux, `rustls` en Windows y macOS (ADR-0036).

use std::io;

use tokio::net::TcpStream;

/// Aceptador TLS con el certificado y la clave del servidor local.
pub(crate) struct LocalTlsAcceptor(Inner);

#[cfg(target_os = "linux")]
type Inner = tokio_native_tls::TlsAcceptor;

#[cfg(not(target_os = "linux"))]
type Inner = tokio_rustls::TlsAcceptor;

/// El flujo ya cifrado de una conexión aceptada.
#[cfg(target_os = "linux")]
pub(crate) type LocalTlsStream = tokio_native_tls::TlsStream<TcpStream>;

/// El flujo ya cifrado de una conexión aceptada.
#[cfg(not(target_os = "linux"))]
pub(crate) type LocalTlsStream = tokio_rustls::server::TlsStream<TcpStream>;

impl LocalTlsAcceptor {
    /// Construye el aceptador desde el certificado PEM y la clave PEM PKCS#8.
    pub(crate) fn from_pem(certificate: &[u8], key: &[u8]) -> Result<Self, String> {
        inner_from_pem(certificate, key).map(Self)
    }

    /// Completa el saludo TLS de una conexión entrante.
    pub(crate) async fn accept(&self, stream: TcpStream) -> io::Result<LocalTlsStream> {
        handshake(&self.0, stream).await
    }
}

#[cfg(target_os = "linux")]
async fn handshake(acceptor: &Inner, stream: TcpStream) -> io::Result<LocalTlsStream> {
    acceptor.accept(stream).await.map_err(io::Error::other)
}

#[cfg(not(target_os = "linux"))]
async fn handshake(acceptor: &Inner, stream: TcpStream) -> io::Result<LocalTlsStream> {
    acceptor.accept(stream).await
}

#[cfg(target_os = "linux")]
fn inner_from_pem(certificate: &[u8], key: &[u8]) -> Result<Inner, String> {
    use tokio_native_tls::native_tls::{Identity, TlsAcceptor};

    let identity = Identity::from_pkcs8(certificate, key).map_err(|error| error.to_string())?;
    let acceptor = TlsAcceptor::new(identity).map_err(|error| error.to_string())?;
    Ok(Inner::from(acceptor))
}

#[cfg(not(target_os = "linux"))]
fn inner_from_pem(certificate: &[u8], key: &[u8]) -> Result<Inner, String> {
    use std::sync::Arc;
    use tokio_rustls::rustls::crypto::ring::default_provider;
    use tokio_rustls::rustls::pki_types::pem::PemObject;
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use tokio_rustls::rustls::ServerConfig;

    let chain = CertificateDer::pem_slice_iter(certificate)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let key = PrivateKeyDer::from_pem_slice(key).map_err(|error| error.to_string())?;
    let config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|error| error.to_string())?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|error| error.to_string())?;
    Ok(Inner::from(Arc::new(config)))
}
