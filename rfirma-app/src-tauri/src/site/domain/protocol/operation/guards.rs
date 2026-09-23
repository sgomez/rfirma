//! Las guardias de formato y algoritmo que comparten `sign` y `signandsave`.

use super::super::algorithm::AskedAlgorithm;
use super::super::codes::{Parameter, SafCode};
use super::super::detection::{detect_signature, DetectedSignature};
use super::super::format::{format_of, RequestedFormat, XadesEnvelope};
use super::super::refusal::{Refusal, RefusalSituation};
use super::super::url::AfirmaUrl;
use super::properties::required;
use super::sign::SignatureRound;
use super::AUTO;

/// Resuelve el formato efectivo con `format=auto`, exigiendo firma previa si la ronda es multifirma.
pub(super) fn resolve_auto_format(
    document: &[u8],
    round: SignatureRound,
) -> Result<RequestedFormat, Refusal> {
    if matches!(round, SignatureRound::First) {
        return Ok(format_of(document));
    }
    match detect_signature(document) {
        Some(DetectedSignature::Pdf) => Ok(RequestedFormat::Pades),
        Some(DetectedSignature::Invoice) => Ok(RequestedFormat::FacturaE),
        Some(DetectedSignature::Xml) => Ok(RequestedFormat::Xades(XadesEnvelope::Enveloping)),
        Some(DetectedSignature::Cms) => Ok(RequestedFormat::Cades),
        None => Err(Refusal::new(
            SafCode::UnknownSigner,
            "el formato de firma no se ha podido determinar a partir de los datos aportados",
        )),
    }
}

/// Una factura ni se cofirma ni se contrafirma: `AOFacturaESigner` lanza una
/// `UnsupportedOperationException` en las dos, y aquí es un `SAF_04`.
pub fn refuse_a_multisignature_of_an_invoice(
    round: SignatureRound,
    format: RequestedFormat,
) -> Result<(), Refusal> {
    let first = matches!(round, SignatureRound::First);
    if !first && matches!(format, RequestedFormat::FacturaE) {
        return Err(Refusal::new(
            SafCode::UnsupportedOperation,
            "una factura ni se cofirma ni se contrafirma: AOFacturaESigner lanza una \
             UnsupportedOperationException en las dos",
        )
        .found_while_processing());
    }
    Ok(())
}

/// CAdES y XAdES contrafirman: el resto sale con el rechazo del original.
pub fn refuse_a_countersignature_outside_cades_and_xades(
    round: SignatureRound,
    format: RequestedFormat,
) -> Result<(), Refusal> {
    let counters = matches!(round, SignatureRound::Counter { .. });
    let supported = matches!(
        format,
        RequestedFormat::Cades | RequestedFormat::Cms | RequestedFormat::Xades(_)
    );
    if counters && !supported {
        return Err(countersign_refusal());
    }
    Ok(())
}

/// `mode=explicit` con XAdES: `SAF_06`, la desviación que documenta `domain/protocol/mod.rs`.
pub fn refuse_explicit_xades(
    format: RequestedFormat,
    declared_params: &[(String, String)],
) -> Result<(), Refusal> {
    let explicit = declared_params.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("mode") && value.eq_ignore_ascii_case("explicit")
    });
    if explicit && matches!(format, RequestedFormat::Xades(_)) {
        return Err(Refusal::new(
            SafCode::UnsupportedFormat,
            "'mode=explicit' con XAdES no se reproduce: ver domain/protocol/mod.rs",
        ));
    }
    Ok(())
}

/// El formato que nombra la sede, nada si pide `auto`, o el `SAF_06` que nombra
/// el que el original no firma en tres fases.
pub(super) fn requested_format(url: &AfirmaUrl) -> Result<Option<RequestedFormat>, Refusal> {
    let format = required(url, "format", Parameter::Format)
        .map_err(|refusal| refusal.because(RefusalSituation::MissingFormat))?;

    if format.trim().eq_ignore_ascii_case(AUTO) {
        return Ok(None);
    }
    RequestedFormat::named(format).map(Some).ok_or_else(|| {
        Refusal::new(
            SafCode::UnsupportedFormat,
            format!("el formato '{format}' no se atiende: no lo firma AutoFirma en tres fases"),
        )
    })
}

/// La huella del `algorithm` que pide la sede, o el `SAF_03` que lo nombra.
pub(super) fn check_algorithm(url: &AfirmaUrl) -> Result<AskedAlgorithm, Refusal> {
    let algorithm = required(url, "algorithm", Parameter::Algorithm)?;
    AskedAlgorithm::named(algorithm).ok_or_else(|| {
        Refusal::about(
            Parameter::Algorithm,
            format!("el algoritmo '{algorithm}' no se atiende: rFirma firma con SHA-2"),
        )
    })
}

/// `'countersign' solo existe en CAdES y XAdES`, compartido por `read_operation`
/// y por el `cop` de `signandsave`.
fn countersign_refusal() -> Refusal {
    Refusal::new(
        SafCode::UnsupportedOperation,
        "'countersign' no existe fuera de CAdES y XAdES: AOPDFSigner.countersign lanza una \
         UnsupportedOperationException",
    )
    .found_while_processing()
}
