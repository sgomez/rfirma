use super::*;
use crate::documents::ports::DialogClues;

#[test]
fn unattached_adapter_reports_missing_app_handle() {
    let adapter = RealPortalDialogs::new();
    let clues = DialogClues::new();

    let pick_result = adapter.pick_file(&clues);
    assert!(pick_result.is_err());
    assert!(pick_result.unwrap_err().contains("no hay manejador"));

    let pick_files_result = adapter.pick_files(&clues);
    assert!(pick_files_result.is_err());
    assert!(pick_files_result.unwrap_err().contains("no hay manejador"));

    let save_result = adapter.save_file(&clues);
    assert!(save_result.is_err());
    assert!(save_result.unwrap_err().contains("no hay manejador"));
}
