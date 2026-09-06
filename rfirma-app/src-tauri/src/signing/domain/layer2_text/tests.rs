use super::{compose_layer2_text, mask_id_number, obfuscate_ids, VisibleTextFields};
use crate::signing::domain::language::Language;

fn all_fields<'a>() -> VisibleTextFields<'a> {
    VisibleTextFields {
        signer_name: Some("ADA LOVELACE BYRON - 99999999R"),
        issuer: Some("AC FNMT Usuarios"),
        signed_at: Some("31/08/2026 12:00:00 CEST"),
        reason: Some("Conforme"),
        pseudonym: false,
    }
}

#[test]
fn composes_one_paragraph_and_leaves_the_reason_on_its_own_line() {
    assert_eq!(
        compose_layer2_text(&all_fields(), Language::Spanish),
        "Firmado por: ADA LOVELACE BYRON - ***9999**. \
         Emisor: AC FNMT Usuarios. \
         Fecha: 31/08/2026 12:00:00 CEST.\n\
         Motivo: Conforme"
    );
}

#[test]
fn forces_no_line_break_other_than_the_one_before_the_reason() {
    let without_reason = VisibleTextFields {
        reason: None,
        ..all_fields()
    };
    assert!(!compose_layer2_text(&without_reason, Language::Spanish).contains('\n'));
    assert_eq!(
        compose_layer2_text(&all_fields(), Language::Spanish)
            .lines()
            .count(),
        2
    );
}

#[test]
fn composes_only_the_reason_when_it_is_the_only_box_checked() {
    let fields = VisibleTextFields {
        reason: Some("Conforme"),
        ..VisibleTextFields::default()
    };
    assert_eq!(
        compose_layer2_text(&fields, Language::Spanish),
        "Motivo: Conforme"
    );
}

#[test]
fn drops_the_unchecked_fields() {
    let fields = VisibleTextFields {
        signer_name: Some("ADA LOVELACE BYRON"),
        ..VisibleTextFields::default()
    };
    assert_eq!(
        compose_layer2_text(&fields, Language::Spanish),
        "Firmado por: ADA LOVELACE BYRON."
    );
}

#[test]
fn composes_nothing_when_no_field_is_checked() {
    assert_eq!(
        compose_layer2_text(&VisibleTextFields::default(), Language::Spanish),
        ""
    );
}

#[test]
fn never_emits_an_autofirma_wildcard() {
    for language in Language::ALL {
        let text = compose_layer2_text(&all_fields(), language);
        assert!(
            !text.contains("$$"),
            "el texto en {} lleva un comodín: {text}",
            language.tag()
        );
    }
}

#[test]
fn follows_the_language_of_the_application() {
    let fields = all_fields();
    let spanish = compose_layer2_text(&fields, Language::Spanish);
    for language in Language::ALL {
        let text = compose_layer2_text(&fields, language);
        assert!(
            text.contains("ADA LOVELACE BYRON"),
            "falta el titular en {}",
            language.tag()
        );
        if language != Language::Spanish {
            assert_ne!(text, spanish, "{} no traduce nada", language.tag());
        }
    }
}

#[test]
fn masks_the_id_inside_the_common_name_without_a_switch() {
    let fields = VisibleTextFields {
        signer_name: Some("ADA LOVELACE BYRON - 99999999R"),
        ..VisibleTextFields::default()
    };
    let text = compose_layer2_text(&fields, Language::Spanish);
    assert!(!text.contains("99999999R"), "el DNI sale en claro: {text}");
    assert!(text.contains("***9999**"), "{text}");
}

#[test]
fn a_pseudonym_certificate_is_exempt_from_the_mask() {
    let fields = VisibleTextFields {
        signer_name: Some("SEUDONIMO 99999999R"),
        pseudonym: true,
        ..VisibleTextFields::default()
    };
    assert_eq!(
        compose_layer2_text(&fields, Language::Spanish),
        "Firmado por: SEUDONIMO 99999999R."
    );
}

#[test]
fn masks_the_identifier_of_every_spanish_common_name() {
    assert_eq!(
        obfuscate_ids("ADA LOVELACE BYRON - 99999999R"),
        "ADA LOVELACE BYRON - ***9999**"
    );
    assert_eq!(
        obfuscate_ids("ADA LOVELACE BYRON - NIF 99999999R"),
        "ADA LOVELACE BYRON - NIF ***9999**"
    );
    assert_eq!(
        obfuscate_ids("X1234567L - EMPRESA EJEMPLO SL - A12345674"),
        "****4567* - EMPRESA EJEMPLO SL - ****4567*"
    );
    assert_eq!(
        obfuscate_ids("APELLIDO1 APELLIDO2, ADA (FIRMA)"),
        "APELLIDO1 APELLIDO2, ADA (FIRMA)"
    );
}

#[test]
fn leaves_alone_what_is_not_an_identifier() {
    assert_eq!(obfuscate_ids("ADA LOVELACE BYRON"), "ADA LOVELACE BYRON");
    assert_eq!(obfuscate_ids("600123456 y 2026"), "600123456 y 2026");
    assert_eq!(obfuscate_ids("ANDRÉS PEÑA"), "ANDRÉS PEÑA");
    assert_eq!(obfuscate_ids(""), "");
}

#[test]
fn masks_a_dni_like_autofirma_does() {
    assert_eq!(mask_id_number("99999999R"), "***9999**");
    assert_eq!(mask_id_number("12345678Z"), "***4567**");
}

#[test]
fn masks_a_nie_like_autofirma_does() {
    assert_eq!(mask_id_number("X1234567L"), "****4567*");
}

#[test]
fn shifts_the_mask_when_there_are_fewer_digits_than_positions() {
    assert_eq!(mask_id_number("12345"), "*2345");
    assert_eq!(mask_id_number("1234"), "1234");
}

#[test]
fn masks_from_the_back_when_there_are_fewer_digits_than_visible_positions() {
    assert_eq!(mask_id_number("AB123"), "*B123");
}

#[test]
fn masks_only_the_segment_that_holds_the_digits() {
    assert_eq!(mask_id_number("IDCES-99999999R"), "IDCES-***9999**");
    assert_eq!(mask_id_number("12345678-Z"), "***4567*-Z");
    assert_eq!(mask_id_number("99999999 R"), "***9999* R");
}

#[test]
fn keeps_the_digit_run_across_a_separator_like_java_does() {
    assert_eq!(mask_id_number("12-345"), "12-*45");
}

#[test]
fn counts_the_digits_of_the_identifier_and_not_those_of_the_whole_name() {
    assert_eq!(
        obfuscate_ids("ADA 12 LOVELACE 345 BYRON - 99999999R"),
        "ADA 12 LOVELACE 345 BYRON - ***9999**"
    );
}
