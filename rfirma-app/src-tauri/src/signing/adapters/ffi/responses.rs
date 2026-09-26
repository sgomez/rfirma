//! Traduce las respuestas JSON que devuelve el puente a los tipos del dominio.

use base64::Engine;

use crate::identity::domain::holder::{common_name_of, holder_of, organization_identifier_of};
use crate::signing::domain::bridge::{
    BridgeError, DataRejection, PreSignBlock, PreSignature, PreviousSignature,
    PreviousSignaturesReport, SealedPreSignature, SignatureVerdict,
};
use crate::signing::domain::SessionSeal;

const PRESIGN_LIST_KEY: &str = "pres";
const UNREGISTERED_SIGNATURES_KIND: &str = "pdfHasUnregisteredSignatures";
const INCOMPATIBLE_POLICY_KIND: &str = "incompatiblePolicy";

/// Parsea la respuesta JSON de prefirma, venga como `pre` (PAdES) o como `pres` (CAdES).
pub fn parse_presign(json: &str) -> Result<PreSignature, BridgeError> {
    let response = parse_response(json)?;
    Ok(PreSignature {
        session: field(&response, "session")?.to_owned(),
        blocks: blocks_of(&response)?,
        stamp: SessionSeal::from_bridge(field(&response, "stamp")?),
    })
}

/// Los bloques a firmar: la lista identificada de CAdES, o el único `pre` de PAdES.
fn blocks_of(response: &serde_json::Value) -> Result<Vec<PreSignBlock>, BridgeError> {
    let Some(list) = response
        .get(PRESIGN_LIST_KEY)
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(vec![PreSignBlock {
            id: String::new(),
            pre: decoded_pre(field(response, "pre")?)?,
        }]);
    };
    if list.is_empty() {
        return Err(BridgeError::MalformedResponse(
            "la prefirma no trae ninguna entrada en \"pres\"".to_owned(),
        ));
    }
    list.iter()
        .map(|entry| {
            Ok(PreSignBlock {
                id: field(entry, "id")?.to_owned(),
                pre: decoded_pre(field(entry, "pre")?)?,
            })
        })
        .collect()
}

fn decoded_pre(encoded: &str) -> Result<Vec<u8>, BridgeError> {
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| BridgeError::MalformedResponse(format!("pre no es Base64: {error}")))
}

/// El PKCS#1 de una postfirma PAdES, que solo sabe ensamblar una firma.
pub(super) fn only_pkcs1(sealed: &SealedPreSignature) -> Result<&str, BridgeError> {
    match sealed.signed() {
        [only] => Ok(only.pkcs1_b64()),
        several => Err(BridgeError::MalformedResponse(format!(
            "la prefirma trae {} bloques a firmar y PAdES solo ensambla uno",
            several.len()
        ))),
    }
}

/// El PKCS#1 tal y como lo espera CAdES: la lista de firmas con su identificador.
pub(super) fn pkcs1_list(sealed: &SealedPreSignature) -> String {
    let entries: Vec<serde_json::Value> = sealed
        .signed()
        .iter()
        .map(|block| serde_json::json!({"id": block.id(), "pk1": block.pkcs1_b64()}))
        .collect();
    serde_json::Value::Array(entries).to_string()
}

/// Parsea la respuesta JSON de postfirma PAdES.
pub fn parse_postsign(json: &str) -> Result<Vec<u8>, BridgeError> {
    parse_signed_document(json, super::PADES_DOCUMENT_KEY)
}

pub(super) fn parse_signed_document(json: &str, key: &str) -> Result<Vec<u8>, BridgeError> {
    let response = parse_response(json)?;
    base64::engine::general_purpose::STANDARD
        .decode(field(&response, key)?)
        .map_err(|error| BridgeError::MalformedResponse(format!("{key} no es Base64: {error}")))
}

/// Parsea la respuesta JSON del filtrado de certificados.
pub fn parse_filter_selection(json: &str) -> Result<Vec<usize>, BridgeError> {
    let response = parse_response(json)?;
    let selected = response
        .get("selected")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| BridgeError::MalformedResponse("falta el campo \"selected\"".to_owned()))?;

    selected
        .iter()
        .map(|index| {
            index
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| {
                    BridgeError::MalformedResponse(format!("«{index}» no es un indice del listado"))
                })
        })
        .collect()
}

