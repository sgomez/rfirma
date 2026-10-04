use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use super::schema::conforming_json;
use super::*;
use crate::identity::application::tests::TestAuthority;

struct StoresHolding(Vec<TokenCertificate>);

impl CertificateStores for StoresHolding {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.0.clone())
    }

    fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
        None
    }
}

fn listed_as_json(words: &[&str], stores: &dyn CertificateStores) -> Value {
    let outcome = attended_with(words, stores);
    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    conforming_json("listaliases", &outcome.stdout)
}

fn utc(instant: std::time::SystemTime) -> String {
    DateTime::<Utc>::from(instant).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn in_a_store(store: Store, label: &str) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(store, "token", label, None),
        TestAuthority::root("Otro").der(),
    )
}

#[test]
fn listaliases_json_gives_each_certificate_with_its_alias_store_names_and_validity() {
    let root = TestAuthority::root_with_serial("Raíz de pruebas", 0x0A_1B2C);
    let signer = root
        .issues("Firmante de pruebas")
        .as_certificate("FIRMANTE");
    let (not_before, not_after) = signer.validity().expect("el DER se lee");

    let listed = listed_as_json(
        &["listaliases", "-json"],
        &StoresHolding(vec![root.as_certificate("RAIZ"), signer]),
    );

    let certificates = listed["certificates"].as_array().expect("lista");
    assert_eq!(certificates.len(), 2);
    assert_eq!(certificates[0]["alias"], "RAIZ");
    assert_eq!(certificates[0]["serialNumber"], "0A1B2C");
    assert_eq!(
        certificates[1],
        json!({
            "alias": "FIRMANTE",
            "store": "pkcs11:/usr/lib/softhsm/libsofthsm2.so",
            "subject": "CN=Firmante de pruebas",
            "issuer": "CN=Raíz de pruebas",
            "serialNumber": certificates[1]["serialNumber"],
            "notBefore": utc(not_before),
            "notAfter": utc(not_after),
        })
    );
}

#[test]
fn listaliases_json_writes_the_text_in_utf8_without_escaping_it() {
    let root = TestAuthority::root("Raíz de pruebas");

    let outcome = attended_with(
        &["listaliases", "-json"],
        &StoresHolding(vec![root.as_certificate("RAÍZ")]),
    );

    let stdout = String::from_utf8(outcome.stdout).expect("UTF-8");
    assert!(stdout.contains("\"alias\":\"RAÍZ\""), "{stdout}");
    assert!(!stdout.contains("\\u"), "{stdout}");
}

#[test]
fn listaliases_json_names_the_store_as_store_accepts_it() {
    let firefox = Store::nss(
        "/usr/lib/libsoftokn3.so",
        Path::new("/home/persona/.mozilla/firefox/perfil"),
    );

    let listed = listed_as_json(
        &["listaliases", "-json"],
        &StoresHolding(vec![
            in_a_store(Store::module("/usr/lib/opensc-pkcs11.so"), "TARJETA"),
            in_a_store(firefox, "NAVEGADOR"),
            in_a_store(Store::module("cng:My"), "WINDOWS"),
        ]),
    );

    let stores: Vec<&Value> = listed["certificates"]
        .as_array()
        .expect("lista")
        .iter()
        .map(|certificate| &certificate["store"])
        .collect();
    assert_eq!(
        stores,
        [
            &json!("pkcs11:/usr/lib/opensc-pkcs11.so"),
            &json!("mozilla"),
            &json!("windows")
        ]
    );
}

#[test]
fn listaliases_json_with_no_certificate_gives_an_empty_list() {
    let listed = listed_as_json(&["listaliases", "-json"], &StoresWith::labels(&[]));

    assert_eq!(listed, json!({"certificates": []}));
}

#[test]
fn listaliases_json_leaves_out_what_an_unreadable_certificate_cannot_say() {
    let listed = listed_as_json(&["listaliases", "-json"], &StoresWith::labels(&["ROTO"]));

    assert_eq!(
        listed,
        json!({"certificates": [{"alias": "ROTO", "store": "pkcs11:/modulo.so"}]})
    );
}

#[test]
fn listaliases_json_keeps_quotes_backslashes_and_control_characters_of_an_alias() {
    let alias = "a\"b\\c\n\t\u{1}d";

    let listed = listed_as_json(&["listaliases", "-json"], &StoresWith::labels(&[alias]));

    assert_eq!(listed["certificates"][0]["alias"], alias);
}

#[test]
fn listaliases_json_that_fails_leaves_stdout_empty_and_says_why_on_stderr() {
    let refused = attended_with(
        &["listaliases", "-json", "-store", "dnie"],
        &StoresWith::labels(&["UNO"]),
    );
    let failed = attended_with(&["listaliases", "-json"], &NoStoreOpens);
    let with_a_password = attended(&["listaliases", "-json", "-password-fd", "3"]);

    for outcome in [refused, failed, with_a_password] {
        assert_ne!(outcome.exit_code, SUCCEEDED);
        assert!(outcome.stdout.is_empty(), "{:?}", outcome.stdout);
        assert!(!outcome.stderr.is_empty());
    }
}
