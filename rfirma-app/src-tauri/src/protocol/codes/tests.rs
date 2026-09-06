use super::*;
use std::collections::BTreeSet;

#[test]
fn the_catalogue_is_the_fifty_three_published_codes_and_nothing_else() {
    let literals: BTreeSet<&str> = SafCode::ALL.iter().map(|code| code.as_str()).collect();

    assert_eq!(literals.len(), 53, "hay codigos repetidos en el catalogo");
    let expected: BTreeSet<String> = (0..53).map(|number| format!("SAF_{number:02}")).collect();
    let expected: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(literals, expected);
}

#[test]
fn each_variant_sits_on_the_row_of_its_own_number() {
    for (number, code) in SafCode::ALL.iter().enumerate() {
        assert_eq!(*code as usize, number, "{code:?} no esta en su sitio");
        assert_eq!(code.as_str(), format!("SAF_{number:02}"));
    }
}

#[test]
fn every_code_carries_a_phrase_of_ours_in_plain_ascii() {
    for code in SafCode::ALL {
        let phrase = code.phrase();
        assert!(!phrase.is_empty(), "{code:?} no tiene frase");
        assert!(phrase.is_ascii(), "«{phrase}» no es ASCII");
        assert!(
            !phrase.ends_with('.'),
            "«{phrase}» acaba en punto y el parametro se nombra detras"
        );
    }
}

#[test]
fn every_refusal_travels_as_a_line_the_published_client_recognises() {
    for code in SafCode::ALL {
        let line = WireAnswer::refused(code).on_the_wire();

        assert!(line.starts_with("SAF_"), "«{line}» no la lee como error");
        assert!(
            line.len() > 4,
            "«{line}» tiene cuatro caracteres y no es un error para el cliente"
        );
    }
}

#[test]
fn a_bad_parameter_is_named_behind_the_code() {
    let line = WireAnswer::refused_because_of(SafCode::Params, Parameter::Ports).on_the_wire();

    assert_eq!(
        line,
        "SAF_03: Error en los parametros de entrada; el parametro que falla es 'ports'"
    );
}

#[test]
fn the_three_answers_that_are_not_codes_travel_bare() {
    assert_eq!(WireAnswer::Cancelled.on_the_wire(), "CANCEL");
    assert_eq!(WireAnswer::OutOfMemory.on_the_wire(), "MEMORY_ERROR");
    assert_eq!(WireAnswer::Nothing.on_the_wire(), "NULL");
}

#[test]
fn every_parameter_is_named_as_the_protocol_names_it() {
    for parameter in Parameter::ALL {
        let name = parameter.name();
        assert!(!name.is_empty());
        assert!(
            name.chars().all(|letter| letter.is_ascii_lowercase()),
            "«{name}» no es un nombre de parametro del protocolo"
        );
    }
}
