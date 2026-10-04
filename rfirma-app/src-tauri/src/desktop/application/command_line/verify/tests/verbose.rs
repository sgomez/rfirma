use super::*;

fn verbose(words: &[&str], signatures: Vec<DocumentSignature>) -> String {
    let outcome = verified_reading(words, &Reading(Ok(signatures)));
    assert_eq!(outcome.exit_code, SUCCEEDED);
    printed(&outcome)
}

fn expired_signature(name: &str) -> DocumentSignature {
    let mut signature = a_signature(name, "", Some("2026-09-20T16:01:44Z"));
    signature.validity = Validity::Expired;
    signature.validity_reason = Some(ValidityReason::CertificateExpired {
        date: "2026-01-02T00:00:00Z".to_owned(),
        holder: None,
    });
    signature
}

#[test]
fn verbose_starts_with_the_format_and_the_count_and_lists_one_line_per_signature() {
    let output = verbose(
        &["verify", "-i", "firmado.pdf", "-v"],
        vec![
            a_signature(
                "NOMBRE APELLIDO1 APELLIDO2",
                "99999999R",
                Some("2026-09-14T08:32:05Z"),
            ),
            a_signature(
                "OTRA PERSONA PRUEBA",
                "00000000T",
                Some("2026-09-20T23:30:00Z"),
            ),
        ],
    );

    assert_eq!(
        output,
        "\
PAdES · 2 firmas

✓ NOMBRE APELLIDO1 APELLIDO2 · 2026-09-14
✓ OTRA PERSONA PRUEBA · 2026-09-21
"
    );
}

#[test]
fn the_long_form_of_verbose_prints_the_same() {
    let reader = Reading(Ok(vec![a_signature("UNA PERSONA", "99999999R", None)]));

    let short = verified_reading(&["verify", "-i", "firmado.pdf", "-v"], &reader);
    let long = verified_reading(&["verify", "--verbose", "-i", "firmado.pdf"], &reader);

    assert_eq!(long, short);
}

#[test]
fn a_single_expired_signature_is_counted_as_expired_in_the_header() {
    let output = verbose(
        &["verify", "-i", "firmado.pdf", "-v"],
        vec![
            a_signature("UNA", "", Some("2026-09-14T08:32:05Z")),
            expired_signature("OTRA"),
            a_signature("TERCERA", "", None),
        ],
    );

    assert_eq!(
        output,
        "\
PAdES · 3 firmas · 1 caducada

✓ UNA · 2026-09-14
⚠ OTRA · 2026-09-20
✓ TERCERA
"
    );
}

#[test]
fn an_invalid_signature_makes_the_header_count_problems() {
    let mut broken = a_signature("ROTA", "", None);
    broken.validity = Validity::Invalid;
    broken.validity_reason = Some(ValidityReason::Damaged);

    let output = verbose(
        &["verify", "-i", "firmado.pdf", "-v"],
        vec![expired_signature("CADUCADA"), broken],
    );

    assert!(
        output.starts_with("PAdES · 2 firmas · 2 problemas\n"),
        "{output}"
    );
    assert!(output.contains("\n✗ ROTA\n"), "{output}");
}

#[test]
fn the_document_findings_come_above_the_signatures_and_count_as_problems() {
    let reader = ReadingWithFindings(
        vec![a_signature("UNA", "", None)],
        vec![
            DocumentFinding::ModifiedAfterLastSignature,
            DocumentFinding::FormFilledAfterSigning,
            DocumentFinding::ContentAddedOnTop,
        ],
    );

    let outcome = verified_reading_with(&["verify", "-v", "-i", "firmado.pdf"], &reader);

    assert_eq!(
        printed(&outcome),
        "\
PAdES · 1 firma · 3 problemas
⚠ Se ha modificado después de la última firma
⚠ Se ha rellenado el formulario después de firmar
⚠ Se ha añadido contenido encima de lo firmado

✓ UNA
"
    );
}

#[test]
fn a_document_without_signatures_says_so_in_the_header() {
    let output = verbose(&["verify", "-v", "-i", "firmado.pdf"], Vec::new());

    assert_eq!(output, "PAdES · sin firmas\n");
}

