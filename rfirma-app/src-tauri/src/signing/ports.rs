//! Puertos del contexto de firma: el puente, los dos motores que presta y el hilo que lo aloja.

use crate::signing::domain::bridge::{BridgeError, PostSignRequest, PreSignRequest, PreSignature};
use crate::signing::domain::isolate_gone::IsolateGone;

/// El puente nativo visto desde el ciclo: prefirma y postfirma, y ninguna entrada que firme (ADR-0001).
pub trait Bridge {
    /// Prefirma PAdES: los atributos que el token firmará y el sello de sesión.
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError>;

    /// Postfirma PAdES: el PDF firmado a partir de una prefirma ya sellada.
    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError>;
}

/// El hilo dueño del puente: corre una tarea con el puente delante y devuelve lo que salió (ADR-0003).
pub trait IsolateHost {
    /// Corre la tarea en el hilo del puente; `Err` fuera si el hilo murió, `Err` dentro si el puente no abre.
    fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce(&dyn Bridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone>;
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
