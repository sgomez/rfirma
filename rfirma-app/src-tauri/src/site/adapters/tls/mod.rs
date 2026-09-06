//! Material criptográfico del canal: CA local y certificado del servidor (ADR-0005).

pub mod authority;
pub mod server;
pub mod store;

pub use crate::site::domain::tls_error::{Situation, TlsError};
pub use authority::LocalCa;
pub use server::LocalServerCertificate;
pub use store::{CaFiles, LocalCaStore};
