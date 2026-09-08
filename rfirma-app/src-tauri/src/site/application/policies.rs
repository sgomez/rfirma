//! Expansión y combinación de políticas de firma declaradas por la sede (ADR-0016).

use std::collections::BTreeMap;

use crate::signing::domain::bridge::{BridgeError, Format};
use crate::signing::domain::to_java_properties;
use crate::site::domain::protocol::pairs_of;
use crate::site::ports::PolicyEngine;

/// Caso de uso: expande los parámetros adicionales declarados por la sede.
pub fn expanded_for_the_site<E: PolicyEngine>(
    engine: &E,
    declared: &[(String, String)],
    format: Format,
) -> Result<BTreeMap<String, String>, BridgeError> {
    let block = to_java_properties(&declared.iter().cloned().collect());
    let expanded = engine.expand(&block, format.name())?;
    Ok(pairs_of(&expanded).into_iter().collect())
}

#[cfg(test)]
mod tests;
