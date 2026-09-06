//! Expansión y combinación de políticas de firma declaradas por la sede (ADR-0016).

use std::collections::BTreeMap;

use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::to_java_properties;
use crate::signing::ports::PolicyEngine;
use crate::site::domain::protocol::{pairs_of, PADES};

/// Caso de uso: expande los parámetros adicionales declarados por la sede.
pub fn expanded_for_the_site<E: PolicyEngine>(
    engine: &E,
    declared: &[(String, String)],
) -> Result<BTreeMap<String, String>, BridgeError> {
    let block = to_java_properties(&declared.iter().cloned().collect());
    let expanded = engine.expand(&block, PADES)?;
    Ok(pairs_of(&expanded).into_iter().collect())
}

/// Combina los parámetros de la sede con la configuración propia de rFirma.
pub fn merged_with(
    from_the_site: BTreeMap<String, String>,
    ours: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = from_the_site;
    merged.extend(ours);
    merged
}

#[cfg(test)]
mod tests;
