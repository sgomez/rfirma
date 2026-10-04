use super::*;
use crate::signing::domain::catalog::counted;

const VERBOSITIES: [&str; 3] = ["-v", "-vv", "-vvv"];

fn a_document_with_everything() -> ReadingWithFindings {
    let mut represented = a_signature(
        "00000000T NOMBRE APELLIDOUNO (R: B00000000)",
        "IDCES-00000000T",
        None,
    );
    represented.organization_identifier = Some("VATES-B00000000".to_owned());
    represented.organization_name = Some("EMPRESA FICTICIA SL".to_owned());
    represented.signing_date = Some(SigningDate::Stamped {
        at: "2026-09-20T16:01:44Z".to_owned(),
        tsa: "TSA FICTICIA".to_owned(),
    });
    represented.validity = Validity::Expired;
    represented.validity_reason = Some(ValidityReason::CertificateExpired {
        date: "2026-01-02T00:00:00Z".to_owned(),
        holder: None,
    });
    represented.certificate_valid_from = Some("2025-01-01T00:00:00Z".to_owned());
    represented.certificate_valid_until = Some("2030-06-30T12:30:00Z".to_owned());
    represented.signature_algorithm = Some("SHA256withRSA".to_owned());
    represented.profile = Some("PAdES B-B-Level".to_owned());
    let mut broken = a_signature("OTRA PERSONA", "", None);
    broken.signing_date = Some(SigningDate::Declared {
        at: "2026-09-21T10:00:00Z".to_owned(),
    });
    broken.validity = Validity::Invalid;
    broken.validity_reason = Some(ValidityReason::Damaged);
    broken.certificate_valid_from = Some("2025-01-01T00:00:00Z".to_owned());
    represented.countersignatures = vec![broken];
    let mut third = a_signature("TERCERA PERSONA", "", None);
    third.certificate_valid_until = Some("2030-06-30T12:30:00Z".to_owned());
    ReadingWithFindings(
        vec![represented, third],
        vec![
            DocumentFinding::ModifiedAfterLastSignature,
            DocumentFinding::FormFilledAfterSigning,
            DocumentFinding::ContentAddedOnTop,
        ],
    )
}

fn a_document_with_one_expired_signature() -> ReadingWithFindings {
    let mut expired = a_signature("UNA PERSONA", "", None);
    expired.validity = Validity::Expired;
    ReadingWithFindings(vec![expired], Vec::new())
}

fn every_output_in(language: Language) -> Vec<String> {
    let mut outputs = Vec::new();
    for verbosity in VERBOSITIES {
        let words = ["verify", verbosity, "-i", "firmado.pdf"];
        for document in [
            a_document_with_everything(),
            a_document_with_one_expired_signature(),
            ReadingWithFindings(Vec::new(), Vec::new()),
        ] {
            outputs.push(printed(&verified_reading_in(language, &words, &document)));
        }
        let verifier = Answering::with(&["x"]);
        let unrecognized = attended_in(
            language,
            &words,
            &OneFile(b"no es una firma"),
            &verifier,
            &Untouched,
            &SummerInMadrid,
        );
        outputs.push(printed(&unrecognized));
    }
    outputs
}

fn every_text_printed_in(language: Language) -> Vec<String> {
    let text = |key: &str, values: &[(&str, &str)]| translated(language, key, values);
    let mut texts: Vec<String> = [
        "panel.signed.field.signer",
        "panel.signed.field.onBehalfOf",
        "panel.signed.field.issuer",
        "panel.signed.field.date",
        "panel.signed.field.sealed",
        "panel.signed.field.reason",
        "commandLine.verify.field.serialNumber",
        "commandLine.verify.field.validity",
        "commandLine.verify.field.algorithm",
        "commandLine.verify.field.profile",
        "commandLine.verify.noSignatures",
        "panel.signed.unrecognized.title",
        "documentFinding.modifiedAfterLastSignature",
        "documentFinding.formFilledAfterSigning",
        "documentFinding.contentAddedOnTop",
        "signatureReason.damaged",
    ]
    .into_iter()
    .map(|key| text(key, &[]))
    .collect();
    texts.extend([
        text(
            "commandLine.verify.onBehalfOf",
            &[("entity", "EMPRESA FICTICIA SL")],
        ),
        text(
            "commandLine.verify.validFrom",
            &[("date", "2025-01-01 02:00:00 +02:00")],
        ),
        text(
            "commandLine.verify.validUntil",
            &[("date", "2030-06-30 14:30:00 +02:00")],
        ),
        text(
            "signatureReason.certificateExpired",
            &[("date", "2026-01-02")],
        ),
        counted(language, "panel.signed.count", 2),
        counted(language, "panel.signed.count", 1),
        counted(language, "panel.signed.countersignatureCount", 1),
        counted(language, "panel.previousSignatures.problems", 5),
        counted(language, "panel.previousSignatures.expired", 1),
    ]);
    texts
}

#[test]
fn every_text_of_verbose_comes_from_the_catalog() {
    let outputs = every_output_in(Language::Spanish).join("\n");

    for text in every_text_printed_in(Language::Spanish) {
        assert!(outputs.contains(&text), "«{text}» no sale en:\n{outputs}");
    }
}

#[test]
fn with_the_system_in_english_verbose_prints_no_spanish_text() {
    let english = every_output_in(Language::English).join("\n");

    for text in every_text_printed_in(Language::English) {
        assert!(english.contains(&text), "«{text}» no sale en:\n{english}");
    }
    for text in every_text_printed_in(Language::Spanish) {
        assert!(!english.contains(&text), "«{text}» sale en:\n{english}");
    }
}

#[test]
fn with_a_system_language_rfirma_does_not_publish_verbose_is_in_spanish() {
    let language = Language::first_of(["de_DE.UTF-8"]);

    assert_eq!(
        every_output_in(language),
        every_output_in(Language::Spanish)
    );
}

#[test]
fn the_header_counts_with_the_plural_rules_of_the_language() {
    let signatures = vec![a_signature("UNA", "", None), a_signature("OTRA", "", None)];
    let document = ReadingWithFindings(signatures, vec![DocumentFinding::ContentAddedOnTop]);

    for language in Language::ALL {
        let output = printed(&verified_reading_in(
            language,
            &["verify", "-v", "-i", "firmado.pdf"],
            &document,
        ));

        assert!(
            output.starts_with(&format!(
                "PAdES · {} · {}\n",
                counted(language, "panel.signed.count", 2),
                counted(language, "panel.previousSignatures.problems", 1)
            )),
            "{language:?}: {output}"
        );
    }
}
