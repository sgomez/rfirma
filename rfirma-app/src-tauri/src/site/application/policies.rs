//! Expansión y combinación de políticas de firma declaradas por la sede (ADR-0016).

use std::collections::BTreeMap;

use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::to_java_properties;
use crate::site::domain::protocol::{pairs_of, PADES};
use crate::site::ports::PolicyEngine;

/// Caso de uso: expande los parámetros adicionales declarados por la sede.
pub fn expanded_for_the_site<E: PolicyEngine>(
    engine: &E,
    declared: &[(String, String)],
) -> Result<BTreeMap<String, String>, BridgeError> {
    let block = to_java_properties(&declared.iter().cloned().collect());
    let expanded = engine.expand(&block, PADES)?;
    Ok(pairs_of(&expanded).into_iter().collect())
}

#[cfg(test)]
mod tests;
