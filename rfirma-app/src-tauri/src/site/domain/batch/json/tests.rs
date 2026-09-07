use super::*;

#[test]
fn an_object_round_trips_keeping_the_order_of_the_document() {
    let text = r#"{"b":"2","a":"1","params":{"NEED_PRE":"true","PRE":"QUJD"}}"#;

    let value = Json::parse(text).expect("es JSON valido");

    assert_eq!(value.to_json_string(), text);
    assert_eq!(value.get("a").and_then(Json::as_str), Some("1"));
    assert_eq!(
        value
            .get("params")
            .and_then(Json::as_object)
            .map(|entries| entries.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>()),
        Some(vec!["NEED_PRE", "PRE"])
    );
}

#[test]
fn an_array_of_objects_round_trips() {
    let text = r#"[{"id":"001"},{"id":"002"}]"#;

    let value = Json::parse(text).expect("es JSON valido");

    assert_eq!(value.to_json_string(), text);
    assert_eq!(value.as_array().map(<[Json]>::len), Some(2));
}

#[test]
fn booleans_null_and_numbers_round_trip() {
    let text = r#"{"stoponerror":false,"concurrenttimeout":30,"nada":null}"#;

    let value = Json::parse(text).expect("es JSON valido");

    assert_eq!(value.to_json_string(), text);
    assert_eq!(
        value.get("stoponerror").and_then(Json::as_bool),
        Some(false)
    );
}

#[test]
fn escaped_characters_round_trip() {
    let text = r#"{"texto":"linea 1\nlinea 2 con \"comillas\""}"#;

    let value = Json::parse(text).expect("es JSON valido");

    assert_eq!(value.to_json_string(), text);
    assert_eq!(
        value.get("texto").and_then(Json::as_str),
        Some("linea 1\nlinea 2 con \"comillas\"")
    );
}

#[test]
fn removing_a_key_drops_it_and_keeps_the_rest_in_order() {
    let mut value = Json::parse(r#"{"a":"1","b":"2","c":"3"}"#).expect("es JSON valido");

    let removed = value.remove("b");

    assert_eq!(removed, Some(Json::String("2".to_owned())));
    assert_eq!(value.to_json_string(), r#"{"a":"1","c":"3"}"#);
}

#[test]
fn setting_a_new_key_appends_it_at_the_end() {
    let mut value = Json::parse(r#"{"a":"1"}"#).expect("es JSON valido");

    value.set("b", Json::String("2".to_owned()));
    value.set("a", Json::String("nuevo".to_owned()));

    assert_eq!(value.to_json_string(), r#"{"a":"nuevo","b":"2"}"#);
}

#[test]
fn malformed_json_is_rejected_without_panicking() {
    assert!(Json::parse("{").is_err());
    assert!(Json::parse("{\"a\":}").is_err());
    assert!(Json::parse("no es json").is_err());
}

#[test]
fn every_string_escape_of_the_grammar_round_trips_or_is_read_correctly() {
    assert_eq!(
        Json::parse(r#""a\/b""#).expect("barra escapada").as_str(),
        Some("a/b")
    );
    assert_eq!(
        Json::parse(r#""a\bb""#)
            .expect("retroceso escapado")
            .as_str(),
        Some("a\u{8}b")
    );
    assert_eq!(
        Json::parse(r#""a\fb""#)
            .expect("salto de pagina escapado")
            .as_str(),
        Some("a\u{c}b")
    );
    assert_eq!(
        Json::parse(r#""café""#).expect("escape unicode").as_str(),
        Some("café")
    );
}

#[test]
fn a_multi_byte_character_survives_unescaped() {
    let value = Json::parse("\"café\"").expect("una cadena UTF-8 sin escapar");

    assert_eq!(value.as_str(), Some("café"));
    assert_eq!(value.to_json_string(), "\"café\"");
}

#[test]
fn an_unterminated_string_is_rejected() {
    assert!(Json::parse("\"sin cerrar").is_err());
}

#[test]
fn an_incomplete_unicode_escape_is_rejected() {
    assert!(Json::parse(r#""\u00""#).is_err());
}

#[test]
fn an_unknown_escape_is_rejected() {
    assert!(Json::parse(r#""\x""#).is_err());
}
