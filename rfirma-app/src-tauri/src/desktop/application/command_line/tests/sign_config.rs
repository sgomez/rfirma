use super::*;

#[test]
fn sign_hands_the_signer_the_properties_of_config() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner::default();

    let outcome = signed_over(
        &[
            "sign",
            "-i",
            "doc.pdf",
            "-o",
            "f.pdf",
            "-alias",
            "yo",
            "-config",
            "# motivo\\nsignReason=Conforme\\nheadless=true",
        ],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    let expected = BTreeMap::from([("signReason".to_owned(), "Conforme".to_owned())]);
    assert_eq!(*signer.parameters.borrow(), vec![expected]);
}

#[test]
fn a_config_a_site_could_not_declare_fails_before_opening_any_store() {
    let stores = StoresWith::labels(&["yo"]);
    let signer = RecordingSigner::default();
    let files = FilesInMemory::with("doc.pdf", A_PDF);

    let outcome = attended_in(
        &[
            "sign",
            "-i",
            "doc.pdf",
            "-o",
            "f.pdf",
            "-alias",
            "yo",
            "-config",
            "visibleSignature=want",
        ],
        &stores,
        &RecordingDesktop::default(),
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("-config"), "{}", said(&outcome));
    assert!(!stores.opened.get());
    assert!(signer.asked.borrow().is_empty());
    assert_eq!(files.at("f.pdf"), None);
}
