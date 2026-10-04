//! Los casos de grada C de `verify --json`: el mapeo desde el modelo real de firmas previas, validado contra el esquema de `verify`.

use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use openssl::bn::BigNum;
use serde_json::Value;

use super::schema::conforming_json;
use super::support::signing_certificate;
use super::*;

fn in_json(path: &Path) -> Value {
    let outcome = attended(&["verify", "-i", &path.display().to_string(), "--json"]);
    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert!(outcome.stderr.is_empty(), "{:?}", outcome.stderr);
    conforming_json("verify", &outcome.stdout)
}

fn utc(instant: SystemTime) -> String {
    DateTime::<Utc>::from(instant).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn in_hexadecimal(decimal: &str) -> String {
    let hex = BigNum::from_dec_str(decimal)
        .and_then(|number| number.to_hex_str().map(|hex| hex.to_string()))
        .expect("el número de serie es decimal");
    if hex.len().is_multiple_of(2) {
        hex
    } else {
        format!("0{hex}")
    }
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn verify_json_of_a_valid_signature_gives_it_valid_with_its_certificate() {
    let verified = in_json(&sample("reference/cades-implicit.p7s"));

    assert_eq!(verified["standard"], "CAdES");
    assert_eq!(verified["validity"], "valid");
    let signature = &verified["signatures"][0];
    assert_eq!(signature["validity"], "valid");
    assert!(signature.get("reason").is_none(), "{signature}");
    let certificate = &signature["certificate"];
    for key in ["subject", "issuer", "serialNumber", "notBefore", "notAfter"] {
        assert!(certificate[key].is_string(), "{key} en {certificate}");
    }
    assert!(signature["signatureAlgorithm"]["name"].is_string());
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn verify_json_of_a_cades_made_with_the_ec_certificate_gives_the_ecdsa_name_and_oid() {
    let verified = in_json(&a_cades_signed_with_the_ec_certificate());

    assert_eq!(
        verified["signatures"][0]["signatureAlgorithm"],
        serde_json::json!({"name": "ecdsa-with-SHA256", "oid": "1.2.840.10045.4.3.2"})
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn verify_json_of_a_tampered_cades_gives_it_invalid_and_modified_after_signing() {
    let verified = in_json(&a_tampered_cades());

    assert_eq!(verified["validity"], "invalid");
    let signature = &verified["signatures"][0];
    assert_eq!(signature["validity"], "invalid");
    assert_eq!(signature["reason"]["type"], "modifiedAfterSigning");
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn verify_json_of_an_expired_certificate_gives_the_date_it_expired() {
    let verified = in_json(&sample("previous-signatures/cades-expired.csig"));

    assert_eq!(verified["validity"], "expired");
    let signature = &verified["signatures"][0];
    assert_eq!(signature["reason"]["type"], "certificateExpired");
    assert_eq!(
        signature["reason"]["date"],
        signature["certificate"]["notAfter"]
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn verify_json_of_a_pdf_signed_twice_gives_both_signatures_with_the_token_certificate() {
    let since = Utc::now();
    let token = signing_certificate();
    let (not_before, not_after) = token.validity().expect("el certificado se lee");

    let verified = in_json(&a_pdf_signed_twice_with_the_token());

    assert_eq!(verified["standard"], "PAdES");
    assert_eq!(verified["validity"], "valid");
    let signatures = verified["signatures"].as_array().expect("una lista");
    assert_eq!(signatures.len(), 2);
    for signature in signatures {
        let certificate = &signature["certificate"];
        assert_eq!(
            certificate["serialNumber"],
            in_hexadecimal(&token.serial_number().expect("número de serie")).as_str()
        );
        assert_eq!(certificate["notBefore"], utc(not_before).as_str());
        assert_eq!(certificate["notAfter"], utc(not_after).as_str());
        let common_name = signature["signer"]["commonName"].as_str().expect("CN");
        assert!(
            common_name.starts_with("EIDAS CERTIFICADO PRUEBAS"),
            "{signature}"
        );
        assert!(
            certificate["subject"]
                .as_str()
                .is_some_and(|subject| subject.contains(&format!("CN={common_name}"))),
            "{certificate}"
        );
        assert_eq!(
            signature["signatureAlgorithm"],
            serde_json::json!({
                "name": "sha256WithRSAEncryption",
                "oid": "1.2.840.113549.1.1.11"
            })
        );
        let signed_at = signature["signingTime"]["at"]
            .as_str()
            .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
            .expect("la fecha de la firma");
        assert!(
            signed_at.timestamp() >= since.timestamp() - 1,
            "{signature}"
        );
        assert_eq!(signature["closesDocument"], false);
    }
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn verify_json_of_a_file_that_is_not_a_signature_gives_an_empty_list() {
    let verified = in_json(&sample("reference/challenge.bin"));

    assert_eq!(
        verified,
        serde_json::json!({"standard": "unrecognized", "findings": [], "signatures": []})
    );
}
