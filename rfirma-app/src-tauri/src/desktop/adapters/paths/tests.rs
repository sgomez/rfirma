use super::*;
use std::collections::HashMap;

fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), OsString::from(*value)))
        .collect();
    move |name: &str| map.get(name).cloned()
}

fn resolve(platform: Platform, pairs: &[(&str, &str)]) -> Paths {
    Paths::resolve(platform, &environment(pairs)).expect("deberia resolverse")
}

#[test]
fn the_configuration_home_has_no_application_directory_behind_it() {
    let home = xdg_config_home(&environment(&[
        ("HOME", "/home/quien"),
        ("XDG_CONFIG_HOME", "/home/quien/.config"),
    ]))
    .expect("deberia resolverse");

    assert_eq!(home, PathBuf::from("/home/quien/.config"));
}

#[test]
fn without_the_variable_the_configuration_home_falls_back_under_home() {
    let home =
        xdg_config_home(&environment(&[("HOME", "/home/quien")])).expect("deberia resolverse");

    assert_eq!(home, PathBuf::from("/home/quien/.config"));
}

#[test]
fn linux_splits_configuration_and_state_across_two_xdg_directories() {
    let paths = resolve(
        Platform::Linux,
        &[
            ("HOME", "/home/quien"),
            ("XDG_CONFIG_HOME", "/home/quien/.config"),
            ("XDG_STATE_HOME", "/home/quien/.local/state"),
            ("XDG_DATA_HOME", "/home/quien/.local/share"),
        ],
    );

    assert_eq!(
        paths.config_file(),
        PathBuf::from("/home/quien/.config/rfirma/config.json")
    );
    assert_eq!(
        paths.state_file(),
        PathBuf::from("/home/quien/.local/state/rfirma/state.json")
    );
    assert_eq!(
        paths.rubric_path(),
        PathBuf::from("/home/quien/.local/share/rfirma/rubric.jpg")
    );
}

#[test]
fn linux_falls_back_to_the_xdg_defaults_under_home() {
    let paths = resolve(Platform::Linux, &[("HOME", "/home/quien")]);

    assert_eq!(
        paths.config_file(),
        PathBuf::from("/home/quien/.config/rfirma/config.json")
    );
    assert_eq!(
        paths.state_file(),
        PathBuf::from("/home/quien/.local/state/rfirma/state.json")
    );
}

#[test]
fn a_relative_xdg_variable_is_ignored_instead_of_writing_next_to_the_cwd() {
    let paths = resolve(
        Platform::Linux,
        &[("HOME", "/home/quien"), ("XDG_CONFIG_HOME", ".config")],
    );

    assert_eq!(
        paths.config_file(),
        PathBuf::from("/home/quien/.config/rfirma/config.json")
    );
}

#[test]
fn windows_keeps_the_state_out_of_the_roaming_profile() {
    let paths = resolve(
        Platform::Windows,
        &[
            ("APPDATA", r"C:\Users\quien\AppData\Roaming"),
            ("LOCALAPPDATA", r"C:\Users\quien\AppData\Local"),
        ],
    );

    assert_eq!(
        paths.config_file(),
        PathBuf::from(r"C:\Users\quien\AppData\Roaming").join("rfirma/config.json")
    );
    assert_eq!(
        paths.state_file(),
        PathBuf::from(r"C:\Users\quien\AppData\Local").join("rfirma/state.json")
    );
    assert_eq!(
        paths.rubric_path(),
        PathBuf::from(r"C:\Users\quien\AppData\Roaming").join("rfirma/rubric.jpg")
    );
}

#[test]
fn macos_collapses_the_split_into_two_files_in_one_directory() {
    let paths = resolve(Platform::MacOs, &[("HOME", "/Users/quien")]);

    assert_eq!(
        paths.config_file().parent(),
        paths.state_file().parent(),
        "en macOS los dos ficheros comparten directorio"
    );
    assert_ne!(paths.config_file(), paths.state_file());
    assert_eq!(
        paths.config_file(),
        PathBuf::from("/Users/quien/Library/Application Support/rfirma/config.json")
    );
}

#[test]
fn an_environment_without_a_home_is_a_failure_naming_the_variable() {
    let error = Paths::resolve(Platform::Linux, &environment(&[]))
        .expect_err("sin HOME no deberia resolverse");

    assert_eq!(error.variable(), "HOME");
    assert!(error.to_string().contains("HOME"));
}

#[test]
fn windows_without_the_local_profile_is_a_failure_naming_it() {
    let error = Paths::resolve(
        Platform::Windows,
        &environment(&[("APPDATA", r"C:\Users\quien\AppData\Roaming")]),
    )
    .expect_err("sin LOCALAPPDATA no deberia resolverse");

    assert_eq!(error.variable(), "LOCALAPPDATA");
}

