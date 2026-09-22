//! El registro de los bugs conocidos de AutoFirma 1.9.2, las fichas del anexo A1 con su estado en
//! `master`; no dice qué comprobación incumple cada uno, eso lo declara el catálogo.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Si el bug sigue vivo en la rama de desarrollo del original.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum InMaster {
    Fixed,
    Present,
    Partial,
}

/// Una ficha `BUG-NN` del anexo A1: su título, sin marcas de Markdown, y su estado en `master`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct KnownBug {
    pub id: String,
    pub title: String,
    pub master: InMaster,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    bug: Vec<KnownBug>,
}

static THE_KNOWN_BUGS: LazyLock<Vec<KnownBug>> = LazyLock::new(|| {
    the_known_bugs_in(include_str!("../bugs/autofirma-1.9.2.toml"))
        .unwrap_or_else(|complaint| panic!("bugs/autofirma-1.9.2.toml: {complaint}"))
});

pub fn the_known_bugs() -> &'static [KnownBug] {
    &THE_KNOWN_BUGS
}

pub(crate) fn the_known_bug(id: &str) -> Option<&'static KnownBug> {
    the_known_bugs().iter().find(|bug| bug.id == id)
}

fn the_known_bugs_in(raw: &str) -> Result<Vec<KnownBug>, String> {
    let registry: Registry = toml::from_str(raw).map_err(|error| error.to_string())?;
    let mut ids: Vec<&str> = registry.bug.iter().map(|bug| bug.id.as_str()).collect();
    ids.sort_unstable();
    match ids.windows(2).find(|pair| pair[0] == pair[1]) {
        Some(pair) => Err(format!("{} está repetido", pair[0])),
        None => Ok(registry.bug),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_reads_and_finds_a_bug_by_its_id() {
        let bug = the_known_bug("BUG-15").unwrap();

        assert_eq!(bug.master, InMaster::Present);
        assert!(the_known_bug("BUG-99").is_none());
    }

    #[test]
    fn a_repeated_bug_is_refused() {
        let raw = "[[bug]]\nid = \"BUG-01\"\ntitle = \"a\"\nmaster = \"fixed\"\n\n\
                   [[bug]]\nid = \"BUG-01\"\ntitle = \"b\"\nmaster = \"present\"\n";

        assert_eq!(the_known_bugs_in(raw).unwrap_err(), "BUG-01 está repetido");
    }

    #[test]
    fn a_state_in_master_outside_the_vocabulary_is_refused() {
        let raw = "[[bug]]\nid = \"BUG-01\"\ntitle = \"a\"\nmaster = \"gone\"\n";

        assert!(the_known_bugs_in(raw).is_err());
    }
}
