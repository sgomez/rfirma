//! El lote local leído del JSON que manda la sede (`JSONBatchManager.parseBatchConfig`, 1.9.2).

use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine as _;

use super::json::Json;
use crate::site::domain::protocol::{
    format_of, pairs_of, without_the_launcher_keys, Parameter, Refusal, RequestedFormat, SafCode,
    SignatureRound, AUTO, COSIGN, SIGN,
};

/// Una firma del lote local, con lo que heredó del lote (`SingleSignOperation`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalSingleSign {
    id: String,
    document: Vec<u8>,
    round: SignatureRound,
    format: Option<RequestedFormat>,
    extra_params: Vec<(String, String)>,
}

impl LocalSingleSign {
    /// El identificador con el que la sede nombra esta firma.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// El documento a firmar, ya descodificado.
    pub fn document(&self) -> &[u8] {
        &self.document
    }

    /// `sign` o `cosign`, propio o heredado del lote.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// El formato con el que se firma de verdad, con `auto` resuelto por la cabecera.
    pub fn effective_format(&self) -> RequestedFormat {
        self.format.unwrap_or_else(|| format_of(&self.document))
    }

    /// El formato, propio o heredado del lote; nada si el lote pide `auto`.
    pub fn format(&self) -> Option<RequestedFormat> {
        self.format
    }

    /// Los `extraParams` ya expandidos, propios o heredados del lote.
    pub fn extra_params(&self) -> &[(String, String)] {
        &self.extra_params
    }
}

/// El lote local ya leído: su algoritmo, si para en el primer error y sus firmas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBatch {
    algorithm: String,
    stop_on_error: bool,
    signs: Vec<LocalSingleSign>,
}

impl LocalBatch {
    /// El algoritmo con el que se firma todo el lote.
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Si el lote para en el primer error.
    pub fn stops_on_error(&self) -> bool {
        self.stop_on_error
    }

    /// Las firmas del lote, en el orden en el que la sede las declaró.
    pub fn signs(&self) -> &[LocalSingleSign] {
        &self.signs
    }
}

/// Lee el JSON del lote local, o el rechazo que nombra lo que la sede mandó mal.
pub fn parse_local_batch(config: &[u8]) -> Result<LocalBatch, Refusal> {
    let text = std::str::from_utf8(config).map_err(|error| {
        Refusal::about(Parameter::Data, format!("el lote no es UTF-8: {error}"))
    })?;
    let json = Json::parse(text).map_err(|error| {
        Refusal::about(
            Parameter::Data,
            format!(
                "El JSON de definicion de lote de firmas no esta formado correctamente: {error}"
            ),
        )
    })?;

    let stop_on_error = json
        .get("stoponerror")
        .and_then(Json::as_bool)
        .unwrap_or(false);
    let round = match json.get("suboperation").and_then(Json::as_str) {
        Some(name) => round_named(name)?,
        None => SignatureRound::First,
    };
    let format = match json.get("format").and_then(Json::as_str) {
        Some(name) => format_named(name)?,
        None => {
            return Err(Refusal::about(
                Parameter::Format,
                "El lote no declaraba el formato de firma",
            ))
        }
    };
    let algorithm = match json.get("algorithm").and_then(Json::as_str) {
        Some(algorithm) => algorithm.to_owned(),
        None => {
            return Err(Refusal::about(
                Parameter::Algorithm,
                "El lote no declaraba el algoritmo de firma",
            ))
        }
    };
    let extra_params = match json.get("extraparams").and_then(Json::as_str) {
        Some(encoded) => expand_extra_params(encoded)?,
        None => Vec::new(),
    };

    let declared = json
        .get("singlesigns")
        .and_then(Json::as_array)
        .ok_or_else(|| {
            Refusal::about(
                Parameter::Data,
                "La declaracion del lote suministrada no es correcta: falta 'singlesigns'",
            )
        })?;

    let mut signs = Vec::with_capacity(declared.len());
    for single in declared {
        signs.push(single_sign(single, round, format, &extra_params)?);
    }

    Ok(LocalBatch {
        algorithm,
        stop_on_error,
        signs,
    })
}

fn single_sign(
    single: &Json,
    round: SignatureRound,
    format: Option<RequestedFormat>,
    extra_params: &[(String, String)],
) -> Result<LocalSingleSign, Refusal> {
    let id = single
        .get("id")
        .and_then(Json::as_str)
        .ok_or_else(|| {
            Refusal::about(
                Parameter::Data,
                "No se ha incluido el identificador de un documento del lote",
            )
        })?
        .to_owned();
    let reference = single
        .get("datareference")
        .and_then(Json::as_str)
        .ok_or_else(|| {
            Refusal::about(
                Parameter::Data,
                "No se ha incluido la referencia de un documento del lote",
            )
        })?;

    Ok(LocalSingleSign {
        id,
        document: decode_base64(reference)?,
        round: match single.get("suboperation").and_then(Json::as_str) {
            Some(name) => round_named(name)?,
            None => round,
        },
        format: match single.get("format").and_then(Json::as_str) {
            Some(name) => format_named(name)?,
            None => format,
        },
        extra_params: match single.get("extraparams").and_then(Json::as_str) {
            Some(encoded) => expand_extra_params(encoded)?,
            None => extra_params.to_vec(),
        },
    })
}

/// La ronda que nombra `suboperation`, sin `countersign`: rFirma solo firma y cofirma.
fn round_named(name: &str) -> Result<SignatureRound, Refusal> {
    match name.trim().to_ascii_lowercase().as_str() {
        SIGN => Ok(SignatureRound::First),
        COSIGN => Ok(SignatureRound::Again),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("la suboperacion '{other}' del lote no se atiende: solo 'sign' o 'cosign'"),
        )),
    }
}

/// El formato que nombra el lote, nada si pide `auto`, o el `SAF_06` que lo nombra.
fn format_named(name: &str) -> Result<Option<RequestedFormat>, Refusal> {
    if name.trim().eq_ignore_ascii_case(AUTO) {
        return Ok(None);
    }
    RequestedFormat::named(name).map(Some).ok_or_else(|| {
        Refusal::new(
            SafCode::UnsupportedFormat,
            format!("el formato '{name}' no se atiende: no lo firma AutoFirma en tres fases"),
        )
    })
}

/// Los `extraParams` del lote: Base64, y las `\n` escritas como texto son saltos de línea.
fn expand_extra_params(encoded: &str) -> Result<Vec<(String, String)>, Refusal> {
    let decoded = decode_base64(encoded)?;
    let text = String::from_utf8(decoded).map_err(|error| {
        Refusal::about(
            Parameter::Properties,
            format!("los 'extraparams' del lote no son texto: {error}"),
        )
    })?;

    Ok(without_the_launcher_keys(pairs_of(
        &text.trim().replace("\\n", "\n"),
    )))
}

/// El Base64 de dentro del lote, que es el del alfabeto normal y no el del protocolo.
fn decode_base64(encoded: &str) -> Result<Vec<u8>, Refusal> {
    let normalized: String = encoded
        .chars()
        .filter(|character| *character != '=')
        .map(|character| match character {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();

    STANDARD_NO_PAD
        .decode(normalized.as_bytes())
        .map_err(|error| {
            Refusal::about(
                Parameter::Data,
                format!("el lote trae Base64 invalido: {error}"),
            )
        })
}

#[cfg(test)]
mod tests;
