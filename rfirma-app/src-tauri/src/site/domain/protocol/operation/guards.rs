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
use crate::site::domain::triphase_server::ServerFormat;

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
            "FacturaE no admite cofirma ni contrafirma",
        )
        .because(RefusalSituation::InvoiceMultisignature));
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

/// `SAF_06` donde AutoFirma firmaría la huella SHA-1 en vez del documento.
pub fn refuse_explicit_xades(
    round: SignatureRound,
    format: RequestedFormat,
    through_the_site_server: Option<ServerFormat>,
    declared_params: &[(String, String)],
) -> Result<(), Refusal> {
    let signs_the_digest = matches!(round, SignatureRound::First)
        && matches!(format, RequestedFormat::Xades(_))
        && through_the_site_server != Some(ServerFormat::Xades)
        && declares(declared_params, "mode", "explicit")
        && !declares(declared_params, "useManifest", "true");
    if signs_the_digest {
        return Err(Refusal::new(
            SafCode::UnsupportedFormat,
            "mode=explicit con XAdES (firma de la huella SHA-1)",
        )
        .because(RefusalSituation::ExplicitXades));
    }
    Ok(())
}

fn declares(declared_params: &[(String, String)], key: &str, value: &str) -> bool {
    declared_params.iter().any(|(declared, its)| {
        declared.eq_ignore_ascii_case(key) && its.eq_ignore_ascii_case(value)
    })
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

fn countersign_refusal() -> Refusal {
    Refusal::new(
        SafCode::UnsupportedOperation,
        "contrafirma fuera de CAdES, CMS y XAdES",
    )
    .because(RefusalSituation::UnsupportedCountersignature)
}
