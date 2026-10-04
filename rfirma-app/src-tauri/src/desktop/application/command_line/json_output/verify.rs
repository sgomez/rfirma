//! Lo que saca `verify --json`, construido desde el modelo de firmas previas de ADR-0043 y no desde las líneas de texto del original.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use super::{in_hexadecimal, CertificateOutput};
use crate::signing::domain::{
    DocumentFinding, DocumentSignature, DocumentSignatures, SignatureStandard, SigningDate,
    Validity, ValidityReason,
};

/// Lo que saca `verify --json`.
#[derive(Serialize)]
pub(in super::super) struct VerifiedDocument {
    standard: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    validity: Option<&'static str>,
    findings: Vec<&'static str>,
    signatures: Vec<VerifiedSignature>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedSignature {
    validity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<ReasonOutput>,
    signer: SignerOutput,
    certificate: CertificateOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature_algorithm: Option<AlgorithmOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signing_time: Option<SigningTimeOutput>,
    closes_document: bool,
    countersignatures: Vec<VerifiedSignature>,
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ReasonOutput {
    CertificateExpired {
        #[serde(skip_serializing_if = "Option::is_none")]
        date: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        holder: Option<String>,
    },
    CertificateNotYetValid {
        #[serde(skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    ModifiedAfterSigning,
    Damaged,
    UnknownSignatureType,
    UnsupportedAlgorithm,
    CosignNotAdmitted {
        #[serde(skip_serializing_if = "Option::is_none")]
        closed_by: Option<String>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignerOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    common_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization_name: Option<String>,
}

#[derive(Serialize)]
struct AlgorithmOutput {
    name: String,
}

#[derive(Serialize)]
struct SigningTimeOutput {
    at: String,
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    tsa: Option<String>,
}

impl VerifiedDocument {
    /// El documento con las firmas que se leyeron de él.
    pub(in super::super) fn of(signatures: &DocumentSignatures) -> Self {
        let worst = signatures
            .signatures()
            .iter()
            .flat_map(with_its_countersignatures)
            .map(|signature| signature.validity)
            .max_by_key(|validity| gravity_of(*validity));
        Self {
            standard: standard_name(signatures.format()),
            validity: worst.map(validity_name),
            findings: signatures
                .findings()
                .iter()
                .map(|finding| finding_name(*finding))
                .collect(),
            signatures: signatures
                .signatures()
                .iter()
                .map(VerifiedSignature::of)
                .collect(),
        }
    }

    /// Unos datos que no son de ningún formato de firma.
    pub(in super::super) fn unrecognized() -> Self {
        Self::of(&DocumentSignatures::default().in_format(SignatureStandard::Unrecognized))
    }
}

impl VerifiedSignature {
    fn of(signature: &DocumentSignature) -> Self {
        Self {
            validity: validity_name(signature.validity),
            reason: signature.validity_reason.as_ref().map(ReasonOutput::of),
            signer: SignerOutput {
                common_name: non_empty(&signature.name),
                serial_number: non_empty(&signature.id_number),
                organization_identifier: signature.organization_identifier.clone(),
                organization_name: signature.organization_name.clone(),
            },
            certificate: CertificateOutput {
                subject: non_empty(&signature.certificate_subject),
                issuer: non_empty(&signature.certificate_issuer),
                serial_number: in_hexadecimal(&signature.certificate_serial_number),
                not_before: signature.certificate_valid_from.as_deref().and_then(in_utc),
                not_after: signature
                    .certificate_valid_until
                    .as_deref()
                    .and_then(in_utc),
            },
            signature_algorithm: signature
                .signature_algorithm
                .clone()
                .map(|name| AlgorithmOutput { name }),
            signing_time: SigningTimeOutput::of(signature),
            closes_document: signature.closes_document,
            countersignatures: signature.countersignatures.iter().map(Self::of).collect(),
        }
    }
}

impl ReasonOutput {
    fn of(reason: &ValidityReason) -> Self {
        match reason {
            ValidityReason::CertificateExpired { date, holder } => Self::CertificateExpired {
                date: in_utc(date),
                holder: holder.clone(),
            },
            ValidityReason::CertificateNotYetValid { date } => {
                Self::CertificateNotYetValid { date: in_utc(date) }
            }
            ValidityReason::ModifiedAfterSigning => Self::ModifiedAfterSigning,
            ValidityReason::Damaged => Self::Damaged,
            ValidityReason::UnknownSignatureType => Self::UnknownSignatureType,
            ValidityReason::UnsupportedAlgorithm => Self::UnsupportedAlgorithm,
            ValidityReason::CosignNotAdmitted { closed_by } => Self::CosignNotAdmitted {
                closed_by: closed_by.clone(),
            },
        }
    }
}

impl SigningTimeOutput {
    fn of(signature: &DocumentSignature) -> Option<Self> {
        let (at, source, tsa) = match &signature.signing_date {
            Some(SigningDate::Stamped { at, tsa }) => (at.as_str(), "timestamp", non_empty(tsa)),
            Some(SigningDate::Declared { at }) => (at.as_str(), "declared", None),
            None => (signature.signing_time.as_deref()?, "declared", None),
        };
        Some(Self {
            at: in_utc(at)?,
            source,
            tsa,
        })
    }
}

fn with_its_countersignatures(signature: &DocumentSignature) -> Vec<&DocumentSignature> {
    std::iter::once(signature)
        .chain(
            signature
                .countersignatures
                .iter()
                .flat_map(with_its_countersignatures),
        )
        .collect()
}

fn gravity_of(validity: Validity) -> u8 {
    match validity {
        Validity::Valid => 0,
        Validity::Expired => 1,
        Validity::Invalid => 2,
    }
}

fn validity_name(validity: Validity) -> &'static str {
    match validity {
        Validity::Valid => "valid",
        Validity::Expired => "expired",
        Validity::Invalid => "invalid",
    }
}

fn standard_name(standard: SignatureStandard) -> &'static str {
    match standard {
        SignatureStandard::Pades => "PAdES",
        SignatureStandard::Cades => "CAdES",
        SignatureStandard::Xades => "XAdES",
        SignatureStandard::Unrecognized => "unrecognized",
    }
}

fn finding_name(finding: DocumentFinding) -> &'static str {
    match finding {
        DocumentFinding::ModifiedAfterLastSignature => "modifiedAfterLastSignature",
        DocumentFinding::FormFilledAfterSigning => "formFilledAfterSigning",
        DocumentFinding::ContentAddedOnTop => "contentAddedOnTop",
    }
}

fn non_empty(text: &str) -> Option<String> {
    Some(text.to_owned()).filter(|text| !text.is_empty())
}

fn in_utc(instant: &str) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(instant).ok()?;
    Some(
        parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    )
}
