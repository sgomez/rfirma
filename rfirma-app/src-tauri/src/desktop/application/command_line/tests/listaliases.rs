use super::*;

#[test]
fn listaliases_writes_one_alias_per_line_on_stdout_and_succeeds() {
    let outcome = attended_with(&["listaliases"], &StoresWith::labels(&["UNO", "DOS"]));

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(outcome.stdout, b"UNO\nDOS\n");
    assert!(outcome.stderr.is_empty(), "{:?}", outcome.stderr);
}

#[test]
fn listaliases_with_no_certificate_succeeds_with_an_empty_stdout_and_says_so_on_stderr() {
    let outcome = attended(&["LISTALIASES"]);

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert!(outcome.stdout.is_empty());
    assert!(
        said(&outcome).contains("ningún certificado"),
        "{}",
        said(&outcome)
    );
}

#[test]
fn listaliases_fails_on_stderr_when_no_store_opens() {
    let outcome = attended_with(&["listaliases"], &NoStoreOpens);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
    assert!(said(&outcome).contains("almacén"), "{}", said(&outcome));
}

#[test]
fn listaliases_with_a_store_rfirma_does_not_open_is_refused_without_opening_any_store() {
    for store in [
        "pkcs12:/a.p12",
        "dni",
        "dnie",
        "windows",
        "mac",
        "inventado",
    ] {
        let stores = StoresWith::labels(&["UNO"]);

        let outcome = attended_with(&["listaliases", "-store", store], &stores);

        assert_eq!(outcome.exit_code, REFUSED, "{store}");
        assert!(outcome.stdout.is_empty());
        assert!(said(&outcome).contains("almacén"), "{}", said(&outcome));
        assert!(!stores.opened.get(), "{store}");
    }
}

#[test]
fn listaliases_with_a_module_that_was_not_discovered_fails_without_opening_any_store() {
    let stores = StoresWith::labels(&["UNO"]);

    let outcome = attended_with(&["listaliases", "-store", "pkcs11:/otro.so"], &stores);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
    assert!(said(&outcome).contains("/otro.so"), "{}", said(&outcome));
    assert!(!stores.opened.get());
}

#[test]
fn listaliases_with_a_discovered_module_lists_its_certificates() {
    let outcome = attended_with(
        &["listaliases", "-store", "pkcs11:/modulo.so"],
        &StoresWith::labels(&["UNO"]),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(String::from_utf8(outcome.stdout).unwrap(), "UNO\n");
}

#[test]
fn listaliases_with_the_nss_family_leaves_the_card_modules_out() {
    let outcome = attended_with(
        &["listaliases", "-store", "mozilla"],
        &StoresWith::labels(&["UNO"]),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert!(outcome.stdout.is_empty());
}
