//! Pruebas de la firma suelta de sede que pide SHA-1, con la preferencia «Permitir SHA-1» apagada y activada.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::session;
use crate::signing::application::tests::{a_memory, A_CADES_SIGNATURE};
use crate::site::application::errand::*;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelMessage, NegotiatedCredential, Refusal, RefusalSituation, SafCode,
};
use base64::Engine as _;

const AN_INVOICE: &[u8] = b"<Facturae><FileHeader/><Parties/><Invoices/></Facturae>";

fn a_request(query: &str, document: Option<&[u8]>, properties: &str) -> AfirmaUrl {
    let base64 = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE.encode(bytes);
    let dat = document.map_or(String::new(), |bytes| format!("&dat={}", base64(bytes)));
    let text = format!(
        "afirma://sign?{query}&idsession={CREDENTIAL}&properties={}{dat}",
        base64(properties.as_bytes())
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Lo que pasa con la petición: el paso del trámite y, si se consintió, los algoritmos del token.
struct Attended {
    step: ErrandStep,
    refused_at_consent: Option<SiteRefusal>,
    signed_with: Vec<SignatureAlgorithm>,
    sha1_still_allowed_once: bool,
}

/// Atiende `url` con la preferencia dada y, si llega al consentimiento, consiente y firma en el token.
fn attended_with_the_preference(url: AfirmaUrl, allowed: bool) -> Attended {
    attended(url, allowed, false)
}

/// Como `attended_with_the_preference`, y con SHA-1 permitido solo esta vez si `allowed_once`.
fn attended(url: AfirmaUrl, allowed: bool, allowed_once: bool) -> Attended {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_configuration(&Configuration {
            allow_sha1: allowed,
            ..Configuration::default()
        })
        .expect("la memoria de pruebas escribe");
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut neighbours = a_neighbourhood(home.path(), &listed, &opened, &memory);
    neighbours.ours = ours;
    neighbours.bridge = TheBridge::answering();
    let desk = a_desk(&engine, &policies, &neighbours, &scratch);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, _wire) = the_wire();

    let step = attend(&desk, url, handle, &live).expect("hay codec negociado");
    if allowed_once {
        live.allow_sha1_once();
    }
    let mut refused_at_consent = None;
    let sha1_still_allowed_once = if let ErrandStep::AskingToSign(asking) = &step {
        let chosen = asking.certificates[0].id.clone();
        match consent(&desk, &chosen, &live) {
            Ok(Consented::SigningWith(_)) => {
                session::sign_on_token(
                    &neighbours.signer,
                    &neighbours.session,
                    &the_typed_secret(),
                )
                .expect("el token de pruebas firma el PRE");
            }
            Ok(_) => panic!("una firma se consiente firmando"),
            Err(ConsentError::Refused(refusal)) => refused_at_consent = Some(refusal),
            Err(other) => panic!("el certificado vale: {other:?}"),
        }
        live.end();
        live.sha1_allowed_once()
    } else {
        false
    };
    Attended {
        step,
        refused_at_consent,
        signed_with: neighbours.signer.signed_with(),
        sha1_still_allowed_once,
    }
}

fn the_refusal_shown(step: &ErrandStep, what: &str) -> Refusal {
    let ErrandStep::ShowingTheRefusal(refusal) = step else {
        panic!("{what}: el rechazo se enseña antes de contestar: {step:?}");
    };
    refusal.clone()
}

fn every_signature_with_sha1() -> Vec<(AfirmaUrl, &'static str)> {
    vec![
        (
            a_request(
                "op=sign&format=PAdES&algorithm=SHA1withRSA",
                Some(A_PDF),
                "",
            ),
            "sign en PAdES",
        ),
        (
            a_request(
                "op=sign&format=XAdES&algorithm=SHA-1",
                Some(AN_XML_CHALLENGE),
                "",
            ),
            "sign en XAdES",
        ),
        (
            a_request(
                "op=sign&format=CAdES&algorithm=SHA1withECDSA",
                Some(A_PDF),
                "",
            ),
            "sign en CAdES",
        ),
        (
            a_request(
                "op=sign&format=FacturaE&algorithm=SHA",
                Some(AN_INVOICE),
                "",
            ),
            "sign en FacturaE",
        ),
        (
            a_request("op=cosign&format=PAdES&algorithm=SHA1", Some(A_PDF), ""),
            "cosign",
        ),
        (
            a_request(
                "op=countersign&format=CAdES&algorithm=1.3.14.3.2.26",
                Some(A_CADES_SIGNATURE),
                "",
            ),
            "countersign",
        ),
        (
            a_request(
                "op=signandsave&cop=sign&format=PAdES&algorithm=SHA1withRSA",
                Some(A_PDF),
                "",
            ),
            "signandsave",
        ),
    ]
}

#[test]
fn without_the_preference_every_signature_with_sha1_asks_to_allow_it_and_never_consents_alone() {
    for (url, what) in every_signature_with_sha1() {
        let attended = attended_with_the_preference(url, false);

        let ErrandStep::AskingToSign(asking) = &attended.step else {
            panic!("{what}: llega al consentimiento: {:?}", attended.step);
        };
        assert!(asking.sha1_to_allow, "{what}: pide permitirlo");
        assert!(!asking.without_asking, "{what}: no consiente solo");
        assert!(matches!(
            asking.consenting(),
            Moment::AskingToSign {
                sha1_allowed: false,
                sha1_to_allow: true,
                ..
            }
        ));
    }
}

#[test]
fn without_the_preference_nor_the_permission_consenting_is_refused_as_saf_03_and_the_token_signs_nothing(
) {
    for (url, what) in every_signature_with_sha1() {
        let attended = attended_with_the_preference(url, false);

        assert!(
            matches!(
                attended.refused_at_consent,
                Some(SiteRefusal::Sha1NotAllowed(_))
            ),
            "{what}: el consentimiento se rechaza"
        );
        assert!(attended.signed_with.is_empty(), "{what}: el token no firma");
    }
}

#[test]
fn allowed_once_every_signature_with_sha1_is_signed_with_it_without_the_preference() {
    for (url, what) in [
        (
            a_request(
                "op=sign&format=PAdES&algorithm=SHA1withRSA",
                Some(A_PDF),
                "",
            ),
            "sign en PAdES",
        ),
        (
            a_request(
                "op=sign&format=XAdES&algorithm=SHA-1",
                Some(AN_XML_CHALLENGE),
                "",
            ),
            "sign en XAdES",
        ),
        (
            a_request("op=sign&format=CAdES&algorithm=SHA1", Some(A_PDF), ""),
            "sign en CAdES",
        ),
        (
            a_request(
                "op=sign&format=FacturaE&algorithm=SHA1",
                Some(AN_INVOICE),
                "",
            ),
            "sign en FacturaE",
        ),
    ] {
        let attended = attended(url, false, true);

        assert!(attended.refused_at_consent.is_none(), "{what}");
        assert_eq!(
            attended.signed_with,
            vec![SignatureAlgorithm::Sha1Ecdsa],
            "{what}: se firma con SHA-1"
        );
        assert!(
            !attended.sha1_still_allowed_once,
            "{what}: se olvida al acabar"
        );
    }
}

#[test]
fn with_the_preference_a_signature_with_sha1_does_not_ask_to_allow_it_and_keeps_the_mark() {
    let url = a_request("op=sign&format=CAdES&algorithm=SHA1", Some(A_PDF), "");

    let attended = attended_with_the_preference(url, true);

    let ErrandStep::AskingToSign(asking) = &attended.step else {
        panic!("llega al consentimiento: {:?}", attended.step);
    };
    assert!(!asking.sha1_to_allow);
    assert!(matches!(
        asking.consenting(),
        Moment::AskingToSign {
            sha1_allowed: true,
            sha1_to_allow: false,
            ..
        }
    ));
}

#[test]
fn with_the_preference_a_signature_outside_xml_is_signed_with_sha1_on_the_token() {
    for (url, what) in [
        (
            a_request(
                "op=sign&format=CAdES&algorithm=SHA1withRSA",
                Some(A_PDF),
                "",
            ),
            "sign en CAdES",
        ),
        (
            a_request("op=sign&format=PAdES&algorithm=SHA1", Some(A_PDF), ""),
            "sign en PAdES",
        ),
        (
            a_request("op=sign&format=CMS/PKCS#7&algorithm=SHA-1", Some(A_PDF), ""),
            "sign en CMS",
        ),
        (
            a_request("op=sign&format=NONE&algorithm=SHA1", Some(A_PDF), ""),
            "sign en PKCS#1",
        ),
        (
            a_request(
                "op=cosign&format=CAdES&algorithm=SHA1withECDSA",
                Some(A_CADES_SIGNATURE),
                "",
            ),
            "cosign en CAdES",
        ),
        (
            a_request(
                "op=countersign&format=CAdES&algorithm=1.3.14.3.2.26",
                Some(A_CADES_SIGNATURE),
                "",
            ),
            "countersign en CAdES",
        ),
        (
            a_request(
                "op=signandsave&cop=sign&format=PAdES&algorithm=SHA1withRSA",
                Some(A_PDF),
                "",
            ),
            "signandsave en PAdES",
        ),
    ] {
        let attended = attended_with_the_preference(url, true);

        assert!(
            matches!(&attended.step, ErrandStep::AskingToSign(_)),
            "{what}: llega al consentimiento: {:?}",
            attended.step
        );
        assert_eq!(
            attended.signed_with,
            vec![SignatureAlgorithm::Sha1Ecdsa],
            "{what}: el certificado de pruebas lleva clave EC y firma con SHA-1, nunca con SHA-256"
        );
    }
}

#[test]
fn with_the_preference_the_consent_carries_the_sha1_mark() {
    let url = a_request("op=sign&format=CAdES&algorithm=SHA1", Some(A_PDF), "");

    let attended = attended_with_the_preference(url, true);

    let ErrandStep::AskingToSign(asking) = &attended.step else {
        panic!("llega al consentimiento: {:?}", attended.step);
    };
    assert!(matches!(
        asking.consenting(),
        Moment::AskingToSign {
            sha1_allowed: true,
            ..
        }
    ));
}

#[test]
fn a_consent_with_sha2_carries_no_sha1_mark() {
    let url = a_request("op=sign&format=CAdES&algorithm=SHA256", Some(A_PDF), "");

    let attended = attended_with_the_preference(url, true);

    let ErrandStep::AskingToSign(asking) = &attended.step else {
        panic!("llega al consentimiento: {:?}", attended.step);
    };
    assert!(matches!(
        asking.consenting(),
        Moment::AskingToSign {
            sha1_allowed: false,
            ..
        }
    ));
}

#[test]
fn with_the_preference_xades_and_facturae_are_signed_with_sha1_on_the_token() {
    for (url, what) in [
        (
            a_request(
                "op=sign&format=XAdES&algorithm=SHA1",
                Some(AN_XML_CHALLENGE),
                "",
            ),
            "sign en XAdES",
        ),
        (
            a_request(
                "op=sign&format=FacturaE&algorithm=SHA1withRSA",
                Some(AN_INVOICE),
                "",
            ),
            "sign en FacturaE",
        ),
        (
            a_request(
                "op=sign&format=auto&algorithm=SHA1",
                Some(AN_XML_CHALLENGE),
                "",
            ),
            "un XML con format=auto",
        ),
    ] {
        let attended = attended_with_the_preference(url, true);

        assert!(
            matches!(&attended.step, ErrandStep::AskingToSign(_)),
            "{what}: llega al consentimiento: {:?}",
            attended.step
        );
        assert_eq!(
            attended.signed_with,
            vec![SignatureAlgorithm::Sha1Ecdsa],
            "{what}: el certificado de pruebas lleva clave EC y firma con SHA-1, nunca con SHA-256"
        );
    }
}

#[test]
fn with_the_preference_the_explicit_xades_is_still_refused() {
    let url = a_request(
        "op=sign&format=XAdES&algorithm=SHA1",
        Some(AN_XML_CHALLENGE),
        "mode=explicit\n",
    );

    let attended = attended_with_the_preference(url, true);

    let refusal = the_refusal_shown(&attended.step, "XAdES explícita");
    assert_eq!(refusal.situation(), RefusalSituation::ExplicitXades);
    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}
