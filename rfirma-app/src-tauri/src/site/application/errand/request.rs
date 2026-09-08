//! Peticiones de la sede desacopladas de la versión del protocolo.

use crate::site::domain::protocol::{
    BatchRequest, LoadRequest, Refusal, SaveRequest, SelectCertificate, SignAndSaveRequest,
    SignRequest,
};

/// Lo que la sede pide, ya leído y sin versión de protocolo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteRequest {
    /// Selección de un certificado que cumpla con el filtro especificado.
    SelectCertificate(SelectCertificate),
    /// Firma de documento en formato PAdES.
    Sign(SignRequest),
    /// Firma de documento en formato PAdES, guardada por el portal tras la postfirma.
    SignAndSave(SignAndSaveRequest),
    /// Guardar un fichero en el equipo.
    Save(SaveRequest),
    /// Cargar uno o varios ficheros del equipo.
    Load(LoadRequest),
    /// Firmar un lote remoto contra los dos servlets que declara la sede.
    Batch(BatchRequest),
    /// Operación no atendida con el rechazo correspondiente.
    NotAttended(Refusal),
}
