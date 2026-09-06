use super::*;

#[test]
fn a_refusal_keeps_its_untranslated_detail_next_to_the_code() {
    let refusal = Refusal::about(Parameter::Ports, "el parametro 'ports' no es numerico");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.detail(), "el parametro 'ports' no es numerico");
    assert!(refusal.to_string().starts_with("SAF_03: "));
}

#[test]
fn the_situation_of_a_refusal_changes_nothing_that_goes_out() {
    let plain = Refusal::about(Parameter::Properties, "'signaturePages=append'");
    let classified = plain
        .clone()
        .because(RefusalSituation::AppendedSignaturePage);

    assert_eq!(plain.situation(), RefusalSituation::Unknown);
    assert_eq!(
        classified.situation(),
        RefusalSituation::AppendedSignaturePage
    );
    assert_eq!(classified.answer(), plain.answer());
    assert_eq!(classified.detail(), plain.detail());
}

#[test]
fn the_untranslated_detail_is_not_part_of_what_goes_out() {
    let refusal = Refusal::about(Parameter::IdSession, "idsession='../../etc/passwd'");

    let line = refusal.answer().on_the_wire();

    assert!(!line.contains("passwd"), "«{line}» lleva el detalle crudo");
    assert!(line.ends_with("el parametro que falla es 'idsession'"));
}
