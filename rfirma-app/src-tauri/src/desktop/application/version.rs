//! Comprobación de versiones nuevas publicadas y su instalación, solo si es mayor que la que corre (ADR-0015).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::installation::Installation;
use crate::desktop::domain::version_check::VersionCheck;
use crate::desktop::ports::{UpdateInstaller, VersionMemory};

/// Puerto de red que obtiene el cuerpo de la última publicación.
pub type ReleaseFeed<'a> = &'a dyn Fn() -> Option<String>;

/// Versión semántica de tres componentes numéricos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// Versión en ejecución leída de la compilación del paquete.
    pub fn running() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION"))
            .expect("la version del paquete es mayor.menor.parche")
    }

    /// Parsea una versión desde una cadena con formato semver.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let mut numbers = text.strip_prefix('v').unwrap_or(text).split('.');
        let mut next = || numbers.next()?.parse::<u64>().ok();
        let (major, minor, patch) = (next()?, next()?, next()?);
        if numbers.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Comprueba si existe una versión publicada posterior a la en ejecución.
pub fn new_version(
    running: Version,
    memory: &dyn VersionMemory,
    feed: ReleaseFeed<'_>,
    channel: Channel,
    now: SystemTime,
) -> Option<NewVersion> {
    let announced =
        ask_and_remember(memory, feed, channel, now).or_else(|| remembered_answer(memory))?;

    (announced > running).then(|| NewVersion {
        version: announced,
        installable: channel.installs_from_the_app(),
    })
}

/// Versión nueva publicada y si se puede instalar desde la aplicación.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewVersion {
    /// La versión anunciada.
    pub version: Version,
    /// Si el canal permite instalarla desde la aplicación.
    pub installable: bool,
}

/// Instala la versión anunciada si es mayor que la que corre; si no, no toca la instalación.
pub fn install_new_version(running: Version, installer: &dyn UpdateInstaller) -> Installation {
    let announced = match installer.announced() {
        Ok(announced) => announced.as_deref().and_then(Version::parse),
        Err(failure) => return failure.into(),
    };
    if !announced.is_some_and(|announced| announced > running) {
        return Installation::NoUpdate;
    }
    match installer.install() {
        Ok(()) => Installation::Installed,
        Err(failure) => failure.into(),
    }
}

/// Lee la última comprobación guardada en memoria, sea cual sea su antigüedad.
pub(crate) fn remembered_answer(memory: &dyn VersionMemory) -> Option<Version> {
    Version::parse(&memory.last_version_check()?.announced)
}

/// Consulta el puerto de red y persiste la comprobación si es válida.
pub(crate) fn ask_and_remember(
    memory: &dyn VersionMemory,
    feed: ReleaseFeed<'_>,
    channel: Channel,
    now: SystemTime,
) -> Option<Version> {
    let announced = announced_version(&feed()?, channel)?;

    let _ = memory.remember_version_check(VersionCheck {
        checked_at: seconds_since_epoch(now),
        announced: announced.to_string(),
    });

    Some(announced)
}

/// Extrae la versión anunciada: `version` del `latest.json` en Windows, `tag_name` en GitHub.
fn announced_version(body: &str, channel: Channel) -> Option<Version> {
    let field = match channel {
        Channel::Windows => "version",
        Channel::Native | Channel::Flatpak => "tag_name",
    };
    let feed: serde_json::Value = serde_json::from_str(body).ok()?;
    Version::parse(feed.get(field)?.as_str()?)
}

/// Convierte una marca temporal a segundos desde el inicio de época Unix.
fn seconds_since_epoch(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
