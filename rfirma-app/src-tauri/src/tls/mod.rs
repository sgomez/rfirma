//! Material criptográfico del canal: CA local y certificado del servidor (ADR-0005).

pub mod authority;
pub mod error;
pub mod server;
pub mod store;

pub use authority::LocalCa;
pub use error::{Situation, TlsError};
pub use server::LocalServerCertificate;
pub use store::{CaFiles, LocalCaStore};
