use std::cell::Cell;
use std::time::{Duration, SystemTime};

use super::{install_new_version, new_version, NewVersion, Version};
use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::installation::{InstallFailure, Installation};
use crate::desktop::domain::version_check::VersionCheck;
use crate::desktop::ports::UpdateInstaller;
use crate::signing::application::tests::a_memory;

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
        Channel::Native,
        at(1_756_000_000),
    );

    assert_eq!(
        newer.map(|new| new.version.to_string()),
        Some("0.4.0".into())
    );
}

fn a_latest_json(version: &str) -> String {
    format!(
        r#"{{"version":"{version}","notes":"","pub_date":"2026-09-30T00:00:00Z","platforms":{{"windows-x86_64":{{"signature":"sig","url":"https://rfirma.sgomez.me/windows/rFirma.exe"}}}}}}"#
    )
}

#[test]
fn on_windows_the_feed_is_read_as_latest_json_and_the_version_is_installable() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());

    let newer = new_version(
        Version::parse("0.3.1").expect("es una version"),
        &memory,
        &|| Some(a_latest_json("0.4.0")),
        Channel::Windows,
        at(1_756_000_000),
    );

    assert_eq!(
        newer,
        Some(NewVersion {
            version: Version::parse("0.4.0").expect("es una version"),
            installable: true,
        })
    );
}

#[test]
fn on_linux_the_version_is_announced_but_not_installable() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());

    for channel in [Channel::Native, Channel::Flatpak] {
        let newer = new_version(
            Version::parse("0.3.1").expect("es una version"),
            &memory,
            &|| Some(a_release("v0.4.0")),
            channel,
            at(1_756_000_000),
        );

        assert_eq!(newer.map(|new| new.installable), Some(false));
    }
}

#[test]
fn each_channel_reads_only_its_own_feed_format() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let running = Version::parse("0.3.1").expect("es una version");

    assert_eq!(
        new_version(
            running,
            &memory,
            &|| Some(a_release("v0.4.0")),
            Channel::Windows,
            at(1)
        ),
        None
    );
    assert_eq!(
        new_version(
            running,
            &memory,
            &|| Some(a_latest_json("0.4.0")),
            Channel::Native,
            at(2)
        ),
        None
    );
    assert_eq!(
        memory
            .state()
            .expect("deberia leerse")
            .into_value()
            .version_check,
        None,
        "un feed que no es el del canal no se recuerda"
    );
}

#[test]
fn the_same_version_or_an_older_one_is_not_announced() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.0").expect("es una version");

    assert_eq!(
        new_version(
            running,
            &memory,
            &|| Some(a_release("v0.4.0")),
            Channel::Native,
            at(1_000)
        ),
        None,
        "la que se esta ejecutando no es una version nueva"
    );
    assert_eq!(
        new_version(
            running,
            &memory,
            &|| Some(a_release("v0.3.9")),
            Channel::Native,
            at(2_000)
        ),
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
        Channel::Native,
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
fn the_feed_is_asked_even_when_the_last_answer_was_a_moment_ago() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_version_check(VersionCheck {
            checked_at: 1_756_000_000,
            announced: "0.4.0".to_string(),
        })
        .expect("deberia anotarse");
    let asked = Cell::new(false);

    let announced = new_version(
        Version::parse("0.3.0").expect("es una version"),
        &memory,
        &|| {
            asked.set(true);
            Some(a_release("v0.5.0"))
        },
        Channel::Native,
        at(1_756_000_001),
    );

    assert!(
        asked.get(),
        "se pregunta a la fuente de publicaciones aunque la ultima respuesta sea reciente"
    );
    assert_eq!(
        announced.map(|new| new.version.to_string()),
        Some("0.5.0".into())
    );
}

#[test]
fn without_a_response_the_last_known_version_is_used() {
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
        &|| None,
        Channel::Native,
        at(1_756_000_100),
    );

    assert_eq!(
        announced.map(|new| new.version.to_string()),
        Some("0.4.0".into())
    );
}

#[test]
fn an_invalid_response_does_not_overwrite_the_last_known_version() {
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
        &|| Some("<html>502 Bad Gateway</html>".to_string()),
        Channel::Native,
        at(1_756_000_100),
    );

    assert_eq!(
        announced.map(|new| new.version.to_string()),
        Some("0.4.0".into()),
        "una respuesta invalida no sobrescribe la ultima conocida"
    );
    assert_eq!(
        memory
            .state()
            .expect("deberia leerse")
            .into_value()
            .version_check,
        Some(VersionCheck {
            checked_at: 1_756_000_000,
            announced: "0.4.0".to_string(),
        }),
        "la comprobacion anterior sigue intacta"
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
        Channel::Native,
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
        Channel::Native,
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

struct FakeInstaller {
    announced: Result<Option<&'static str>, InstallFailure>,
    install: Result<(), InstallFailure>,
    installed: Cell<bool>,
}

impl FakeInstaller {
    fn announcing(
        announced: Result<Option<&'static str>, InstallFailure>,
        install: Result<(), InstallFailure>,
    ) -> Self {
        Self {
            announced,
            install,
            installed: Cell::new(false),
        }
    }
}

impl UpdateInstaller for FakeInstaller {
    fn announced(&self) -> Result<Option<String>, InstallFailure> {
        self.announced.map(|announced| announced.map(str::to_owned))
    }

    fn install(&self) -> Result<(), InstallFailure> {
        self.install?;
        self.installed.set(true);
        Ok(())
    }
}

fn running() -> Version {
    Version::parse("0.3.1").expect("es una version")
}

#[test]
fn a_newer_announced_version_is_installed() {
    let installer = FakeInstaller::announcing(Ok(Some("0.4.0")), Ok(()));

    assert_eq!(
        install_new_version(running(), &installer),
        Installation::Installed
    );
    assert!(installer.installed.get());
}

#[test]
fn without_a_newer_version_nothing_is_installed() {
    for announced in [
        None,
        Some("0.3.1"),
        Some("0.2.9"),
        Some("no es una version"),
    ] {
        let installer = FakeInstaller::announcing(Ok(announced), Ok(()));

        assert_eq!(
            install_new_version(running(), &installer),
            Installation::NoUpdate
        );
        assert!(!installer.installed.get());
    }
}

#[test]
fn a_network_failure_leaves_the_installation_untouched() {
    for installer in [
        FakeInstaller::announcing(Err(InstallFailure::Network), Ok(())),
        FakeInstaller::announcing(Ok(Some("0.4.0")), Err(InstallFailure::Network)),
    ] {
        assert_eq!(
            install_new_version(running(), &installer),
            Installation::NetworkFailure
        );
        assert!(!installer.installed.get());
    }
}

#[test]
fn an_invalid_signature_leaves_the_installation_untouched() {
    let installer =
        FakeInstaller::announcing(Ok(Some("0.4.0")), Err(InstallFailure::InvalidSignature));

    assert_eq!(
        install_new_version(running(), &installer),
        Installation::InvalidSignature
    );
    assert!(!installer.installed.get());
}

#[test]
fn on_linux_installing_is_not_available() {
    let installer = FakeInstaller::announcing(
        Err(InstallFailure::NotAvailable),
        Err(InstallFailure::NotAvailable),
    );

    assert_eq!(
        install_new_version(running(), &installer),
        Installation::NotAvailable
    );
    assert!(!installer.installed.get());
}
