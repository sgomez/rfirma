use super::super::presign::{BatchDataResult, PresignResult};
use super::*;

/// Capturado de `JSONBatchInfoParser.buildResult` de 1.9.2 (`~/.m2`).
const RESULT_WITH_ONE_ERROR: &[u8] =
    b"{\"signs\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"}]}";

#[test]
fn build_result_matches_the_original_byte_for_byte() {
    let errors = vec![BatchDataResult::new(
        "002",
        PresignResult::ErrorPre,
        Some("fallo".to_owned()),
    )];

    assert_eq!(build_result(&errors), RESULT_WITH_ONE_ERROR);
}

#[test]
fn build_result_omits_the_description_when_there_is_none() {
    let errors = vec![BatchDataResult::new("002", PresignResult::ErrorPre, None)];

    let result = build_result(&errors);

    assert_eq!(
        result,
        b"{\"signs\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\"}]}"
    );
}

/// Capturado de `JSONBatchInfoParser.buildEmptyResult` de 1.9.2 (`~/.m2`).
const EMPTY_RESULT: &[u8] = b"{\"signs\":[]}";

#[test]
fn build_empty_result_matches_the_original_byte_for_byte() {
    assert_eq!(build_empty_result(), EMPTY_RESULT);
}
