//! Peticiones de la sede desacopladas de la versión del protocolo.

use crate::site::domain::protocol::{
    LoadRequest, Refusal, SaveRequest, SelectCertificate, SignRequest,
};

/// Lo que la sede pide, ya leído y sin versión de protocolo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteRequest {
    /// Selección de un certificado que cumpla con el filtro especificado.
    SelectCertificate(SelectCertificate),
    /// Firma de documento en formato PAdES.
    Sign(SignRequest),
    /// Guardar un fichero en el equipo.
    Save(SaveRequest),
    /// Cargar uno o varios ficheros del equipo.
    Load(LoadRequest),
    /// Operación no atendida con el rechazo correspondiente.
    NotAttended(Refusal),
}