#[test]
fn the_three_memories_never_share_a_file() {
    for platform in [Platform::Linux, Platform::Windows, Platform::MacOs] {
        let paths = resolve(
            platform,
            &[
                ("HOME", "/home/quien"),
                ("APPDATA", "/roaming"),
                ("LOCALAPPDATA", "/local"),
            ],
        );
        let files = [paths.config_file(), paths.state_file(), paths.rubric_path()];
        for (index, file) in files.iter().enumerate() {
            assert!(
                !files[index + 1..].contains(file),
                "{platform:?} repite {}",
                file.display()
            );
        }
    }
}

#[test]
fn resolving_a_path_does_not_create_anything_on_disk() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let paths = Paths::under(directory.path());

    assert!(!paths.config_file().exists());
    assert!(!paths.state_file().exists());
    assert!(!paths.rubric_path().exists());
}

#[test]
fn the_test_root_keeps_configuration_and_state_apart() {
    let paths = Paths::under("/tmp/prueba");

    assert_ne!(paths.config_file().parent(), paths.state_file().parent());
}

#[test]
fn the_documents_folder_follows_the_xdg_variable_when_the_system_localises_it() {
    let documents = documents_folder_of(
        Platform::Linux,
        &environment(&[
            ("HOME", "/home/quien"),
            ("XDG_DOCUMENTS_DIR", "/home/quien/Documentos"),
        ]),
    )
    .expect("deberia resolverse");

    assert_eq!(documents, PathBuf::from("/home/quien/Documentos"));
}

#[test]
fn without_the_xdg_variable_the_documents_folder_is_the_english_default() {
    let documents = documents_folder_of(Platform::Linux, &environment(&[("HOME", "/home/quien")]))
        .expect("deberia resolverse");

    assert_eq!(documents, PathBuf::from("/home/quien/Documents"));
}

#[test]
fn the_other_two_systems_hang_the_documents_folder_off_their_own_profile() {
    let windows = documents_folder_of(
        Platform::Windows,
        &environment(&[("USERPROFILE", r"C:\Users\quien")]),
    )
    .expect("deberia resolverse");
    let macos = documents_folder_of(Platform::MacOs, &environment(&[("HOME", "/Users/quien")]))
        .expect("deberia resolverse");

    assert_eq!(windows, PathBuf::from(r"C:\Users\quien").join("Documents"));
    assert_eq!(macos, PathBuf::from("/Users/quien/Documents"));
}

#[test]
fn resolving_the_documents_folder_does_not_create_it() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let home = directory.path().join("quien");

    let documents = documents_folder_of(
        Platform::Linux,
        &environment(&[("HOME", &home.to_string_lossy())]),
    )
    .expect("deberia resolverse");

    assert!(
        !documents.exists(),
        "resolver una ruta no toca el disco, y la de destino no se crea nunca (ADR-0011)"
    );
}

#[cfg(unix)]
#[test]
fn restricting_leaves_the_directory_and_the_file_only_for_their_owner() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let inside = directory.path().join("rfirma");
    std::fs::create_dir(&inside).expect("deberia crearse");
    let file = inside.join("state.json");
    std::fs::write(&file, b"{}").expect("deberia escribirse");

    restrict_to_owner(&inside).expect("deberia poder restringirse");
    restrict_to_owner(&file).expect("deberia poder restringirse");

    let mode = |path: &Path| {
        std::fs::metadata(path)
            .expect("deberia leerse")
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode(&inside), 0o700);
    assert_eq!(mode(&file), 0o600);
}

#[cfg(unix)]
#[test]
fn the_private_key_file_is_born_unreadable_for_anyone_else() {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let key = directory.path().join("local-ca.key.pem");

    let mut file = create_owner_only_file(&key).expect("deberia crearse");
    file.write_all(b"-----BEGIN PRIVATE KEY-----")
        .expect("deberia escribirse");

    assert_eq!(
        std::fs::metadata(&key)
            .expect("deberia leerse")
            .permissions()
            .mode()
            & 0o777,
        0o600,
        "el modo va en el `open`, no en un `chmod` posterior (ADR-0005)"
    );
}

#[test]
fn the_local_ca_lives_in_the_data_directory_and_the_server_certificate_nowhere() {
    let paths = Paths::under("/tmp/raiz");

    assert_eq!(
        paths.local_ca_certificate_path(),
        PathBuf::from("/tmp/raiz/data/rfirma/local-ca.crt.pem")
    );
    assert_eq!(
        paths.local_ca_key_path(),
        PathBuf::from("/tmp/raiz/data/rfirma/local-ca.key.pem")
    );
}
