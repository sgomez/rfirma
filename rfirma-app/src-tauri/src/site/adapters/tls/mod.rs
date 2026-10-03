//! Material criptográfico del canal en el lado de los adaptadores: el certificado efímero del servidor local y las dos ranuras en disco de la CA local (ADR-0005).

pub mod server;
pub mod store;

pub use server::LocalServerCertificate;
pub use store::{CaFiles, LocalCaStore};
