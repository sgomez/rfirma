use super::*;
use std::fs;

#[test]
fn the_marker_present_means_the_flatpak_channel() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let marker = directory.path().join(".flatpak-info");
    fs::write(&marker, b"[Application]\n").expect("deberia escribirse");

    assert_eq!(Channel::over(&marker), Channel::Flatpak);
}

#[test]
fn no_marker_means_the_native_channel() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    assert_eq!(
        Channel::over(&directory.path().join(".flatpak-info")),
        Channel::Native
    );
}

#[test]
fn the_real_question_is_asked_over_the_well_known_marker() {
    assert_eq!(SANDBOX_MARKER, "/.flatpak-info");
}

#[test]
fn inside_the_sandbox_there_is_no_answer_at_all() {
    let handlers = registered_handlers_for_scheme(Channel::Flatpak, "afirma");

    assert_eq!(handlers, RegisteredHandlers::NotAvailableInsideTheSandbox);
}

#[test]
fn outside_the_sandbox_the_answer_comes_from_the_desktop() {
    let handlers = registered_handlers_for_scheme(Channel::Native, "afirma");

    assert!(matches!(handlers, RegisteredHandlers::Known(_)));
}

#[test]
fn our_desktop_file_is_the_one_the_bundler_installs() {
    let configuration: serde_json::Value =
        serde_json::from_str(include_str!("../../../../tauri.conf.json"))
            .expect("tauri.conf.json deberia ser JSON");
    let product = configuration["productName"]
        .as_str()
        .expect("tauri.conf.json deberia declarar productName");

    assert_eq!(OUR_DESKTOP_FILE, format!("{product}.desktop"));
}

#[test]
fn a_handler_carries_both_the_name_and_the_desktop_file() {
    let handler = RegisteredHandler::new("AutoFirma", "autofirma.desktop");

    assert_eq!(handler.name(), "AutoFirma");
    assert_eq!(handler.id(), "autofirma.desktop");
}

#[test]
fn every_offered_handler_can_be_written_as_a_default() {
    let RegisteredHandlers::Known(handlers) =
        registered_handlers_for_scheme(Channel::Native, "http")
    else {
        panic!("fuera del sandbox tiene que haber lista");
    };

    assert!(handlers.iter().all(|handler| !handler.id().is_empty()));
}
