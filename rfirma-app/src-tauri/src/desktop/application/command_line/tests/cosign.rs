use super::*;

#[test]
fn cosign_signs_through_the_signer_with_the_cosign_operation_and_the_same_parameters() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner::default();

    let outcome = signed_over(
        &[
            "cosign",
            "-i",
            "doc.pdf",
            "-o",
            "dos.pdf",
            "-alias",
            "yo",
            "-algorithm",
            "sha256",
        ],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(files.at("dos.pdf").as_deref(), Some(SIGNED));
    assert_eq!(
        *signer.operations.borrow(),
        vec![SignatureOperation::Cosign]
    );
    assert_eq!(signer.asked.borrow()[0].3, Algorithm::Sha256);
}

#[test]
fn sign_asks_the_signer_for_the_sign_operation() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner::default();

    signed_over(
        &["sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo"],
        &files,
        &signer,
    );

    assert_eq!(*signer.operations.borrow(), vec![SignatureOperation::Sign]);
}

#[test]
fn the_double_dash_form_of_every_option_gives_the_same_result_as_the_original_form() {
    let single = [
        "-i",
        "doc.pdf",
        "-o",
        "dos.pdf",
        "-alias",
        "yo",
        "-algorithm",
        "sha256",
    ];
    let double = [
        "-i",
        "doc.pdf",
        "-o",
        "dos.pdf",
        "--alias",
        "yo",
        "--algorithm",
        "sha256",
    ];
    let mut results = Vec::new();
    for options in [single, double] {
        let files = FilesInMemory::with("doc.pdf", A_PDF);
        let signer = RecordingSigner::default();
        let mut words = vec!["cosign"];
        words.extend(options);

        let outcome = signed_over(&words, &files, &signer);

        assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
        results.push((files.at("dos.pdf"), signer.asked.borrow()[0].3));
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn the_errors_name_the_double_dash_form() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner::default();

    let outcome = signed_over(&["sign", "-i", "doc.pdf", "-o", "f.pdf"], &files, &signer);

    let said = said(&outcome);
    assert!(said.contains("--alias"), "{said}");
    assert!(!said.contains(" -alias"), "{said}");
}

#[test]
fn the_password_is_refused_in_either_form_naming_the_double_dash_alternative() {
    for password in ["-password", "--password"] {
        let files = FilesInMemory::with("doc.pdf", A_PDF);
        let signer = RecordingSigner::default();

        let outcome = signed_over(
            &[
                "sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo", password, "1234",
            ],
            &files,
            &signer,
        );

        assert_ne!(outcome.exit_code, SUCCEEDED);
        assert!(
            said(&outcome).contains("--password-fd"),
            "{}",
            said(&outcome)
        );
    }
}
