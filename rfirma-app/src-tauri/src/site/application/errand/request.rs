//! Peticiones de la sede desacopladas de la versión del protocolo.

use crate::site::domain::batch::LocalBatch;
use crate::site::domain::protocol::{
    BatchRequest, LoadRequest, PendingSignRequest, Refusal, SaveRequest, SelectCertificate,
    SignAndSaveRequest, SignRequest,
};

/// Lo que la sede pide, ya leído y sin versión de protocolo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteRequest {
    /// Selección de un certificado que cumpla con el filtro especificado.
    SelectCertificate(SelectCertificate),
    /// Firma de documento en formato PAdES.
    Sign(SignRequest),
    /// Firma de un documento que la sede no mandó: lo elige la persona.
    SignWithoutDocument(PendingSignRequest),
    /// Firma de documento en formato PAdES, guardada por el portal tras la postfirma.
    SignAndSave(SignAndSaveRequest),
    /// Guardar un fichero en el equipo.
    Save(SaveRequest),
    /// Cargar uno o varios ficheros del equipo.
    Load(LoadRequest),
    /// Firmar un lote remoto contra los dos servlets que declara la sede.
    Batch(BatchRequest),
    /// Firmar aquí mismo el lote local que manda la sede.
    LocalBatch(Box<LocalBatchAsk>),
    /// Operación no atendida con el rechazo correspondiente.
    NotAttended(Refusal),
}

/// El lote local, leído o no, junto con lo que la sede pidió a su alrededor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBatchAsk {
    /// El lote tal y como lo pidió la sede.
    pub request: BatchRequest,
    /// Las firmas del lote leídas del JSON, o por qué no se pudieron leer.
    pub batch: Result<LocalBatch, Refusal>,
}
