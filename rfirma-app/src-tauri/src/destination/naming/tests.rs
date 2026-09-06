use super::*;

#[test]
fn the_signed_document_takes_the_suffix_before_the_extension() {
    assert_eq!(signed_name("contrato.pdf"), "contrato-firmado.pdf");
    assert_eq!(signed_name("acta 2024.PDF"), "acta 2024-firmado.PDF");
}

#[test]
fn a_name_that_already_ends_in_the_suffix_does_not_take_a_second_one() {
    assert_eq!(signed_name("contrato-firmado.pdf"), "contrato-firmado.pdf");
    assert_eq!(signed_name("contrato-FIRMADO.pdf"), "contrato-FIRMADO.pdf");
}

#[test]
fn an_accented_name_is_signed_and_does_not_panic() {
    assert_eq!(signed_name("árbitros.pdf"), "árbitros-firmado.pdf");
    assert_eq!(signed_name("1ª parte.pdf"), "1ª parte-firmado.pdf");
    assert_eq!(signed_name("ñ.pdf"), "ñ-firmado.pdf");
    assert_eq!(
        signed_name("Acuerdo Marco-firmado.pdf"),
        "Acuerdo Marco-firmado.pdf"
    );
}

#[test]
fn the_suffix_does_not_stack_on_a_name_that_already_carries_a_number() {
    assert_eq!(
        signed_name("contrato-firmado-2.pdf"),
        "contrato-firmado.pdf"
    );
    assert_eq!(
        signed_name("contrato-FIRMADO-9.pdf"),
        "contrato-FIRMADO.pdf"
    );
}

#[test]
fn a_number_that_is_part_of_the_original_name_survives() {
    assert_eq!(signed_name("informe-2.pdf"), "informe-2-firmado.pdf");
    assert_eq!(signed_name("anexo-2b.pdf"), "anexo-2b-firmado.pdf");
    assert_eq!(signed_name("acta-.pdf"), "acta--firmado.pdf");
}

#[test]
fn a_name_without_extension_keeps_not_having_one() {
    assert_eq!(signed_name("informe"), "informe-firmado");
}

#[test]
fn a_dotfile_is_all_stem_and_not_an_extension_without_a_name() {
    assert_eq!(signed_name(".oculto"), ".oculto-firmado");
}

#[test]
fn a_document_without_a_usable_name_still_gets_one() {
    assert_eq!(signed_name(""), "documento-firmado");
}

#[test]
fn the_number_goes_after_the_suffix_and_before_the_extension() {
    assert_eq!(
        numbered("contrato-firmado.pdf", FIRST_NUMBER),
        "contrato-firmado-2.pdf"
    );
    assert_eq!(numbered("informe-firmado", 3), "informe-firmado-3");
}

#[test]
fn the_first_namesake_is_the_second_file_because_the_first_carries_no_number() {
    assert_eq!(FIRST_NUMBER, 2);
}
