//! El lote sin trámite: `TriphaseData`, la prefirma con errores del remoto, la lectura del local y el resultado de los dos.

pub mod json;
pub mod local;
pub mod presign;
pub mod result;
pub mod triphase;

pub use local::{parse_local_batch, LocalBatch, LocalSingleSign};
pub use presign::{
    parse_json_presign, update_batch_with_errors, BatchDataResult, PresignError, PresignOutcome,
    PresignResult,
};
pub use result::{build_empty_result, build_local_result, build_result, LocalBatchResult};
pub use triphase::{apply_pk1, TriSign, TriphaseData, TriphaseDataError};

/// El formato en el que viaja el lote: el XML heredado, o JSON con `jsonbatch=true`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchFormat {
    /// El XML heredado (`signbatch`, sin `jsonbatch`).
    Xml,
    /// El lote en JSON (`jsonbatch=true`).
    Json,
}

impl BatchFormat {
    /// El nombre del parámetro con el que el lote viaja a los servlets: `xml` o `json`.
    pub fn param_name(self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Json => "json",
        }
    }
}
