use super::*;

fn signed_allowing_sha1(
    words: &[&str],
    files: &FilesInMemory,
    signer: &RecordingSigner,
) -> Outcome {
    let ports = CommandLinePorts {
        stores: &StoresWith::labels(&["yo"]),
        terminal: &ScriptedTerminal,
        descriptor: &ScriptedDescriptor,
        desktop: &RecordingDesktop::default(),
        filter: &Untouched,
        files,
        verifier: &Untouched,
        reader: &Untouched,
        time_zone: &Untouched,
        language: crate::signing::domain::Language::Spanish,
        platform: Platform::Linux,
        signer,
        window: &Untouched,
        sha1_allowed: true,
    };
    attend(&arguments_of(words), &ports)
}

#[test]
fn sha1_refused_by_the_preference_names_where_to_allow_it() {
    let outcome = attended(&[
        "sign",
        "-i",
        "a.pdf",
        "-o",
        "b.pdf",
        "-alias",
        "yo",
        "-algorithm",
        "sha1",
    ]);

    assert_eq!(outcome.exit_code, REFUSED);
    assert!(
        said(&outcome).contains("Preferencias → Firma"),
        "{}",
        said(&outcome)
    );
}

#[test]
fn sign_and_cosign_with_sha1_sign_with_sha1_when_the_preference_allows_it() {
    for (command, operation) in [
        ("sign", SignatureOperation::Sign),
        ("cosign", SignatureOperation::Cosign),
    ] {
        let files = FilesInMemory::with("doc.pdf", A_PDF);
        let signer = RecordingSigner::default();

        let outcome = signed_allowing_sha1(
            &[
                command,
                "-i",
                "doc.pdf",
                "-o",
                "firmado.pdf",
                "-alias",
                "yo",
                "-algorithm",
                "sha1",
            ],
            &files,
            &signer,
        );

        assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
        assert_eq!(signer.asked.borrow()[0].3, Algorithm::Sha1);
        assert_eq!(*signer.operations.borrow(), vec![operation]);
    }
}

#[test]
fn xades_with_sha1_signs_with_sha1_when_the_preference_allows_it() {
    let files = FilesInMemory::with("doc.xml", A_PDF);
    let signer = RecordingSigner::default();

    let outcome = signed_allowing_sha1(
        &[
            "sign",
            "-i",
            "doc.xml",
            "-o",
            "firmado.xsig",
            "-alias",
            "yo",
            "-format",
            "xades",
            "-algorithm",
            "sha1",
        ],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(signer.asked.borrow()[0].3, Algorithm::Sha1);
}
