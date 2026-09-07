//! La prefirma JSON del lote remoto con errores por elemento, y el lote actualizado con ellos (`JSONPreSignBatchParser`, `JSONBatchInfo`, 1.9.2).

use std::fmt;

use super::json::Json;
use super::triphase::TriphaseData;

/// Un `JSONPreSignBatchParser` o un `JSONBatchInfoParser` mal formados.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresignError(String);

impl fmt::Display for PresignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PresignError {}

/// El estado de una firma parcial del lote (`BatchDataResult.Result`, 1.9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresignResult {
    NotStarted,
    DoneAndSaved,
    DoneButNotSavedYet,
    DoneButSaveSkipped,
    DoneButErrorSaving,
    ErrorPre,
    ErrorPost,
    Skipped,
    SaveRollbacked,
}

impl PresignResult {
    /// El literal tal y como viaja en el JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "NOT_STARTED",
            Self::DoneAndSaved => "DONE_AND_SAVED",
            Self::DoneButNotSavedYet => "DONE_BUT_NOT_SAVED_YET",
            Self::DoneButSaveSkipped => "DONE_BUT_SAVED_SKIPPED",
            Self::DoneButErrorSaving => "DONE_BUT_ERROR_SAVING",
            Self::ErrorPre => "ERROR_PRE",
            Self::ErrorPost => "ERROR_POST",
            Self::Skipped => "SKIPPED",
            Self::SaveRollbacked => "SAVE_ROLLBACKED",
        }
    }

    fn parse(literal: &str) -> Option<Self> {
        Some(match literal {
            "NOT_STARTED" => Self::NotStarted,
            "DONE_AND_SAVED" => Self::DoneAndSaved,
            "DONE_BUT_NOT_SAVED_YET" => Self::DoneButNotSavedYet,
            "DONE_BUT_SAVED_SKIPPED" => Self::DoneButSaveSkipped,
            "DONE_BUT_ERROR_SAVING" => Self::DoneButErrorSaving,
            "ERROR_PRE" => Self::ErrorPre,
            "ERROR_POST" => Self::ErrorPost,
            "SKIPPED" => Self::Skipped,
            "SAVE_ROLLBACKED" => Self::SaveRollbacked,
            _ => return None,
        })
    }
}

/// El resultado parcial de una firma del lote (`BatchDataResult`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchDataResult {
    id: String,
    result: PresignResult,
    description: Option<String>,
}

impl BatchDataResult {
    /// Crea un resultado parcial.
    pub fn new(id: impl Into<String>, result: PresignResult, description: Option<String>) -> Self {
        Self {
            id: id.into(),
            result,
            description,
        }
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
}

/// Lo que devuelve la prefirma JSON: `TriphaseData` cuando alguna firma se pudo prefirmar, y los
/// errores de las que no (`PresignBatch`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PresignOutcome {
    triphase_data: Option<TriphaseData>,
    errors: Vec<BatchDataResult>,
}

impl PresignOutcome {
    /// El `TriphaseData` de las firmas que sí se prefirmaron, si hay alguna.
    pub fn triphase_data(&self) -> Option<&TriphaseData> {
        self.triphase_data.as_ref()
    }

    /// Los errores de prefirma de las firmas que fallaron.
    pub fn errors(&self) -> &[BatchDataResult] {
        &self.errors
    }
}

/// Lee la respuesta de prefirma JSON: `td` con las firmas que salieron bien, `results` con las
/// que no (`JSONPreSignBatchParser.parseFromJSON`, 1.9.2).
pub fn parse_json_presign(response: &[u8]) -> Result<PresignOutcome, PresignError> {
    let text = std::str::from_utf8(response)
        .map_err(|error| PresignError(format!("la respuesta no es UTF-8: {error}")))?;
    let value = Json::parse(text).map_err(|error| {
        PresignError(format!(
            "el JSON de prefirma del lote no esta formado correctamente: {error}"
        ))
    })?;

    let triphase_data = match value.get("td") {
        Some(td) => Some(
            TriphaseData::parse_json(td.to_json_string().as_bytes())
                .map_err(|error| PresignError(error.to_string()))?,
        ),
        None => None,
    };

    let errors = match value.get("results").and_then(Json::as_array) {
        Some(results) if !results.is_empty() => {
            let mut parsed = Vec::with_capacity(results.len());
            for entry in results {
                let id = entry
                    .get("id")
                    .and_then(Json::as_str)
                    .ok_or_else(|| {
                        PresignError(
                            "se obtuvo un resultado parcial sin identificador o resultado"
                                .to_owned(),
                        )
                    })?
                    .to_owned();
                let literal = entry.get("result").and_then(Json::as_str).ok_or_else(|| {
                    PresignError(
                        "se obtuvo un resultado parcial sin identificador o resultado".to_owned(),
                    )
                })?;
                let result = PresignResult::parse(literal).ok_or_else(|| {
                    PresignError(format!("estado de firma trifasica desconocido: {literal}"))
                })?;
                let description = entry
                    .get("description")
                    .and_then(Json::as_str)
                    .map(str::to_owned);
                parsed.push(BatchDataResult::new(id, result, description));
            }
            parsed
        }
        _ => Vec::new(),
    };

    Ok(PresignOutcome {
        triphase_data,
        errors,
    })
}

/// Actualiza el lote JSON original con los errores de prefirma: cada `singlesign` que falló
/// pierde su referencia a datos y su configuración, y gana su resultado
/// (`JSONBatchInfo.updateResults`, 1.9.2).
pub fn update_batch_with_errors(
    batch_json: &[u8],
    errors: &[BatchDataResult],
) -> Result<Vec<u8>, PresignError> {
    let text = std::str::from_utf8(batch_json)
        .map_err(|error| PresignError(format!("el lote no es UTF-8: {error}")))?;
    let mut value = Json::parse(text).map_err(|error| {
        PresignError(format!(
            "el JSON del lote de firmas no esta formado correctamente: {error}"
        ))
    })?;

    let singlesigns = value
        .as_object_mut()
        .and_then(|entries| entries.iter_mut().find(|(key, _)| key == "singlesigns"))
        .map(|(_, value)| value)
        .and_then(Json::as_array_mut)
        .ok_or_else(|| PresignError("falta 'singlesigns' en el lote".to_owned()))?;

    for single_sign in singlesigns.iter_mut() {
        let Some(id) = single_sign
            .get("id")
            .and_then(Json::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(matched) = errors.iter().find(|error| error.id() == id) else {
            continue;
        };
        single_sign.remove("datareference");
        single_sign.remove("format");
        single_sign.remove("suboperation");
        single_sign.remove("extraparams");
        single_sign.set("result", Json::String(matched.result().as_str().to_owned()));
        if let Some(description) = matched.description() {
            single_sign.set("description", Json::String(description.to_owned()));
        }
    }

    Ok(value.to_json_string().into_bytes())
}

#[cfg(test)]
mod tests;
