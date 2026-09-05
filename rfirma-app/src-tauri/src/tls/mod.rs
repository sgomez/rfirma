//! El material criptográfico del canal: **la CA local** y **el certificado del
//! servidor local** (ADR-0005, ID-220…ID-226).
//!
//! Son **dos piezas con dos vidas distintas**, y el ID-220 fija cómo se
//! llaman: ni «ancla» ni «hoja» son palabras de este proyecto.
//!
//! | | Quién la firma | Dónde vive | Cuánto dura |
//! |---|---|---|---|
//! | [`LocalCa`] | ella misma | dos ficheros del directorio de datos, la clave en `0600` | [`authority::VALIDITY_DAYS`] |
//! | [`LocalServerCertificate`] | la CA local | solo en memoria | lo que vive el proceso |
//!
//! Esto es **la fábrica y nada más**: aquí no se registra nada en ningún
//! almacén NSS, no se decide cuándo renovar ni se levanta ningún servidor.
//!
//! Los certificados se generan con el crate `openssl` (ID-225). Se descartó
//! `rcgen` porque arrastra `ring`, y `x509-cert` —que ya está en el árbol para
//! *leer* certificados del token— porque obligaría a firmar en Rust.

pub mod authority;
pub mod error;
pub mod server;
pub mod store;

pub use authority::LocalCa;
pub use error::{Situation, TlsError};
pub use server::LocalServerCertificate;
pub use store::LocalCaStore;
