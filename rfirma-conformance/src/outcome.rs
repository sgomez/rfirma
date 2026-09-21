//! El resultado de una comprobación, sus nombres en pantalla y PENDIENTE; no decide cuál toca.

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

/// El resultado de una comprobación, que nunca pinta de verde a un cliente que se apartó del
/// protocolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Compliant,
    Noncompliant,
    NotObservable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Pending,
    Resolved(Outcome),
}

/// Los cuatro nombres en pantalla de un resultado, tal y como viajan a la consola.
#[derive(TS)]
#[ts(export)]
#[allow(dead_code)]
pub(crate) enum ResultName {
    #[ts(rename = "CONFORME")]
    Compliant,
    #[ts(rename = "NO CONFORME")]
    Noncompliant,
    #[ts(rename = "NO OBSERVABLE")]
    NotObservable,
    #[ts(rename = "PENDIENTE")]
    Pending,
}

pub(crate) const PENDING_NAME: &str = "PENDIENTE";

pub(crate) fn the_outcome_named<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Outcome, D::Error> {
    let name = String::deserialize(deserializer)?;
    outcome_of(&name).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "«{name}» no es un resultado: conforme, no-conforme o no-observable"
        ))
    })
}

pub(crate) fn outcome_of(name: &str) -> Option<Outcome> {
    match name {
        "conforme" => Some(Outcome::Compliant),
        "no-conforme" => Some(Outcome::Noncompliant),
        "no-observable" => Some(Outcome::NotObservable),
        _ => None,
    }
}

pub(crate) fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Compliant => "CONFORME",
        Outcome::Noncompliant => "NO CONFORME",
        Outcome::NotObservable => "NO OBSERVABLE",
    }
}

/// El nombre del resultado de una comprobación, que es PENDIENTE si el informe no la tiene.
pub(crate) fn result_name(state: Option<CheckState>) -> &'static str {
    match state {
        Some(CheckState::Resolved(outcome)) => outcome_name(outcome),
        _ => PENDING_NAME,
    }
}
