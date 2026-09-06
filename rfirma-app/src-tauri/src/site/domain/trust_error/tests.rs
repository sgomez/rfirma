use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = TrustError::new(Situation::StoreUnreachable, "SECMOD_OpenUserDB");

    assert_eq!(error.situation(), Situation::StoreUnreachable);
    assert_eq!(error.detail(), "SECMOD_OpenUserDB");
    assert!(error.to_string().contains("StoreUnreachable"));
    assert!(error.to_string().contains("SECMOD_OpenUserDB"));
}
