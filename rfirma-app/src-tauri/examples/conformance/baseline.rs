//! El perfil del cliente, que solo decide cómo se lanza, y los nombres de los resultados; no
//! juzga nada.

use serde::{Deserialize, Deserializer, Serialize};

use crate::dossier::Verdict;

/// El perfil del cliente: qué binario, qué envoltorio y qué raíz de confianza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Profile {
    Autofirma,
    Rfirma,
}

impl Profile {
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "autofirma" => Some(Self::Autofirma),
            "rfirma" => Some(Self::Rfirma),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Autofirma => "autofirma",
            Self::Rfirma => "rfirma",
        }
    }
}

pub(crate) fn the_verdict_named<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Verdict, D::Error> {
    let name = String::deserialize(deserializer)?;
    verdict_of(&name).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "«{name}» no es un resultado: conforme, no-conforme o no-observable"
        ))
    })
}

pub(crate) fn verdict_of(name: &str) -> Option<Verdict> {
    match name {
        "conforme" => Some(Verdict::Compliant),
        "no-conforme" => Some(Verdict::Noncompliant),
        "no-observable" => Some(Verdict::NotObservable),
        _ => None,
    }
}

pub(crate) fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Compliant => "CONFORME",
        Verdict::Noncompliant => "NO CONFORME",
        Verdict::NotObservable => "NO OBSERVABLE",
    }
}

pub(crate) const PENDING_NAME: &str = "PENDIENTE";
