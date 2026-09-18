use super::*;

#[test]
fn known_identifier_opens_its_destination() {
    let mut opened = None;
    let result = open_destination("discussions", |url| {
        opened = Some(url.to_string());
        Ok(())
    });

    assert_eq!(result, Ok(()));
    assert_eq!(
        opened,
        Some("https://github.com/sgomez/rfirma/discussions".to_string())
    );
}

#[test]
fn unknown_identifier_is_rejected_without_opening() {
    let result = open_destination("unknown", |_| panic!("no deberia llamarse al abridor"));

    assert_eq!(
        result,
        Err(DestinationError::UnknownTarget("unknown".to_string()))
    );
}

#[test]
fn url_cannot_be_passed_as_destination() {
    let result = open_destination("https://github.com/sgomez/rfirma/discussions", |_| {
        panic!("no deberia llamarse al abridor")
    });

    assert_eq!(
        result,
        Err(DestinationError::UnknownTarget(
            "https://github.com/sgomez/rfirma/discussions".to_string()
        ))
    );
}

#[test]
fn opener_failure_is_propagated() {
    let result = open_destination("discussions", |_| Err("fallo de red".to_string()));

    assert_eq!(
        result,
        Err(DestinationError::Failed("fallo de red".to_string()))
    );
}
