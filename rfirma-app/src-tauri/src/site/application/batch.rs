//! El lote remoto ya consentido: prefirma, `PK1` con el token y postfirma, sin puente (`ProtocolInvocationLauncherBatch`, `BatchSigner`, 1.9.2).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use crate::identity::domain::certificate::TokenCertificate;
use crate::site::application::session::SiteRefusal;
use crate::site::domain::batch::{
    apply_pk1, build_empty_result, build_result, parse_json_presign, update_batch_with_errors,
    BatchDataResult, BatchFormat, TriphaseData,
};
use crate::site::domain::batch_error::{BatchError, Situation};
use crate::site::domain::protocol::BatchRequest;
use crate::site::domain::signing::SigningRefusal;
use crate::site::ports::{BatchServices, TokenSigning};

/// Cuántas firmas declara el lote, para decirlo en el momento de consentimiento.
pub fn how_many(request: &BatchRequest) -> usize {
    if request.is_json() {
        return single_signs_in_json(request.lote());
    }
    single_signs_in_xml(request.lote())
}

/// Lo que hace falta para firmar un lote ya consentido: los dos servlets, el token y el secreto abierto.
pub struct BatchRun<'a> {
    /// Los dos servlets del lote.
    pub services: &'a dyn BatchServices,
    /// Quien firma con el token, sin que la clave salga de él (ADR-0001).
    pub token: &'a dyn TokenSigning,
    /// El certificado que la persona consintió.
    pub certificate: &'a TokenCertificate,
    /// El secreto ya abierto, el mismo para todas las firmas.
    pub secret: &'a str,
}

/// Caso de uso: prefirma el lote, firma cada `PK1` con el token y postfirma; el resultado sale tal cual.
pub fn signed_batch(run: &BatchRun<'_>, request: &BatchRequest) -> Result<Vec<u8>, SiteRefusal> {
    let format = format_of(request);
    let certs = vec![run.certificate.der().to_vec()];

    let response = run
        .services
        .presign(
            request.presigner_url(),
            format,
            request.lote_base64(),
            &certs,
        )
        .map_err(SiteRefusal::Batch)?;

    let (triphase_data, errors) = presigned(format, &response)?;
    let lote_base64 = if errors.is_empty() {
        request.lote_base64().to_owned()
    } else {
        updated_batch(request, &errors)?
    };

    let Some(triphase_data) = triphase_data else {
        return Ok(match errors.is_empty() {
            true => build_empty_result(),
            false => build_result(&errors),
        });
    };

    let with_pk1 = every_pre_signed(run, request, triphase_data)?;

    run.services
        .postsign(
            request.postsigner_url(),
            format,
            &lote_base64,
            &certs,
            &with_pk1,
        )
        .map_err(SiteRefusal::Batch)
}

fn format_of(request: &BatchRequest) -> BatchFormat {
    match request.is_json() {
        true => BatchFormat::Json,
        false => BatchFormat::Xml,
    }
}

fn presigned(
    format: BatchFormat,
    response: &[u8],
) -> Result<(Option<TriphaseData>, Vec<BatchDataResult>), SiteRefusal> {
    match format {
        BatchFormat::Json => {
            let outcome = parse_json_presign(response).map_err(invalid_presign)?;
            Ok((outcome.triphase_data().cloned(), outcome.errors().to_vec()))
        }
        BatchFormat::Xml => {
            let data = TriphaseData::parse_xml(response).map_err(invalid_presign)?;
            let signed_something = !data.signs().is_empty();
            Ok((signed_something.then_some(data), Vec::new()))
        }
    }
}

fn every_pre_signed(
    run: &BatchRun<'_>,
    request: &BatchRequest,
    triphase_data: TriphaseData,
) -> Result<TriphaseData, SiteRefusal> {
    let mut refused: Option<SigningRefusal> = None;
    let with_pk1 = apply_pk1(triphase_data, |pre| {
        match run
            .token
            .sign(run.certificate, run.secret, request.algorithm(), pre)
        {
            Ok(pk1) => pk1,
            Err(refusal) => {
                refused.get_or_insert(refusal);
                Vec::new()
            }
        }
    })
    .map_err(invalid_presign)?;

    match refused {
        Some(refusal) => Err(SiteRefusal::BatchSigningFailed(refusal)),
        None => Ok(with_pk1),
    }
}

fn updated_batch(
    request: &BatchRequest,
    errors: &[BatchDataResult],
) -> Result<String, SiteRefusal> {
    if !request.is_json() {
        return Ok(request.lote_base64().to_owned());
    }
    let updated = update_batch_with_errors(request.lote(), errors).map_err(invalid_presign)?;
    Ok(URL_SAFE_NO_PAD.encode(updated))
}

fn invalid_presign(detail: impl std::fmt::Display) -> SiteRefusal {
    SiteRefusal::Batch(BatchError::new(
        Situation::InvalidPresignResponse,
        detail.to_string(),
    ))
}

fn single_signs_in_json(lote: &[u8]) -> usize {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(lote) else {
        return 0;
    };
    value
        .get("singlesigns")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

fn single_signs_in_xml(lote: &[u8]) -> usize {
    const SINGLE_SIGN: &[u8] = b"singlesign";

    let Ok(text) = std::str::from_utf8(lote) else {
        return 0;
    };
    let mut reader = quick_xml::Reader::from_str(text);
    let mut found = 0;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(tag) | quick_xml::events::Event::Empty(tag)) => {
                if tag.name().local_name().as_ref() == SINGLE_SIGN {
                    found += 1;
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => return found,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
