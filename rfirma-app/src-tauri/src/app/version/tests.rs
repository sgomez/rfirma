use std::time::{Duration, SystemTime};

use super::{new_version, Version, CACHE_TTL};
use crate::app::fixtures::a_memory;
use crate::memory::VersionCheck;

fn a_release(tag: &str) -> String {
    format!(r#"{{"tag_name":"{tag}","name":"rFirma {tag}"}}"#)
}

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn a_newer_published_version_is_announced() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());

    let newer = new_version(
        Version::parse("0.3.1").expect("es una version"),
        &memory,
        &|| Some(a_release("v0.4.0")),
        at(1_756_000_000),
    );

    assert_eq!(
        newer.map(|version| version.to_string()),
        Some("0.4.0".into())
    );
}

#[test]
fn the_same_version_or_an_older_one_is_not_announced() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.0").expect("es una version");

    assert_eq!(
        new_version(running, &memory, &|| Some(a_release("v0.4.0")), at(1_000)),
        None,
        "la que se esta ejecutando no es una version nueva"
    );
    assert_eq!(
        new_version(running, &memory, &|| Some(a_release("v0.3.9")), at(2_000)),
        None,
        "una publicacion mas vieja tampoco"
    );
}

#[test]
fn without_network_there_is_silence_and_the_cache_is_left_untouched() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());

    let nothing = new_version(
        Version::parse("0.1.0").expect("es una version"),
        &memory,
        &|| None,
        at(1_756_000_000),
    );

    assert_eq!(nothing, None);
    assert_eq!(
        memory
            .state()
            .expect("deberia leerse")
            .into_value()
            .version_check,
        None,
        "sin respuesta no se anota nada: el siguiente arranque vuelve a preguntar"
    );
}

#[test]
fn within_twenty_four_hours_the_port_is_not_asked_again() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_version_check(VersionCheck {
            checked_at: 1_756_000_000,
            announced: "0.4.0".to_string(),
        })
        .expect("deberia anotarse");

    let announced = new_version(
        Version::parse("0.3.0").expect("es una version"),
        &memory,
        &|| panic!("no se puede salir a la red antes de que caduque la cache"),
        at(1_756_000_000 + CACHE_TTL.as_secs() - 1),
    );

    assert_eq!(
        announced.map(|version| version.to_string()),
        Some("0.4.0".into())
    );
}

#[test]
fn after_twenty_four_hours_the_port_is_asked_again() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_version_check(VersionCheck {
            checked_at: 1_756_000_000,
            announced: "0.4.0".to_string(),
        })
        .expect("deberia anotarse");
    let later = at(1_756_000_000 + CACHE_TTL.as_secs());

    let announced = new_version(
        Version::parse("0.3.0").expect("es una version"),
        &memory,
        &|| Some(a_release("v0.5.0")),
        later,
    );

    assert_eq!(
        announced.map(|version| version.to_string()),
        Some("0.5.0".into())
    );
    assert_eq!(
        memory
            .state()
            .expect("deberia leerse")
            .into_value()
            .version_check,
        Some(VersionCheck {
            checked_at: 1_756_000_000 + CACHE_TTL.as_secs(),
            announced: "0.5.0".to_string(),
        }),
        "lo preguntado se anota para las proximas 24 h"
    );
}

#[test]
fn a_release_candidate_tag_is_not_a_version_to_announce() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());

    let announced = new_version(
        Version::parse("0.3.0").expect("es una version"),
        &memory,
        &|| Some(a_release("v0.4.0-rc.1")),
        at(1_756_000_000),
    );

    assert_eq!(announced, None);
}

#[test]
fn an_answer_that_is_not_a_release_is_silence_and_is_not_remembered() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());

    let announced = new_version(
        Version::parse("0.3.0").expect("es una version"),
        &memory,
        &|| Some("<html>502 Bad Gateway</html>".to_string()),
        at(1_756_000_000),
    );

    assert_eq!(announced, None);
    assert_eq!(
        memory
            .state()
            .expect("deberia leerse")
            .into_value()
            .version_check,
        None
    );
}

#[test]
fn versions_are_compared_as_numbers_and_not_as_text() {
    let older = Version::parse("0.9.9").expect("es una version");
    let newer = Version::parse("0.10.0").expect("es una version");

    assert!(newer > older);
    assert_eq!(Version::parse("v1.2.3"), Version::parse("1.2.3"));
    assert_eq!(Version::parse("1.2"), None);
    assert_eq!(Version::parse("1.2.3.4"), None);
}

#[test]
fn the_running_version_comes_from_the_package() {
    assert_eq!(
        Version::running().to_string(),
        env!("CARGO_PKG_VERSION"),
        "la version del paquete es la que se compara"
    );
}
