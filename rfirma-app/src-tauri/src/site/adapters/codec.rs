//! Códec del protocolo v4 para decodificar peticiones y codificar respuestas (ADR-0017).

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::site::domain::batch::parse_local_batch;
use crate::site::domain::protocol::{
    read_operation, AfirmaUrl, BatchRequest, Refusal, SiteOperation, WireAnswer,
    THIRD_PROTOCOL_VERSION,
};

use crate::site::adapters::data_download::HttpDataSource;
use crate::site::adapters::frontier;
use crate::site::application::errand::{LocalBatchAsk, ProtocolCodec, SiteOutcome, SiteRequest};

const RESULT_SEPARATOR: char = '|';

/// El texto exacto que espera `autoscript.js` cuando el guardado sale bien (`CommandProcessorThread.java:295,336`).
pub const SAVE_OK: &str = "SAVE_OK";

/// Códec de la versión 4 del protocolo de comunicación con la sede.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct V4Codec;

impl ProtocolCodec for V4Codec {
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest {
        request_of(read_operation(message, &HttpDataSource))
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        match outcome {
            SiteOutcome::Certificate(der) => on_the_wire(der),
            SiteOutcome::Signature {
                signer_der,
                signature,
                chosen_document,
            } => {
                let pair = format!(
                    "{}{RESULT_SEPARATOR}{}",
                    on_the_wire(signer_der),
                    on_the_wire(signature)
                );
                match chosen_document {
                    Some(name) => format!(
                        "{pair}{RESULT_SEPARATOR}{}",
                        on_the_wire(extra_data_of(name).as_bytes())
                    ),
                    None => pair,
                }
            }
            SiteOutcome::Saved => SAVE_OK.to_owned(),
            SiteOutcome::Loaded(files) => files
                .iter()
                .map(|(name, content)| format!("{name}:{}", STANDARD.encode(content)))
                .collect::<Vec<_>>()
                .join(&RESULT_SEPARATOR.to_string()),
            SiteOutcome::Cancelled => frontier::cancelled().on_the_wire(),
            SiteOutcome::Refused(refusal) => {
                WireAnswer::refused(frontier::code_of(refusal)).on_the_wire()
            }
            SiteOutcome::RefusedByTheProtocol(refusal) => refusal.answer().on_the_wire(),
            SiteOutcome::Batch { result, signer_der } => match signer_der {
                Some(signer_der) => format!(
                    "{}{RESULT_SEPARATOR}{}",
                    STANDARD.encode(result),
                    STANDARD.encode(signer_der)
                ),
                None => STANDARD.encode(result),
            },
        }
    }
}

/// La petición que el trámite atiende a partir de la operación leída.
pub(super) fn request_of(read: Result<SiteOperation, Refusal>) -> SiteRequest {
    match read {
        Ok(SiteOperation::SelectCertificate(request)) => SiteRequest::SelectCertificate(request),
        Ok(SiteOperation::Sign(request)) => SiteRequest::Sign(request),
        Ok(SiteOperation::SignWithoutDocument(request)) => {
            SiteRequest::SignWithoutDocument(request)
        }
        Ok(SiteOperation::Save(request)) => SiteRequest::Save(request),
        Ok(SiteOperation::Load(request)) => SiteRequest::Load(request),
        Ok(SiteOperation::SignAndSave(request)) => SiteRequest::SignAndSave(request),
        Ok(SiteOperation::Batch(request)) => batch_asked(request),
        Err(refusal) => SiteRequest::NotAttended(refusal),
    }
}

/// El lote local se lee aquí mismo, aunque su error no sale hasta firmarlo; el remoto viaja entero a los dos servlets.
fn batch_asked(request: BatchRequest) -> SiteRequest {
    if !request.is_local() {
        return SiteRequest::Batch(request);
    }
    let batch = parse_local_batch(request.lote());
    SiteRequest::LocalBatch(Box::new(LocalBatchAsk { request, batch }))
}

/// Si la versión negociada añade a la firma el tercer componente (`ProtocolInvocationLauncherSign.java:552-557`).
pub(super) fn carries_extra_data(version: i64) -> bool {
    version >= THIRD_PROTOCOL_VERSION
}

/// El JSON de `buildExtraDataResult` (`NativeSignDataProcessor.java:112-126`), con el valor escapado.
pub(super) fn extra_data_of(chosen_document: &str) -> String {
    format!(
        "{{\"filename\": {}}}",
        serde_json::Value::from(chosen_document)
    )
}

fn on_the_wire(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(bytes)
}

#[cfg(test)]
mod tests;
