use super::*;
use serde::Deserialize;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Remembered {
    answer: u32,
}

fn a_file(directory: &Path) -> JsonFile<Remembered> {
    JsonFile::at(directory.join("rfirma/config.json"))
}

#[test]
fn a_support_that_is_not_there_yet_gives_the_defaults_without_a_notice() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());

    let loaded = file.load().expect("deberia leerse");

    assert_eq!(loaded.value(), &Remembered::default());
    assert!(loaded.recovery().is_none());
}

#[test]
fn what_is_saved_comes_back_and_carries_the_format_version() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());

    file.save(&Remembered { answer: 42 })
        .expect("deberia escribirse");

    assert_eq!(
        file.load().expect("deberia leerse").into_value(),
        Remembered { answer: 42 }
    );
    let written: Value =
        serde_json::from_slice(&fs::read(file.path()).expect("deberia leerse el fichero"))
            .expect("deberia ser JSON");
    assert_eq!(written["version"], Value::from(FORMAT_VERSION));
}

#[test]
fn a_corrupt_support_is_set_aside_as_bak_and_the_application_still_starts() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());
    fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
        .expect("deberia crearse");
    fs::write(file.path(), b"{esto no es JSON").expect("deberia escribirse");

    let loaded = file
        .load()
        .expect("una preferencia corrupta no puede ser un fallo");

    assert_eq!(loaded.value(), &Remembered::default());
    let recovery = loaded.recovery().expect("deberia avisar una vez");
    assert!(matches!(recovery.damage(), Damage::Unparsable(_)));
    assert_eq!(
        recovery.backup(),
        Some(directory.path().join("rfirma/config.json.bak").as_path())
    );
    assert!(
        recovery
            .backup()
            .expect("deberia haberse apartado")
            .exists(),
        "lo que habia se conserva en el .bak"
    );
    assert!(
        !file.path().exists(),
        "el fichero roto ya no esta en su sitio"
    );
}

#[test]
fn a_corrupt_support_that_cannot_even_be_set_aside_still_lets_the_application_start() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());
    fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
        .expect("deberia crearse");
    fs::write(file.path(), b"{esto tampoco es JSON").expect("deberia escribirse");
    fs::create_dir(directory.path().join("rfirma/config.json.bak")).expect("deberia crearse");

    let loaded = file
        .load()
        .expect("no poder apartar lo roto tampoco puede impedir arrancar");

    assert_eq!(loaded.value(), &Remembered::default());
    let recovery = loaded.recovery().expect("deberia avisar igualmente");
    assert!(matches!(recovery.damage(), Damage::Unparsable(_)));
    assert_eq!(
        recovery.backup(),
        None,
        "no se pudo apartar, y el aviso lo dice"
    );
    assert!(file.path().exists(), "lo roto sigue donde estaba");
}

#[test]
fn a_support_from_an_unknown_version_is_set_aside_instead_of_interpreted() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());
    fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
        .expect("deberia crearse");
    fs::write(file.path(), br#"{"version": 99, "answer": 7}"#).expect("deberia escribirse");

    let loaded = file
        .load()
        .expect("una version desconocida no puede ser un fallo");

    assert_eq!(loaded.value(), &Remembered::default());
    assert_eq!(
        loaded.recovery().map(Recovery::damage),
        Some(&Damage::UnknownVersion(Some(99)))
    );
}

#[test]
fn a_support_without_a_version_is_set_aside_too() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());
    fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
        .expect("deberia crearse");
    fs::write(file.path(), br#"{"answer": 7}"#).expect("deberia escribirse");

    let loaded = file.load().expect("deberia leerse");

    assert_eq!(
        loaded.recovery().map(Recovery::damage),
        Some(&Damage::UnknownVersion(None))
    );
}

#[test]
fn a_failed_write_leaves_the_previous_content_intact_and_no_temporary_behind() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());
    file.save(&Remembered { answer: 1 })
        .expect("deberia escribirse");
    let taken = directory.path().join("rfirma/otro.json");
    fs::create_dir(&taken).expect("deberia crearse el directorio");
    let blocked: JsonFile<Remembered> = JsonFile::at(&taken);

    let error = blocked
        .save(&Remembered { answer: 2 })
        .expect_err("deberia fallar al escribir");

    assert_eq!(error.situation(), Situation::Unwritable);
    assert!(!directory.path().join("rfirma/otro.json.tmp").exists());
    assert_eq!(
        file.load().expect("deberia leerse").into_value(),
        Remembered { answer: 1 }
    );
}

#[test]
fn a_support_that_exists_but_cannot_be_read_is_a_failure_and_not_the_defaults() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let taken = directory.path().join("config.json");
    fs::create_dir(&taken).expect("deberia crearse el directorio");

    let error = JsonFile::<Remembered>::at(&taken)
        .load()
        .expect_err("un soporte ilegible no puede pasar por primer arranque");

    assert_eq!(error.situation(), Situation::Unreadable);
    assert!(error.detail().contains("config.json"));
}

#[test]
fn erasing_removes_the_support_and_does_not_mind_it_being_gone_already() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let file = a_file(directory.path());
    file.save(&Remembered { answer: 3 })
        .expect("deberia escribirse");

    file.erase().expect("deberia borrarse");

    assert!(!file.path().exists());
    file.erase()
        .expect("borrar lo que ya no esta no es un fallo");
}
