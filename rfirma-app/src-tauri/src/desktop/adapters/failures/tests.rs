use super::*;

#[test]
fn every_desktop_situation_has_its_own_catalog_key() {
    let names = [
        situation_name(Situation::NotAvailableInsideTheSandbox),
        situation_name(Situation::TheListIsNotReadable),
        situation_name(Situation::TheListIsNotWritable),
    ];

    assert!(names.iter().all(|name| name.starts_with("handler")));
    assert_eq!(
        names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        names.len()
    );
}

#[test]
fn a_desktop_error_crosses_with_its_key_and_its_raw_detail() {
    let failure = Failure::from(DesktopError::new(
        Situation::TheListIsNotWritable,
        "Permission denied",
    ));

    assert_eq!(failure.situation, "handlerListUnwritable");
    assert_eq!(failure.detail, "Permission denied");
}
