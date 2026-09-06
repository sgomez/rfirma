use super::to_java_properties;
use std::collections::BTreeMap;

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn writes_one_line_per_entry_in_a_stable_order() {
    let block = to_java_properties(&params(&[
        ("signaturePage", "3"),
        ("signatureSubFilter", "ETSI.CAdES.detached"),
    ]));

    assert_eq!(
        block,
        "signaturePage=3\nsignatureSubFilter=ETSI.CAdES.detached\n"
    );
}

#[test]
fn writes_nothing_for_no_entries() {
    assert_eq!(to_java_properties(&BTreeMap::new()), "");
}

#[test]
fn folds_the_newlines_of_the_layer2_text_into_one_line() {
    let block = to_java_properties(&params(&[(
        "layer2Text",
        "Firmado por: ADA LOVELACE BYRON - ***9999**.\nMotivo: Conforme",
    )]));

    assert_eq!(
        block,
        "layer2Text=Firmado por: ADA LOVELACE BYRON - ***9999**.\\nMotivo: Conforme\n"
    );
    assert_eq!(block.lines().count(), 1);
}

#[test]
fn escapes_the_backslash_before_anything_else() {
    let block = to_java_properties(&params(&[("signReason", "C:\\nada")]));

    assert_eq!(block, "signReason=C:\\\\nada\n");
}

#[test]
fn escapes_the_carriage_return_too() {
    let block = to_java_properties(&params(&[("layer2Text", "uno\r\ndos")]));

    assert_eq!(block, "layer2Text=uno\\r\\ndos\n");
}

#[test]
fn writes_the_accents_as_ascii_escapes() {
    let block = to_java_properties(&params(&[("signReason", "Ratificación")]));

    assert_eq!(block, "signReason=Ratificaci\\u00F3n\n");
    assert!(block.is_ascii(), "el bloque tiene que ser ASCII puro");
}

#[test]
fn writes_a_character_outside_the_basic_plane_as_two_escapes() {
    let block = to_java_properties(&params(&[("signReason", "\u{1F58A}")]));

    assert_eq!(block, "signReason=\\uD83D\\uDD8A\n");
}

#[test]
fn leaves_the_base64_of_the_rubric_untouched() {
    let rubric = "/9j/4AAQSkZJRgABAQEAYABgAAD+abc=";
    let block = to_java_properties(&params(&[("signatureRubricImage", rubric)]));

    assert_eq!(block, format!("signatureRubricImage={rubric}\n"));
}

#[test]
fn escapes_the_separators_when_they_are_in_a_key() {
    let block = to_java_properties(&params(&[("una clave=rara", "valor")]));

    assert_eq!(block, "una\\ clave\\=rara=valor\n");
}

#[test]
fn leaves_the_separators_alone_when_they_are_in_a_value() {
    let block = to_java_properties(&params(&[("signReason", "a=b: c")]));

    assert_eq!(block, "signReason=a=b: c\n");
}
