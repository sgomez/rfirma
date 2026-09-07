use super::*;

#[test]
fn a_presign_with_signs_and_no_errors_parses_the_triphase_data() {
    let response =
        b"{\"td\":{\"format\":\"PAdES\",\"signinfo\":[{\"id\":\"001\",\"params\":{\"NEED_PRE\":\"true\",\"PRE\":\"MYICXDAYBgkqhkiG9w0BA=\"}}]}}";

    let outcome = parse_json_presign(response).expect("es un JSON de prefirma valido");

    let triphase_data = outcome.triphase_data().expect("hay firmas prefirmadas");
    assert_eq!(triphase_data.signs()[0].id(), Some("001"));
    assert!(outcome.errors().is_empty());
}

/// Capturado de `JSONPreSignBatchParser.parseFromJSON` de 1.9.2 (`~/.m2`).
const PRESIGN_WITH_ERRORS: &[u8] =
    b"{\"td\":{\"format\":\"PAdES\",\"signinfo\":[{\"id\":\"001\",\"params\":{\"NEED_PRE\":\"true\",\"PRE\":\"MYICXDAYBgkqhkiG9w0BA=\"}}]},\"results\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"}]}";

#[test]
fn a_presign_with_errors_keeps_both_the_signed_and_the_failed() {
    let outcome = parse_json_presign(PRESIGN_WITH_ERRORS).expect("es un JSON de prefirma valido");

    let triphase_data = outcome.triphase_data().expect("hay firmas prefirmadas");
    assert_eq!(triphase_data.signs()[0].id(), Some("001"));

    assert_eq!(outcome.errors().len(), 1);
    assert_eq!(outcome.errors()[0].id(), "002");
    assert_eq!(outcome.errors()[0].result(), PresignResult::ErrorPre);
    assert_eq!(outcome.errors()[0].description(), Some("fallo"));
}

#[test]
fn a_presign_without_td_nor_results_has_neither() {
    let outcome = parse_json_presign(b"{}").expect("un objeto vacio es JSON valido");

    assert!(outcome.triphase_data().is_none());
    assert!(outcome.errors().is_empty());
}

#[test]
fn an_unknown_result_state_is_rejected() {
    let response = b"{\"results\":[{\"id\":\"002\",\"result\":\"NO_EXISTE\"}]}";

    assert!(parse_json_presign(response).is_err());
}

/// Capturado de `JSONBatchInfo.updateResults` + `getInfoString` de 1.9.2 (`~/.m2`): el
/// `datareference`, `format`, `suboperation` y `extraparams` del firmante que fallo desaparecen,
/// y gana `result`/`description`; el resto del lote no se toca.
const BATCH_JSON: &[u8] =
    b"{\"singlesigns\":[{\"id\":\"002\",\"datareference\":\"AAAA\",\"format\":\"PAdES\",\"suboperation\":\"sign\",\"extraparams\":\"BBBB\"},{\"id\":\"003\",\"datareference\":\"CCCC\"}],\"algorithm\":\"SHA256\",\"stoponerror\":false}";

#[test]
fn updating_the_batch_with_errors_strips_the_failed_sign_and_leaves_the_rest() {
    let errors = vec![BatchDataResult::new(
        "002",
        PresignResult::ErrorPre,
        Some("fallo".to_owned()),
    )];

    let updated = update_batch_with_errors(BATCH_JSON, &errors).expect("el lote esta bien formado");

    assert_eq!(
        updated,
        b"{\"singlesigns\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"},\
          {\"id\":\"003\",\"datareference\":\"CCCC\"}],\"algorithm\":\"SHA256\",\"stoponerror\":false}"
    );
}

#[test]
fn updating_a_batch_without_singlesigns_is_rejected() {
    let result = update_batch_with_errors(b"{}", &[]);

    assert!(result.is_err());
}

#[test]
fn every_presign_result_state_maps_to_its_own_literal() {
    let states = [
        (PresignResult::NotStarted, "NOT_STARTED"),
        (PresignResult::DoneAndSaved, "DONE_AND_SAVED"),
        (PresignResult::DoneButNotSavedYet, "DONE_BUT_NOT_SAVED_YET"),
        (PresignResult::DoneButSaveSkipped, "DONE_BUT_SAVED_SKIPPED"),
        (PresignResult::DoneButErrorSaving, "DONE_BUT_ERROR_SAVING"),
        (PresignResult::ErrorPre, "ERROR_PRE"),
        (PresignResult::ErrorPost, "ERROR_POST"),
        (PresignResult::Skipped, "SKIPPED"),
        (PresignResult::SaveRollbacked, "SAVE_ROLLBACKED"),
    ];

    for (state, literal) in states {
        assert_eq!(state.as_str(), literal);
        let response = format!("{{\"results\":[{{\"id\":\"1\",\"result\":\"{literal}\"}}]}}");
        let outcome = parse_json_presign(response.as_bytes()).expect("es un literal valido");
        assert_eq!(outcome.errors()[0].result(), state);
    }
}
