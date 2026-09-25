//! Pruebas de la firma contra el servidor trifásico de la sede, con el servidor y el token doblados.

use super::*;

use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine as _;

use crate::identity::application::tests::a_usable_certificate;
use crate::identity::domain::error::Situation as TokenSituation;
use crate::site::application::tests::{InMemoryTokenSigning, InMemoryTriphaseServer};
use crate::site::domain::protocol::{CounterTarget, SafCode};

const SERVER_URL: &str = "https://sede.example/tri";

fn the_value_of<'a>(form: &'a [(&'static str, String)], name: &str) -> Option<&'a str> {
    form.iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.as_str())
}

fn a_presignature_asking_to_sign(pre: &[u8]) -> Vec<u8> {
    let xml = format!(
        "<xml>\n <firmas format=\"CAdES\">\n  <firma Id=\"1\">\n   <param n=\"PRE\">{}</param>\n  </firma>\n </firmas>\n</xml>",
        STANDARD.encode(pre)
    );
    URL_SAFE.encode(xml).into_bytes()
}

fn a_postsign_handing_back(signature: &[u8]) -> Vec<u8> {
    format!("OK NEWID={}", URL_SAFE.encode(signature)).into_bytes()
}

fn the_site_declaring(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn signed_with(
    server: &InMemoryTriphaseServer,
    token: &InMemoryTokenSigning,
    round: SignatureRound,
    from_the_site: &BTreeMap<String, String>,
) -> Result<Vec<u8>, SiteRefusal> {
    signed_in(ServerFormat::Cades, server, token, round, from_the_site)
}

fn signed_in(
    format: ServerFormat,
    server: &InMemoryTriphaseServer,
    token: &InMemoryTokenSigning,
    round: SignatureRound,
    from_the_site: &BTreeMap<String, String>,
) -> Result<Vec<u8>, SiteRefusal> {
    let certificate = a_usable_certificate("firmante");
    signed_through_the_server(
        &ServerRun {
            server,
            token,
            certificate: &certificate,
            secret: "1234",
        },
        &ServerAsk {
            format,
            round,
            algorithm: AskedAlgorithm::Sha256,
            document: b"los datos",
            from_the_site,
        },
    )
}

fn the_situation_of(refusal: SiteRefusal) -> Situation {
    match refusal {
        SiteRefusal::Triphase(error) => error.situation(),
        other => panic!("tenia que ser un fallo del servidor trifasico: {other:?}"),
    }
}

#[test]
fn the_site_gets_the_signature_that_the_postsign_hands_back() {
    let server = InMemoryTriphaseServer::answering(
        &a_presignature_asking_to_sign(b"prefirma"),
        &a_postsign_handing_back(b"la firma del servidor"),
    );
    let token = InMemoryTokenSigning::default();

    let signature = signed_with(
        &server,
        &token,
        SignatureRound::First,
        &the_site_declaring(&[("serverUrl", SERVER_URL)]),
    )
    .expect("la firma sale");

    assert_eq!(signature, b"la firma del servidor");
}

#[test]
fn the_token_signs_the_presignature_with_the_hash_the_site_asked() {
    let server = InMemoryTriphaseServer::answering(
        &a_presignature_asking_to_sign(b"prefirma"),
        &a_postsign_handing_back(b"firma"),
    );
    let token = InMemoryTokenSigning::default();

    signed_with(
        &server,
        &token,
        SignatureRound::First,
        &the_site_declaring(&[("serverUrl", SERVER_URL)]),
    )
    .expect("la firma sale");

    assert_eq!(
        token.signed(),
        vec![("SHA256".to_owned(), b"prefirma".to_vec())]
    );
}

#[test]
fn both_phases_reach_the_server_url_with_the_operation_and_the_postsign_brings_the_pk1() {
    let server = InMemoryTriphaseServer::answering(
        &a_presignature_asking_to_sign(b"prefirma"),
        &a_postsign_handing_back(b"firma"),
    );
    let token = InMemoryTokenSigning::default();

    signed_with(
        &server,
        &token,
        SignatureRound::Counter {
            target: CounterTarget::Leafs,
        },
        &the_site_declaring(&[("serverUrl", SERVER_URL)]),
    )
    .expect("la firma sale");

    let forms = server.forms();
    assert_eq!(forms.len(), 2);
    for ((url, form), op) in forms.iter().zip(["pre", "post"]) {
        assert_eq!(url, SERVER_URL);
        assert_eq!(the_value_of(form, "op"), Some(op));
        assert_eq!(the_value_of(form, "cop"), Some("countersign"));
    }
    let session = URL_SAFE
        .decode(the_value_of(&forms[1].1, "session").expect("la postfirma trae la sesion"))
        .expect("en Base64");
    let session = String::from_utf8(session).expect("es XML");
    let pk1 = STANDARD.encode(b"PK1:prefirma");
    assert!(session.contains(&format!("<param n=\"PK1\">{pk1}</param>")));
    assert!(!session.contains("\"PRE\""));
}

#[test]
fn without_server_url_nothing_reaches_the_server_nor_the_token() {
    let server = InMemoryTriphaseServer::default();
    let token = InMemoryTokenSigning::default();

    let refusal = signed_with(&server, &token, SignatureRound::First, &BTreeMap::new())
        .expect_err("no hay servidor");

    assert_eq!(the_situation_of(refusal), Situation::ServerUrlMissing);
    assert!(server.forms().is_empty());
    assert_eq!(token.signing_attempts(), 0);
}

#[test]
fn a_failing_presign_is_refused_before_the_token_signs() {
    let server = InMemoryTriphaseServer::answering(
        b"ERR-14:prefirma:java.io.IOException: la sede no entrega el documento",
        b"",
    );
    let token = InMemoryTokenSigning::default();

    let refusal = signed_with(
        &server,
        &token,
        SignatureRound::First,
        &the_site_declaring(&[("serverUrl", SERVER_URL)]),
    )
    .expect_err("el servidor fallo");

    assert_eq!(the_situation_of(refusal), Situation::ServerException);
    assert_eq!(token.signing_attempts(), 0);
    assert_eq!(server.forms().len(), 1);
}

#[test]
fn a_token_that_refuses_leaves_the_postsign_unasked() {
    let server = InMemoryTriphaseServer::answering(
        &a_presignature_asking_to_sign(b"prefirma"),
        &a_postsign_handing_back(b"firma"),
    );
    let token = InMemoryTokenSigning::refusing(SigningRefusal {
        code: SafCode::SignatureFailed,
        situation: "incorrectPin".to_owned(),
        detail: "pin incorrecto".to_owned(),
        attempts_left: Some(2),
    });

    let refusal = signed_with(
        &server,
        &token,
        SignatureRound::First,
        &the_site_declaring(&[("serverUrl", SERVER_URL)]),
    )
    .expect_err("el token no firma");

    assert!(matches!(refusal, SiteRefusal::Signing(_)), "{refusal:?}");
    assert_eq!(server.forms().len(), 1);
}

#[test]
fn the_algorithm_travels_composed_with_the_key_of_the_certificate() {
    assert_eq!(
        composed_name(AskedAlgorithm::Sha256, Some(KeyKind::Rsa))
            .ok()
            .as_deref(),
        Some("SHA256withRSA")
    );
    assert_eq!(
        composed_name(AskedAlgorithm::Sha512, Some(KeyKind::Ec))
            .ok()
            .as_deref(),
        Some("SHA512withECDSA")
    );
}

#[test]
fn a_key_neither_rsa_nor_ec_never_travels_to_the_server() {
    let refusal = composed_name(AskedAlgorithm::Sha256, None).expect_err("no se compone con RSA");

    assert!(
        matches!(&refusal, SiteRefusal::Token(error) if error.situation() == TokenSituation::KeyNotRsa),
        "{refusal:?}"
    );
}

#[test]
fn a_pades_cosignature_reaches_the_server_as_a_pades_signature_with_every_param() {
    let server = InMemoryTriphaseServer::answering(
        &a_presignature_asking_to_sign(b"prefirma"),
        &a_postsign_handing_back(b"pdf firmado"),
    );
    let token = InMemoryTokenSigning::default();
    let from_the_site = the_site_declaring(&[("serverUrl", SERVER_URL)]);

    let signature = signed_in(
        ServerFormat::Pades,
        &server,
        &token,
        SignatureRound::Again,
        &from_the_site,
    )
    .expect("la firma sale");

    assert_eq!(signature, b"pdf firmado");
    for (_, form) in server.forms() {
        assert_eq!(the_value_of(&form, "format"), Some("pades"));
        assert_eq!(the_value_of(&form, "cop"), Some("sign"));
        assert_eq!(
            the_value_of(&form, "params"),
            Some(URL_SAFE.encode(to_java_properties(&from_the_site)).as_str())
        );
    }
}