#[test]
fn data_of_no_signature_format_say_so_in_verbose_without_asking_for_signatures() {
    let verifier = Answering::with(&["x"]);

    let outcome = verified(
        &["verify", "-v", "-i", "datos.bin"],
        &OneFile(b"no es una firma"),
        &verifier,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(printed(&outcome), "Formato no reconocido\n");
}

#[test]
fn verbose_names_the_format_of_cades_xades_and_facturae() {
    for (document, format) in [
        (A_CMS, "CAdES"),
        (AN_XML, "XAdES"),
        (AN_INVOICE, "FacturaE"),
    ] {
        let reader = Reading(Ok(vec![a_signature("UNA PERSONA", "", None)]));
        let verifier = Answering::with(&["Firma valida"]);

        let outcome = attended(
            &["verify", "-v", "-i", "datos"],
            &OneFile(document),
            &verifier,
            &reader,
            &SummerInMadrid,
        );

        assert_eq!(
            printed(&outcome),
            format!("{format} · 1 firma\n\n✓ UNA PERSONA\n")
        );
    }
}

#[test]
fn verbose_indents_the_countersignatures_under_their_signature() {
    let deepest = a_signature("TERCERA PERSONA", "", None);
    let mut counter = a_signature("OTRA PERSONA", "", None);
    counter.countersignatures = vec![deepest];
    let mut signer = a_signature("UNA PERSONA", "", None);
    signer.countersignatures = vec![counter];
    let reader = Reading(Ok(vec![signer]));
    let verifier = Answering::with(&["Firma valida"]);

    let outcome = attended(
        &["verify", "-v", "-i", "datos.csig"],
        &OneFile(A_CMS),
        &verifier,
        &reader,
        &SummerInMadrid,
    );

    assert_eq!(
        printed(&outcome),
        "\
CAdES · 1 firma · 2 contrafirmas

✓ UNA PERSONA
    ✓ OTRA PERSONA
        ✓ TERCERA PERSONA
"
    );
}

#[test]
fn a_certified_pdf_shows_its_signatures() {
    let certified: &[u8] =
        b"%PDF-1.7\n9 0 obj\n<< /Type /Sig /Reference [ << /TransformMethod /DocMDP >> ] >>\nendobj";
    let engine = Reading(Ok(vec![a_signature("UNA PERSONA", "", None)]));
    let verifier = Answering::with(&["Firma valida"]);

    let outcome = attended(
        &["verify", "-v", "-i", "certificado.pdf"],
        &OneFile(certified),
        &verifier,
        &engine,
        &SummerInMadrid,
    );

    assert_eq!(printed(&outcome), "PAdES · 1 firma\n\n✓ UNA PERSONA\n");
}

#[test]
fn an_encrypted_pdf_fails_to_read_without_reaching_the_engine() {
    let encrypted: &[u8] = b"%PDF-1.7\ntrailer\n<< /Root 1 0 R /Encrypt 5 0 R >>";
    let verifier = Answering::with(&["Firma valida"]);

    let outcome = attended(
        &["verify", "-v", "-i", "cifrado.pdf"],
        &OneFile(encrypted),
        &verifier,
        &Untouched,
        &SummerInMadrid,
    );

    assert!(outcome.stdout.is_empty());
    assert_eq!(outcome.stderr.len(), 1);
    assert!(
        outcome.stderr[0].starts_with("rfirma: no se han podido leer las firmas del documento: "),
        "{:?}",
        outcome.stderr
    );
}

#[test]
fn signatures_that_cannot_be_read_print_nothing_warn_on_stderr_and_end_with_zero() {
    let reader = Reading(Err("el isolate no arranca".to_owned()));

    let outcome = verified_reading(&["verify", "-v", "-i", "firmado.pdf"], &reader);

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert!(outcome.stdout.is_empty());
    assert_eq!(
        outcome.stderr,
        ["rfirma: no se han podido leer las firmas del documento: el puente ha fallado: el isolate no arranca"]
    );
}

#[test]
fn a_representation_certificate_is_named_with_the_entity_on_whose_behalf_it_signs() {
    let mut signature = a_signature(
        "00000000T NOMBRE APELLIDOUNO (R: B00000000)",
        "IDCES-00000000T",
        Some("2026-09-14T08:32:05Z"),
    );
    signature.organization_identifier = Some("VATES-B00000000".to_owned());
    signature.organization_name = Some("EMPRESA FICTICIA SL".to_owned());

    let output = verbose(&["verify", "-v", "-i", "firmado.pdf"], vec![signature]);

    assert_eq!(
        output,
        "PAdES · 1 firma\n\n✓ NOMBRE APELLIDOUNO · por EMPRESA FICTICIA SL · 2026-09-14\n"
    );
}

#[test]
fn a_company_seal_is_named_by_the_company() {
    let mut signature = a_signature("EMPRESA FICTICIA SL - B00000000", "", None);
    signature.organization_identifier = Some("VATES-B00000000".to_owned());
    signature.organization_name = Some("EMPRESA FICTICIA SL".to_owned());

    let output = verbose(&["verify", "-v", "-i", "firmado.pdf"], vec![signature]);

    assert_eq!(output, "PAdES · 1 firma\n\n✓ EMPRESA FICTICIA SL\n");
}

#[test]
fn the_sheet_of_vv_has_the_signer_the_issuer_the_date_and_the_reason() {
    let mut signature = expired_signature("EIDAS CERTIFICADO PRUEBAS - 99999999R");
    signature.id_number = "IDCES-99999999R".to_owned();
    signature.signing_date = Some(SigningDate::Declared {
        at: "2026-09-20T16:01:44Z".to_owned(),
    });

    let output = verbose(&["verify", "-i", "firmado.pdf", "-vv"], vec![signature]);

    assert_eq!(
        output,
        "\
PAdES · 1 firma · 1 caducada

⚠ EIDAS CERTIFICADO PRUEBAS · 2026-09-20
  Firmante:          EIDAS CERTIFICADO PRUEBAS (99999999R)
  Emisor:            AC FNMT Usuarios
  Fecha:             2026-09-20 18:01:44 +02:00
  Motivo:            El certificado caducó el 2026-01-02
"
    );
}

#[test]
fn a_stamped_signature_says_sealed_with_the_tsa() {
    let mut signature = a_signature("UNA", "", None);
    signature.signing_date = Some(SigningDate::Stamped {
        at: "2026-09-20T16:01:44Z".to_owned(),
        tsa: "TSA FICTICIA".to_owned(),
    });

    let output = verbose(&["verify", "-i", "firmado.pdf", "-vv"], vec![signature]);

    assert!(
        output.contains("  Sellada:           2026-09-20 18:01:44 +02:00 (TSA FICTICIA)\n"),
        "{output}"
    );
    assert!(output.contains("✓ UNA · 2026-09-20\n"), "{output}");
}

#[test]
fn the_reasons_are_written_in_the_sheet() {
    for (reason, text) in [
        (
            ValidityReason::CertificateExpired {
                date: "2026-01-02T00:00:00Z".to_owned(),
                holder: Some("ACME SL".to_owned()),
            },
            "El certificado de ACME SL caducó el 2026-01-02",
        ),
        (
            ValidityReason::CertificateExpired {
                date: "2026-01-02T00:00:00Z".to_owned(),
                holder: None,
            },
            "El certificado caducó el 2026-01-02",
        ),
        (
            ValidityReason::ModifiedAfterSigning,
            "Se ha modificado después de firmarse",
        ),
        (ValidityReason::Damaged, "La firma está dañada"),
        (
            ValidityReason::CertificateNotYetValid {
                date: "2027-01-02T00:00:00Z".to_owned(),
            },
            "El certificado no se podía usar antes del 2027-01-02",
        ),
        (
            ValidityReason::UnknownSignatureType,
            "rFirma no conoce este tipo de firma",
        ),
        (
            ValidityReason::CosignNotAdmitted {
                closed_by: Some("UNA".to_owned()),
            },
            "UNA no admitía más firmas",
        ),
        (
            ValidityReason::CosignNotAdmitted { closed_by: None },
            "El documento no admitía más firmas",
        ),
    ] {
        let mut signature = a_signature("X", "", None);
        signature.validity = Validity::Invalid;
        signature.validity_reason = Some(reason);

        let output = verbose(&["verify", "-i", "firmado.pdf", "-vv"], vec![signature]);

        assert!(
            output.contains(&format!("  Motivo:            {text}\n")),
            "{output}"
        );
    }
}

#[test]
fn the_serial_number_is_printed_only_from_the_third_level() {
    let signature = || vec![a_signature("UNA PERSONA", "", None)];
    let with_serial = "  Número de serie:   0123ABCD\n";

    for words in [
        &["verify", "-i", "firmado.pdf", "-vvv"][..],
        &["verify", "-i", "firmado.pdf", "-v", "-v", "-v"][..],
    ] {
        let output = verbose(words, signature());
        assert!(output.ends_with(with_serial), "{words:?}: {output}");
    }
    for words in [
        &["verify", "-i", "firmado.pdf", "-v"][..],
        &["verify", "-i", "firmado.pdf", "-vv"][..],
    ] {
        assert!(!verbose(words, signature()).contains("Número de serie"));
    }
}

#[test]
fn the_third_level_adds_the_validity_algorithm_and_profile_after_the_serial_number() {
    let mut signature = a_signature("UNA PERSONA", "", None);
    signature.certificate_valid_from = Some("2025-01-01T00:00:00Z".to_owned());
    signature.certificate_valid_until = Some("2030-06-30T12:30:00Z".to_owned());
    signature.signature_algorithm = Some("SHA256withRSA".to_owned());
    signature.profile = Some("PAdES B-B-Level".to_owned());

    for words in [
        &["verify", "-i", "firmado.pdf", "-vvv"][..],
        &["verify", "-i", "firmado.pdf", "-vvvv"][..],
    ] {
        let output = verbose(words, vec![signature.clone()]);
        assert!(
            output.ends_with(
                "\
  Número de serie:   0123ABCD
  Vigencia:          2025-01-01 02:00:00 +02:00 – 2030-06-30 14:30:00 +02:00
  Algoritmo:         SHA256withRSA
  Perfil:            PAdES B-B-Level
"
            ),
            "{words:?}: {output}"
        );
    }
}

#[test]
fn the_new_rows_are_absent_below_the_third_level_and_when_the_field_is_unknown() {
    let mut signature = a_signature("UNA PERSONA", "", None);
    signature.signature_algorithm = Some("SHA256withRSA".to_owned());
    signature.certificate_valid_until = Some("2030-06-30T12:30:00Z".to_owned());

    let below = verbose(
        &["verify", "-i", "firmado.pdf", "-vv"],
        vec![signature.clone()],
    );
    let unknown = verbose(&["verify", "-i", "firmado.pdf", "-vvv"], vec![signature]);

    for label in ["Vigencia", "Algoritmo", "Perfil"] {
        assert!(!below.contains(label), "{below}");
    }
    assert!(unknown.contains("  Vigencia:          hasta 2030-06-30 14:30:00 +02:00\n"));
    assert!(unknown.contains("  Algoritmo:         SHA256withRSA\n"));
    assert!(!unknown.contains("Perfil"), "{unknown}");
}

#[test]
fn the_sheets_of_vv_are_separated_by_a_blank_line() {
    let mut signer = a_signature("UNA PERSONA", "", None);
    signer.countersignatures = vec![a_signature("OTRA PERSONA", "", None)];
    let second = a_signature("TERCERA PERSONA", "", None);

    let output = verbose(
        &["verify", "-i", "firmado.pdf", "-vv"],
        vec![signer, second],
    );

    assert_eq!(
        output,
        "\
PAdES · 2 firmas · 1 contrafirma

✓ UNA PERSONA
  Firmante:          UNA PERSONA
  Emisor:            AC FNMT Usuarios

    ✓ OTRA PERSONA
      Firmante:          OTRA PERSONA
      Emisor:            AC FNMT Usuarios

✓ TERCERA PERSONA
  Firmante:          TERCERA PERSONA
  Emisor:            AC FNMT Usuarios
"
    );
}

#[test]
fn the_sheets_of_vvv_are_separated_by_a_blank_line() {
    let output = verbose(
        &["verify", "-i", "firmado.pdf", "-vvv"],
        vec![
            a_signature("UNA PERSONA", "", None),
            a_signature("OTRA PERSONA", "", None),
        ],
    );

    assert_eq!(
        output,
        "\
PAdES · 2 firmas

✓ UNA PERSONA
  Firmante:          UNA PERSONA
  Emisor:            AC FNMT Usuarios
  Número de serie:   0123ABCD

✓ OTRA PERSONA
  Firmante:          OTRA PERSONA
  Emisor:            AC FNMT Usuarios
  Número de serie:   0123ABCD
"
    );
}