/// Parsea la respuesta JSON de expansión de parámetros.
pub fn parse_expanded_params(json: &str) -> Result<String, BridgeError> {
    let response = parse_response(json)?;
    Ok(field(&response, "params")?.to_owned())
}

/// Parsea el veredicto que devuelve la validación de firmas.
pub fn parse_verdict(json: &str) -> Result<SignatureVerdict, BridgeError> {
    let response = parse_response(json)?;
    match field(&response, "verdict")? {
        "valid" => Ok(SignatureVerdict::Valid),
        "unsigned" => Ok(SignatureVerdict::Unsigned),
        "invalid" => Ok(SignatureVerdict::Invalid {
            reason: field(&response, "reason")?.to_owned(),
        }),
        "confirmationNeeded" => Ok(SignatureVerdict::ConfirmationNeeded {
            parameter: field(&response, "param")?.to_owned(),
            message_code: field(&response, "messageCode")?.to_owned(),
        }),
        other => Err(BridgeError::MalformedResponse(format!(
            "veredicto desconocido «{other}»"
        ))),
    }
}

/// Parsea el informe de firmas previas, traduciendo cada firmante con las utilidades del titular.
pub fn parse_previous_signatures(json: &str) -> Result<PreviousSignaturesReport, BridgeError> {
    let response = parse_response(json)?;
    let entries = response
        .get("signatures")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            BridgeError::MalformedResponse("falta el campo \"signatures\"".to_owned())
        })?;

    let signatures = entries
        .iter()
        .map(previous_signature_of)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreviousSignaturesReport::new(signatures))
}

fn previous_signature_of(entry: &serde_json::Value) -> Result<PreviousSignature, BridgeError> {
    let subject = field(entry, "subject")?;
    let issuer = field(entry, "issuer")?;
    let (name, id_number) = holder_of(Some(subject));
    Ok(PreviousSignature {
        name,
        id_number,
        organization_identifier: organization_identifier_of(Some(subject)),
        issuer: common_name_of(Some(issuer)),
        certificate_serial_number: field(entry, "serialNumber")?.to_owned(),
        signing_time: entry
            .get("signingTime")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}

fn parse_response(json: &str) -> Result<serde_json::Value, BridgeError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| BridgeError::MalformedResponse(format!("{error}: {json}")))?;
    match value.get("ok").and_then(serde_json::Value::as_bool) {
        Some(true) => Ok(value),
        Some(false) => {
            let detail = value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("sin detalle")
                .to_owned();
            Err(
                match value.get("kind").and_then(serde_json::Value::as_str) {
                    Some(UNREGISTERED_SIGNATURES_KIND) => {
                        BridgeError::PdfHasUnregisteredSignatures(detail)
                    }
                    Some(INCOMPATIBLE_POLICY_KIND) => BridgeError::IncompatiblePolicy(detail),
                    Some(kind) => match data_rejection_of(kind) {
                        Some(rejection) => BridgeError::DataRejected(rejection, detail),
                        None => BridgeError::Failed(detail),
                    },
                    None => BridgeError::Failed(detail),
                },
            )
        }
        None => Err(BridgeError::MalformedResponse(format!(
            "no trae \"ok\": {json}"
        ))),
    }
}

fn data_rejection_of(kind: &str) -> Option<DataRejection> {
    Some(match kind {
        "invalidPdf" => DataRejection::InvalidPdf,
        "invalidXml" => DataRejection::InvalidXml,
        "invalidData" => DataRejection::InvalidData,
        "noSignData" => DataRejection::NoSignData,
        "facturaeAlreadySigned" => DataRejection::FacturaeAlreadySigned,
        "invalidFacturae" => DataRejection::InvalidFacturae,
        "signWithoutData" => DataRejection::SignWithoutData,
        "pdfPasswordNeeded" => DataRejection::PdfPasswordNeeded,
        _ => return None,
    })
}

fn field<'a>(response: &'a serde_json::Value, name: &str) -> Result<&'a str, BridgeError> {
    response
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| BridgeError::MalformedResponse(format!("falta el campo \"{name}\"")))
}
