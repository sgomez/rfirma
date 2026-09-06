use super::*;

#[test]
fn a_bridge_that_does_not_open_is_a_bridge_error_and_not_a_thread_error() {
    let isolate = Isolate::start_with(|| Err(BridgeError::Failed("no hay libreria".to_owned())));

    let refused = FilterEngine::select(&isolate, "", "").expect_err("el puente no abre");
    assert!(matches!(refused, BridgeError::Failed(_)), "{refused:?}");

    let refused = PolicyEngine::expand(&isolate, "", "pades").expect_err("el puente no abre");
    assert!(matches!(refused, BridgeError::Failed(_)), "{refused:?}");
}
