//! Comprobación y comparación de versiones nuevas publicadas (ADR-0015).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::signing::application::state::VersionCheck;
use crate::Memory;

/// Puerto de red que obtiene el cuerpo de la última publicación.
pub type ReleaseFeed<'a> = &'a dyn Fn() -> Option<String>;

/// Periodo de validez de la comprobación de versión en caché.
pub const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

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
    memory: &Memory,
    feed: ReleaseFeed<'_>,
    now: SystemTime,
) -> Option<Version> {
    let announced = match fresh_answer(memory, now) {
        Some(cached) => cached,
        None => ask_and_remember(memory, feed, now)?,
    };

    (announced > running).then_some(announced)
}

/// Lee la comprobación previa de la memoria si no ha caducado.
fn fresh_answer(memory: &Memory, now: SystemTime) -> Option<Version> {
    let check = memory
        .state()
        .ok()?
        .into_value()
        .version_check
        .filter(|check| {
            seconds_since_epoch(now).saturating_sub(check.checked_at) < CACHE_TTL.as_secs()
        })?;

    Version::parse(&check.announced)
}

/// Consulta el puerto de red y persiste la comprobación si es válida.
fn ask_and_remember(memory: &Memory, feed: ReleaseFeed<'_>, now: SystemTime) -> Option<Version> {
    let announced = announced_version(&feed()?)?;

    let _ = memory.remember_version_check(VersionCheck {
        checked_at: seconds_since_epoch(now),
        announced: announced.to_string(),
    });

    Some(announced)
}

/// Extrae la versión anunciada en el cuerpo de la publicación.
fn announced_version(body: &str) -> Option<Version> {
    let release: serde_json::Value = serde_json::from_str(body).ok()?;
    Version::parse(release.get("tag_name")?.as_str()?)
}

/// Convierte una marca temporal a segundos desde el inicio de época Unix.
fn seconds_since_epoch(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
