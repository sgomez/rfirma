use super::*;
use std::io::ErrorKind;
use std::path::PathBuf;

#[test]
fn a_missing_folder_names_the_path_it_could_not_find() {
    let error =
        DestinationError::about(Situation::FolderMissing, &PathBuf::from("/home/quien/Docs"));

    assert_eq!(error.situation(), Situation::FolderMissing);
    assert_eq!(error.detail(), "/home/quien/Docs");
    assert!(error.to_string().contains("FolderMissing"));
}

#[test]
fn an_unreadable_folder_drags_the_system_error_along() {
    let error = DestinationError::caused_by(
        Situation::FolderUnreadable,
        &PathBuf::from("/mnt/red/Docs"),
        &std::io::Error::new(ErrorKind::PermissionDenied, "denegado"),
    );

    assert!(error.detail().contains("/mnt/red/Docs"));
    assert!(error.detail().contains("denegado"));
}
