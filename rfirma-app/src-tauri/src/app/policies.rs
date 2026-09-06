//! Expansión y combinación de políticas de firma declaradas por la sede (ADR-0016).

use std::collections::BTreeMap;

use crate::ffi::BridgeError;
use crate::protocol::{pairs_of, PADES};
use crate::signing::to_java_properties;

/// Expansor de la política de firma declarada por la sede.
pub trait PolicyEngine {
    /// Expande las propiedades de política de firma en formato Java Properties.
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError>;
}

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
