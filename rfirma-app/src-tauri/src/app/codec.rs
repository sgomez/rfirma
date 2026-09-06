//! Códec del protocolo v4 para decodificar peticiones y codificar respuestas (ADR-0017).

use base64::Engine as _;

use crate::protocol::{read_operation, AfirmaUrl, SiteOperation};

use super::errand::{ProtocolCodec, SiteOutcome, SiteRequest};
use super::frontier;

const RESULT_SEPARATOR: char = '|';

/// Códec de la versión 4 del protocolo de comunicación con la sede.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct V4Codec;

impl ProtocolCodec for V4Codec {
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest {
        match read_operation(message) {
            Ok(SiteOperation::SelectCertificate(request)) => {
                SiteRequest::SelectCertificate(request)
            }
            Ok(SiteOperation::Sign(request)) => SiteRequest::Sign(request),
            Err(refusal) => SiteRequest::NotAttended(refusal),
        }
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        match outcome {
            SiteOutcome::Certificate(der) => on_the_wire(der),
            SiteOutcome::Signature { signer_der, signed } => {
                format!(
                    "{}{RESULT_SEPARATOR}{}",
                    on_the_wire(signer_der),
                    on_the_wire(signed)
                )
            }
            SiteOutcome::Cancelled => frontier::cancelled().on_the_wire(),
            SiteOutcome::Refused { answer, .. } => answer.on_the_wire(),
            SiteOutcome::RefusedByTheProtocol(refusal) => refusal.answer().on_the_wire(),
        }
    }
}

fn on_the_wire(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Failure;
    use crate::protocol::{ChannelMessage, SafCode, WireAnswer};

    const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

    fn an_operation(text: &str) -> AfirmaUrl {
        let ChannelMessage::Operation { url } = ChannelMessage::read(text) else {
            panic!("una URL del protocolo es una operacion");
        };
        url
    }

    #[test]
    fn the_selection_the_published_client_sends_is_what_the_site_wants() {
        let request = V4Codec.decode(&an_operation(&format!(
            "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
        )));
        assert!(matches!(request, SiteRequest::SelectCertificate(_)));
    }

    #[test]
    fn an_operation_that_is_not_attended_is_a_request_with_its_refusal() {
        let request = V4Codec.decode(&an_operation(&format!(
            "afirma://countersign?op=countersign&idsession={CREDENTIAL}"
        )));
        let SiteRequest::NotAttended(refusal) = request else {
            panic!("la contrafirma no se atiende: {request:?}");
        };
        assert!(refusal.answer().on_the_wire().starts_with("SAF_04"));
    }

    #[test]
    fn a_certificate_goes_out_as_url_safe_base64_and_nothing_else() {
        assert_eq!(
            V4Codec.encode(&SiteOutcome::Certificate(vec![0xfb, 0xff, 0xbf])),
            "-_-_"
        );
    }

    #[test]
    fn a_signature_goes_out_behind_its_certificate_separated_by_a_bar() {
        assert_eq!(
            V4Codec.encode(&SiteOutcome::Signature {
                signer_der: vec![0xfb, 0xff, 0xbf],
                signed: b"%PDF".to_vec(),
            }),
            "-_-_|JVBERg=="
        );
    }

    #[test]
    fn the_cancellation_and_the_refusals_go_out_as_the_catalogue_writes_them() {
        assert_eq!(V4Codec.encode(&SiteOutcome::Cancelled), "CANCEL");
        let refused = V4Codec.encode(&SiteOutcome::Refused {
            answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
            failure: Failure::new("certificateNotFound", "detalle que no sale"),
        });
        assert!(refused.starts_with("SAF_19"), "{refused}");
        assert!(
            !refused.contains("detalle que no sale"),
            "el detalle no sale"
        );
    }
}
