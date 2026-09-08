use super::*;
use tauri_plugin_dialog::FilePath;

use crate::site::adapters::scratch::RealScratch;
use crate::site::application::errand::{
    ErrandStep, LiveErrand, LoadCompletion, Moment, SiteOutcome, SiteRefusal,
};

fn a_consent() -> crate::site::application::errand::SavingConsent {
    crate::site::application::errand::SavingConsent {
        data: b"contenido".to_vec(),
        title: None,
        filename: None,
        extensions: Vec::new(),
        description: None,
        starting_folder: None,
        signer_der: None,
    }
}

#[test]
fn each_chosen_path_keeps_its_base_name_and_its_full_path() {
    let chosen = vec![
        FilePath::Path("/home/persona/uno.pdf".into()),
        FilePath::Path("/home/persona/carpeta/dos.pdf".into()),
    ];

    let named = named_paths(chosen).expect("son rutas de disco");

    assert_eq!(named[0].0, "uno.pdf");
    assert_eq!(
        named[0].1,
        std::path::PathBuf::from("/home/persona/uno.pdf")
    );
    assert_eq!(named[1].0, "dos.pdf");
}

#[test]
fn a_url_that_is_not_a_file_path_is_the_only_way_named_paths_fails() {
    let chosen = vec![FilePath::Url(
        "https://example.org/x".parse().expect("es una url"),
    )];

    let failed = named_paths(chosen);

    assert!(failed.is_err());
}

/// `end()` no limpia `moment`: guarda por qué `site_save_file` no republica el que había tras
/// esta función, o la ventana volvería a ver el momento de un trámite ya cerrado.
#[test]
fn a_finished_save_leaves_the_stale_moment_untouched_for_the_caller_to_not_republish() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = LiveErrand::default();
    live.note(Moment::Saving {
        filename: Some("factura.pdf".to_owned()),
    });

    let shown = write_where_chosen(
        Some(FilePath::Path(home.path().join("destino.pdf"))),
        &a_consent(),
        &RealScratch,
        &live,
    )
    .expect("se ha podido escribir");

    assert!(shown, "un guardado sin firma en cola se enseña como tal");
    assert_eq!(
        live.moment(),
        Some(Moment::Saving {
            filename: Some("factura.pdf".to_owned())
        }),
        "sigue siendo el momento anterior a cerrar el tramite: publicarlo otra vez sería mostrar \
         una ventana que ya no describe nada vivo"
    );
}

#[test]
fn a_step_that_continues_the_errand_carries_it_through() {
    let (step, delivered) = told_of_loading(LoadCompletion::Continues(ErrandStep::Saving(
        Box::new(a_consent()),
    )))
    .expect("el tramite sigue, no hay rechazo");

    assert!(matches!(step, Some(ErrandStep::Saving(_))));
    assert!(delivered.is_none());
}

#[test]
fn an_unreadable_file_keeps_its_own_situation() {
    let failure = told_of_loading(LoadCompletion::Delivered(SiteOutcome::Refused(
        SiteRefusal::CannotLoadData("no such file".to_owned()),
    )))
    .expect_err("no hay nada que entregar a la ventana");

    assert_eq!(failure.situation, "cannotLoadData");
    assert_eq!(failure.detail, "no such file");
}

#[test]
fn any_other_refusal_already_delivered_to_the_site_still_reaches_the_window() {
    let failure = told_of_loading(LoadCompletion::Delivered(SiteOutcome::Refused(
        SiteRefusal::NoCertificateTheSiteAccepts,
    )))
    .expect_err("la sede ya lo tiene, pero la ventana no debe quedarse esperando");

    assert_eq!(failure.situation, "certificateNotFound");
}

#[test]
fn loaded_files_are_counted_for_the_window() {
    let (step, delivered) = told_of_loading(LoadCompletion::Delivered(SiteOutcome::Loaded(vec![
        ("uno.pdf".to_owned(), b"x".to_vec()),
        ("dos.pdf".to_owned(), b"y".to_vec()),
    ])))
    .expect("se han entregado dos ficheros");

    assert!(step.is_none());
    assert_eq!(delivered, Some(2));
}

#[test]
fn a_cancellation_has_nothing_to_show_and_nothing_to_reject() {
    let (step, delivered) =
        told_of_loading(LoadCompletion::Delivered(SiteOutcome::Cancelled)).expect("no es rechazo");

    assert!(step.is_none());
    assert!(delivered.is_none());
}
