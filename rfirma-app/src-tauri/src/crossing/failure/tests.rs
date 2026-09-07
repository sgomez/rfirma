use super::Failure;

#[test]
fn a_failure_crosses_with_its_situation_in_camel_case_and_no_attempts_by_default() {
    let failure = Failure::new("documentUnreadable", "el detalle crudo");

    assert_eq!(
        serde_json::to_string(&failure).expect("serializa"),
        r#"{"situation":"documentUnreadable","detail":"el detalle crudo","attemptsLeft":null}"#
    );
}
