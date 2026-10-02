use super::*;

/// El motor de filtros que deja pasar solo los certificados de esas etiquetas.
struct AcceptingLabels(Vec<&'static str>);

impl CertificateFilter for AcceptingLabels {
    fn accepted(
        &self,
        filter: &SiteFilter,
        certificates: Vec<TokenCertificate>,
    ) -> Result<Vec<TokenCertificate>, String> {
        assert_eq!(filter.as_java_properties(), "filters=nonexpired:\n");
        Ok(certificates
            .into_iter()
            .filter(|certificate| self.0.contains(&certificate.reference().label()))
            .collect())
    }
}

fn filtered_with(accepted: &[&'static str], words: &[&str], signer: &RecordingSigner) -> Outcome {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let ports = CommandLinePorts {
        stores: &StoresWith::labels(&["otro", "yo"]),
        terminal: &ScriptedTerminal,
        desktop: &RecordingDesktop::default(),
        filter: &AcceptingLabels(accepted.to_vec()),
        files: &files,
        verifier: &Untouched,
        signer,
    };
    attend(&arguments_of(words), &ports)
}

const FILTERED: [&str; 7] = [
    "sign",
    "-i",
    "doc.pdf",
    "-o",
    "f.pdf",
    "-filter",
    "nonexpired:",
];

#[test]
fn sign_with_a_filter_that_leaves_one_certificate_signs_with_it() {
    let signer = RecordingSigner::default();

    let outcome = filtered_with(&["yo"], &FILTERED, &signer);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(signer.asked.borrow()[0].1, "yo");
}

#[test]
fn sign_with_a_filter_that_leaves_none_or_several_fails_without_signing() {
    for (accepted, wording) in [(vec![], "ningún"), (vec!["otro", "yo"], "varios")] {
        let signer = RecordingSigner::default();

        let outcome = filtered_with(&accepted, &FILTERED, &signer);

        assert_eq!(outcome.exit_code, FAILED, "{accepted:?}");
        assert!(said(&outcome).contains(wording), "{}", said(&outcome));
        assert!(signer.asked.borrow().is_empty());
    }
}

#[test]
fn sign_with_a_filter_naming_no_criterion_of_the_site_fails_before_opening_any_store() {
    let stores = StoresWith::labels(&["yo"]);
    let signer = RecordingSigner::default();

    let outcome = attended_in(
        &[
            "sign",
            "-i",
            "doc.pdf",
            "-o",
            "f.pdf",
            "-filter",
            "inventado:x",
        ],
        &stores,
        &RecordingDesktop::default(),
        &FilesInMemory::with("doc.pdf", A_PDF),
        &signer,
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("inventado:x"), "{}", said(&outcome));
    assert!(!stores.opened.get());
}

fn xml_of(outcome: &Outcome) -> String {
    String::from_utf8(outcome.stdout.clone()).expect("la respuesta es UTF-8")
}

#[test]
fn xml_with_an_output_answers_on_stdout_without_the_signature() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);

    let outcome = signed_over(
        &[
            "sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo", "-xml",
        ],
        &files,
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(
        xml_of(&outcome),
        "<afirma><result>true</result><response><msg>firma guardada en «f.pdf»</msg></response></afirma>\n"
    );
    assert_eq!(files.at("f.pdf"), Some(SIGNED.to_vec()));
}

#[test]
fn xml_without_an_output_carries_the_signature_in_base64_and_writes_no_file() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);

    let outcome = signed_over(
        &["sign", "-i", "doc.pdf", "-alias", "yo", "-xml"],
        &files,
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    let expected = base64::engine::general_purpose::STANDARD.encode(SIGNED);
    assert!(
        xml_of(&outcome).contains(&format!("<sign>{expected}</sign>")),
        "{}",
        xml_of(&outcome)
    );
    assert_eq!(files.files.borrow().len(), 1);
}

#[test]
fn xml_with_a_failure_answers_result_false_and_ends_nonzero() {
    let signer = RecordingSigner {
        fails: true,
        ..RecordingSigner::default()
    };

    let outcome = signed_over(
        &[
            "sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo", "-xml",
        ],
        &FilesInMemory::with("doc.pdf", A_PDF),
        &signer,
    );

    assert_eq!(outcome.exit_code, FAILED);
    let xml = xml_of(&outcome);
    assert!(xml.starts_with("<afirma><result>false</result>"), "{xml}");
    assert!(xml.contains("el token no firma"), "{xml}");
    assert!(!xml.contains("<sign>"), "{xml}");
}
