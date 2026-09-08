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

/// Transcrito de `JSONBatchManager.buildBatchResultJson` de 1.9.2: un elemento
/// por firma, con las claves en el orden en el que el original las pone.
const LOCAL_RESULT_OF_THREE: &[u8] = b"{\"signs\":[\
{\"id\":\"001\",\"result\":\"DONE_AND_SAVED\",\"signature\":\"bGEgZmlybWE=\"},\
{\"id\":\"002\",\"result\":\"SKIPPED\"},\
{\"id\":\"003\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"}]}";

#[test]
fn build_local_result_writes_the_three_outcomes_of_the_original() {
    let results = vec![
        LocalBatchResult::signed("001", b"la firma".to_vec()),
        LocalBatchResult::skipped("002"),
        LocalBatchResult::failed("003", "fallo"),
    ];

    assert_eq!(build_local_result(&results), LOCAL_RESULT_OF_THREE);
}

#[test]
fn a_result_that_is_skipped_afterwards_loses_its_signature() {
    let mut result = LocalBatchResult::signed("001", b"la firma".to_vec());

    result.skip();

    assert_eq!(result.result(), PresignResult::Skipped);
    assert_eq!(result.signature(), None);
}
