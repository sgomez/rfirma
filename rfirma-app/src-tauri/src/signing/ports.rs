//! Puertos del contexto de firma: los dos motores que presta el puente.

use crate::signing::adapters::ffi::BridgeError;

/// Interfaz para evaluar filtros de certificados contra el motor de filtrado.
pub trait FilterEngine {
    /// Devuelve los índices de los certificados que cumplen los criterios.
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError>;
}

/// Expansor de la política de firma declarada por la sede.
pub trait PolicyEngine {
    /// Expande las propiedades de política de firma en formato Java Properties.
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError>;
}
