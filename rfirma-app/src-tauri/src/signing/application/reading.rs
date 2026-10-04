//! Caso de uso de leer las firmas que ya trae un documento: detecta el formato, aplica la admisibilidad de lectura, pide las firmas al motor y marca si la firma local lo rechaza.

use base64::Engine;

use crate::signing::application::cycle::CycleError;
use crate::signing::domain::{
    AdmissibleDocument, DocumentSignatures, Format, Refusal, SignatureStandard, Waivers,
};
use crate::signing::ports::PreviousSignaturesEngine;

/// Las firmas del documento en su formato; un PDF cifrado se rechaza y un fichero no reconocido no llega al motor.
pub fn signatures_of(
    bytes: &[u8],
    engine: &dyn PreviousSignaturesEngine,
) -> Result<DocumentSignatures, CycleError> {
    let format = standard_of(bytes);
    match format {
        SignatureStandard::Unrecognized => {
            return Ok(DocumentSignatures::default().in_format(format))
        }
        SignatureStandard::Pades => {
            AdmissibleDocument::check_for(Format::Pades, bytes, Waivers::READING)
                .map_err(CycleError::from)?;
        }
        SignatureStandard::Cades | SignatureStandard::Xades => {}
    }
    let document_b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(engine
        .previous_signatures(&document_b64)?
        .in_format(format)
        .closed_to_signing(is_closed_to_signing(format, bytes)))
}

fn is_closed_to_signing(format: SignatureStandard, bytes: &[u8]) -> bool {
    format == SignatureStandard::Pades
        && matches!(
            AdmissibleDocument::check_for(Format::Pades, bytes, Waivers::NONE),
            Err(Refusal::Certified)
        )
}

fn standard_of(bytes: &[u8]) -> SignatureStandard {
    use crate::site::domain::protocol::detection::{is_cms_signed_data, shape_of, DetectedShape};
    match shape_of(bytes) {
        DetectedShape::Pdf => SignatureStandard::Pades,
        DetectedShape::Invoice | DetectedShape::Xml => SignatureStandard::Xades,
        DetectedShape::Binary if is_cms_signed_data(bytes) => SignatureStandard::Cades,
        DetectedShape::Binary => SignatureStandard::Unrecognized,
    }
}

#[cfg(test)]
mod tests;
