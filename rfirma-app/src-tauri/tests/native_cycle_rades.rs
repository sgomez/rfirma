//! Pruebas de integración del CMS y el CAdES firmados con rades contra el token, validado por el puente y por openssl (ADR-0001, ADR-0014).
#![cfg(feature = "rades")]

#[path = "native_cycle/support.rs"]
mod support;

use std::collections::BTreeMap;

use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::domain::algorithm::SignatureAlgorithm;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::signing::application::cycle::{self, SigningRequest};
use rfirma_lib::signing::application::rades::Rades;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
use rfirma_lib::signing::domain::{AdmissibleDocument, SignatureConfig, Waivers};

use support::{
    certificate_labelled, ecdsa_composed_for_the_ec_certificate, openssl_cms_finds_no_content_in,
    openssl_cms_verify, signing_certificate, the_original_validator_accepts, write_to_target,
    ACTIVE_EC, CHALLENGE, PIN,
};

fn a_rades_signed_by(
    format: Format,
    certificate: &TokenCertificate,
    algorithm: SignatureAlgorithm,
    declared: &[(&str, &str)],
) -> Vec<u8> {
    let rades = Rades::on_the_system_clock();
    let chain = certificate.chain();
    let reference = certificate.reference().clone();
    let config = SignatureConfig {
        placement: None,
        layer2_text: Some(String::new()),
        rubric_image: None,
        allow_unregistered_signatures: false,
    };
    let from_the_site: BTreeMap<String, String> = declared
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();

    let cycle = cycle::presign(
        &rades,
        SigningRequest {
            format,
            algorithm,
            operation: SignatureOperation::Sign,
            document: AdmissibleDocument::check_for(format, CHALLENGE, Waivers::NONE)
                .expect("el formato admite el reto"),
            chain: &chain,
            config: &config,
            from_the_site: &from_the_site,
            certificate: &reference,
        },
    )
    .expect("rades prefirma");
    let signature = cycle
        .sign_on_token(&pkcs11::RealToken, &ProtectedSecret::from_str(PIN))
        .expect("el token firma los atributos");
    cycle
        .postsign(&rades, signature, &cycle.seal_in_transit())
        .expect("rades postfirma")
        .into_signed_document()
}

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn an_implicit_rades_cms_with_the_rsa_certificate_validates_in_the_bridge_and_in_openssl() {
    let signed = a_rades_signed_by(Format::Cms, &signing_certificate(), cycle::ALGORITHM, &[]);
    let signature = write_to_target("rades-cms-implicito.p7s", &signed);

    assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    the_original_validator_accepts(&signature);
}

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn an_explicit_rades_cms_is_detached_and_openssl_verifies_it_against_the_challenge() {
    let signed = a_rades_signed_by(
        Format::Cms,
        &signing_certificate(),
        cycle::ALGORITHM,
        &[("mode", "explicit")],
    );
    let signature = write_to_target("rades-cms-explicito.p7s", &signed);
    let challenge = write_to_target("rades-cms-reto.bin", CHALLENGE);

    openssl_cms_finds_no_content_in(&signature);
    assert_eq!(openssl_cms_verify(&signature, Some(&challenge)), CHALLENGE);
}

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn a_rades_cms_with_the_ec_certificate_validates_in_the_bridge_and_in_openssl() {
    let certificate = certificate_labelled(ACTIVE_EC);
    let algorithm = ecdsa_composed_for_the_ec_certificate(&certificate);

    let signed = a_rades_signed_by(Format::Cms, &certificate, algorithm, &[]);
    let signature = write_to_target("rades-cms-ecdsa.p7s", &signed);

    assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    the_original_validator_accepts(&signature);
}

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn an_implicit_rades_cades_with_the_rsa_certificate_validates_in_the_bridge_and_in_openssl() {
    let signed = a_rades_signed_by(Format::Cades, &signing_certificate(), cycle::ALGORITHM, &[]);
    let signature = write_to_target("rades-cades-implicita.p7s", &signed);

    assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    the_original_validator_accepts(&signature);
}

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn an_explicit_rades_cades_is_detached_and_openssl_verifies_it_against_the_challenge() {
    let signed = a_rades_signed_by(
        Format::Cades,
        &signing_certificate(),
        cycle::ALGORITHM,
        &[("mode", "explicit")],
    );
    let signature = write_to_target("rades-cades-explicita.p7s", &signed);
    let challenge = write_to_target("rades-cades-reto.bin", CHALLENGE);

    openssl_cms_finds_no_content_in(&signature);
    assert_eq!(openssl_cms_verify(&signature, Some(&challenge)), CHALLENGE);
    the_original_validator_accepts(&signature);
}

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn a_rades_cades_with_the_ec_certificate_validates_in_the_bridge_and_in_openssl() {
    let certificate = certificate_labelled(ACTIVE_EC);
    let algorithm = ecdsa_composed_for_the_ec_certificate(&certificate);

    let signed = a_rades_signed_by(Format::Cades, &certificate, algorithm, &[]);
    let signature = write_to_target("rades-cades-ecdsa.p7s", &signed);

    assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    the_original_validator_accepts(&signature);
}

const THE_AGE_POLICY: [(&str, &str); 4] = [
    ("policyIdentifier", "urn:oid:2.16.724.1.3.1.1.2.1.9"),
    ("policyIdentifierHash", "G7roucf600+f03r/o0bAOQ6WAs0="),
    (
        "policyIdentifierHashAlgorithm",
        "http://www.w3.org/2000/09/xmldsig#sha1",
    ),
    (
        "policyQualifier",
        "https://sede.administracion.gob.es/politica_de_firma_anexo_1.pdf",
    ),
];

#[test]
#[ignore = "grada C con la feature rades: necesita el token y librfirma_crypto.so"]
fn a_rades_cades_under_the_age_policy_carries_the_policy_and_the_data_and_validates() {
    let signed = a_rades_signed_by(
        Format::Cades,
        &signing_certificate(),
        cycle::ALGORITHM,
        &THE_AGE_POLICY,
    );
    let signature = write_to_target("rades-cades-politica-age.p7s", &signed);

    let decoded = rades::cades::decode(&signed).expect("rades lee su CAdES");
    assert_eq!(
        decoded.signers()[0].signature_policy(),
        Some(&rades::SignaturePolicy::age_v1_9())
    );
    assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    the_original_validator_accepts(&signature);
}
