use super::*;

#[test]
fn every_destination_and_rubric_situation_has_a_camel_case_name() {
    let names = [
        DestinationSituation::FolderMissing,
        DestinationSituation::NotAFolder,
        DestinationSituation::FolderUnreadable,
        DestinationSituation::NoFreeName,
    ]
    .map(|situation| destination_told(situation).0)
    .into_iter()
    .chain(
        [
            RubricSituation::NotAnAcceptedImage,
            RubricSituation::DamagedImage,
            RubricSituation::ImageTooLarge,
            RubricSituation::SourceUnreadable,
            RubricSituation::StoreUnwritable,
            RubricSituation::StoreUnreadable,
        ]
        .map(|situation| rubric_told(situation).0),
    );
    for name in names {
        assert!(
            !name.contains('_') && name.chars().next().is_some_and(char::is_lowercase),
            "«{name}» no está en camelCase"
        );
    }
}

#[test]
fn a_document_that_is_no_longer_open_is_unreadable_for_the_window_and_for_the_site() {
    let (failure, code) = document_told(&DocumentError::no_longer_open());

    assert_eq!(failure.situation, "documentUnreadable");
    assert_eq!(code, SafCode::CannotReadData);
}

#[test]
fn a_dropped_file_that_is_not_a_pdf_is_told_like_any_other_non_pdf() {
    assert_eq!(Failure::from(DropRefusal::NotAPdf).situation, "notAPdf");
    assert_eq!(
        Failure::from(DropRefusal::Unreadable("no such file".to_owned())).situation,
        "droppedFileUnreadable"
    );
}
