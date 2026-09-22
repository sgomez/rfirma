//! El vocabulario que publica `driver.mjs --manifest`: sus modos y sus guiones, no cómo los corre.

use std::collections::BTreeMap;
use std::process::Command;

use serde::Deserialize;

use crate::errand::the_driver;

/// Quién escribe lo que llega al cliente (ADR-0027).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Site {
    Published,
    Handwritten,
}

/// El camino por el que un guion llega al cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Family {
    V4Echo,
    Service,
    EndToEnd,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Mode {
    pub bench_only: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Script {
    pub site: Site,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "la leerá el saludo por familia de trámite")
    )]
    pub family: Family,
    pub modes: Vec<String>,
    pub conditions: Vec<String>,
    pub bench_only: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub modes: BTreeMap<String, Mode>,
    pub scripts: BTreeMap<String, Script>,
}

impl Manifest {
    /// El manifiesto que publica `driver.mjs`, o por qué no se pudo leer.
    pub(crate) fn of_the_driver() -> Result<Self, String> {
        let driver = the_driver();
        let output = Command::new("node")
            .arg(&driver)
            .arg("--manifest")
            .output()
            .map_err(|error| format!("node no arrancó {}: {error}", driver.display()))?;
        if !output.status.success() {
            return Err(format!(
                "{} --manifest falló: {}",
                driver.display(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Self::from_json(&String::from_utf8_lossy(&output.stdout))
    }

    pub(crate) fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|error| format!("el manifiesto no se entiende: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_driver_publishes_a_manifest_that_reads() {
        let manifest = Manifest::of_the_driver().unwrap();
        let selection = &manifest.scripts["selectcert"];
        assert_eq!(selection.site, Site::Published);
        assert_eq!(selection.family, Family::EndToEnd);
        assert_eq!(manifest.scripts["protocol-v4"].family, Family::V4Echo);
        assert_eq!(manifest.scripts["protocol-service"].family, Family::Service);
        assert!(manifest.modes["relay"].bench_only);
    }

    #[test]
    fn a_manifest_with_a_family_outside_the_vocabulary_does_not_read() {
        let raw = r#"{"modes":{},"scripts":{"a":{"site":"published","family":"otra",
            "modes":[],"conditions":[],"bench_only":false}}}"#;
        assert!(Manifest::from_json(raw).is_err());
    }
}
