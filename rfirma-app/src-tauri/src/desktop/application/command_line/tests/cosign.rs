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
