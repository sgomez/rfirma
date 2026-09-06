//! Peticiones de la sede desacopladas de la versión del protocolo.

use crate::site::domain::protocol::{Refusal, SelectCertificate, SignRequest};

/// Lo que la sede pide, ya leído y sin versión de protocolo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteRequest {
    /// Selección de un certificado que cumpla con el filtro especificado.
    SelectCertificate(SelectCertificate),
    /// Firma de documento en formato PAdES.
    Sign(SignRequest),
    /// Operación no atendida con el rechazo correspondiente.
    NotAttended(Refusal),
}
