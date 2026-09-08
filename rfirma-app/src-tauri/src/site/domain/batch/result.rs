//! El resultado del lote, el del remoto que no pudo prefirmar nada y el del local (`JSONBatchInfoParser`, `JSONBatchManager.buildBatchResultJson`, 1.9.2).

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use super::json::Json;
use super::presign::{BatchDataResult, PresignResult};

/// El resultado de una firma del lote local (`LocalSingleBatchResult`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBatchResult {
    id: String,
    result: PresignResult,
    description: Option<String>,
    signature: Option<Vec<u8>>,
}

impl LocalBatchResult {
    /// La firma que salió, con su resultado en Base64.
    pub fn signed(id: impl Into<String>, signature: Vec<u8>) -> Self {
        Self {
            id: id.into(),
            result: PresignResult::DoneAndSaved,
            description: None,
            signature: Some(signature),
        }
    }

    /// La firma que no se llegó a intentar porque el lote paró antes.
    pub fn skipped(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            result: PresignResult::Skipped,
            description: None,
            signature: None,
        }
    }

    /// La firma que falló, con el motivo que la sede recibe.
    pub fn failed(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            result: PresignResult::ErrorPre,
            description: Some(description.into()),
            signature: None,
        }
    }

    /// Pasa a saltada y suelta la firma, como hace el lote que para en el primer error.
    pub fn skip(&mut self) {
        self.result = PresignResult::Skipped;
        self.signature = None;
    }

    /// El identificador de la firma a la que pertenece.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// El estado de la firma.
    pub fn result(&self) -> PresignResult {
        self.result
    }

    /// El motivo, si lo hay.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// La firma, si la hay.
    pub fn signature(&self) -> Option<&[u8]> {
        self.signature.as_deref()
    }
}

/// El resultado del lote con los errores individuales que impidieron prefirmar nada.
pub fn build_result(errors: &[BatchDataResult]) -> Vec<u8> {
    let signs = errors
        .iter()
        .map(|error| single_sign(error.id(), error.result(), error.description(), None))
        .collect();

    document_of(signs)
}

/// El resultado del lote local, con un elemento por firma (`JSONBatchManager.buildBatchResultJson`, 1.9.2).
pub fn build_local_result(results: &[LocalBatchResult]) -> Vec<u8> {
    let signs = results
        .iter()
        .map(|result| {
            single_sign(
                result.id(),
                result.result(),
                result.description(),
                result.signature(),
            )
        })
        .collect();

    document_of(signs)
}

/// El resultado del lote cuando la prefirma no devolvió ni firmas ni errores.
pub fn build_empty_result() -> Vec<u8> {
    document_of(Vec::new())
}

fn single_sign(
    id: &str,
    result: PresignResult,
    description: Option<&str>,
    signature: Option<&[u8]>,
) -> Json {
    let mut object = vec![
        ("id".to_owned(), Json::String(id.to_owned())),
        (
            "result".to_owned(),
            Json::String(result.as_str().to_owned()),
        ),
    ];
    if let Some(description) = description {
        object.push((
            "description".to_owned(),
            Json::String(description.to_owned()),
        ));
    }
    if let Some(signature) = signature {
        object.push((
            "signature".to_owned(),
            Json::String(STANDARD.encode(signature)),
        ));
    }

    Json::Object(object)
}

fn document_of(signs: Vec<Json>) -> Vec<u8> {
    Json::Object(vec![("signs".to_owned(), Json::Array(signs))])
        .to_json_string()
        .into_bytes()
}

#[cfg(test)]
mod tests;
