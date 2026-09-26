use super::{mask_id_number, masked_signer, obfuscate_ids};

#[test]
fn masks_the_id_inside_the_signer_name() {
    let masked = masked_signer("ADA LOVELACE BYRON - 99999999R", false);
    assert!(!masked.contains("99999999R"), "{masked}");
    assert!(masked.contains("***9999**"), "{masked}");
}

#[test]
fn a_pseudonym_certificate_is_exempt_from_the_mask() {
    assert_eq!(
        masked_signer("SEUDONIMO 99999999R", true),
        "SEUDONIMO 99999999R"
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
