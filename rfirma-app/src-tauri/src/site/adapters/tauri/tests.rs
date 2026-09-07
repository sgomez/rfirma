use super::*;
use tauri_plugin_dialog::FilePath;

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
