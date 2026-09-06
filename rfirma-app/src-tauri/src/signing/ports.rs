//! Puertos del contexto de firma: el puente de la prefirma y la postfirma, y los dos motores que presta.

use crate::signing::domain::bridge::{BridgeError, PostSignRequest, PreSignRequest, PreSignature};

/// El puente nativo visto desde el ciclo: prefirma y postfirma, y ninguna entrada que firme (ADR-0001).
pub trait Bridge {
    /// Prefirma PAdES: los atributos que el token firmará y el sello de sesión.
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError>;

    /// Postfirma PAdES: el PDF firmado a partir de una prefirma ya sellada.
    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError>;
}

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
