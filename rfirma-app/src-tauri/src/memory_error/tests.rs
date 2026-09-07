use super::*;
use std::io::ErrorKind;
use std::path::PathBuf;

#[test]
fn a_failure_names_the_file_next_to_the_situation() {
    let error = MemoryError::about(
        Situation::Unwritable,
        &PathBuf::from("/x/state.json"),
        &std::io::Error::new(ErrorKind::PermissionDenied, "denegado"),
    );

    assert_eq!(error.situation(), Situation::Unwritable);
    assert!(error.detail().contains("/x/state.json"));
    assert!(error.to_string().contains("denegado"));
}
