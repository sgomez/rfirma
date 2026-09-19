//! Resultado de retirar lo que rFirma dejó fuera de sus carpetas: el manejador y cada almacén.

use super::status::StoreBrand;

/// Qué pasó al retirar algo propio de rFirma de un sitio del sistema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Withdrawal {
    /// Estaba y se ha retirado.
    Withdrawn,
    /// No estaba, no había nada que retirar.
    WasNotThere,
    /// No se ha podido retirar.
    Failed(String),
}

/// Un almacén NSS con el resultado de retirar de él la CA local de rFirma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreWithdrawal {
    /// Marca del almacén.
    pub brand: StoreBrand,
    /// Resultado de la retirada en este almacén.
    pub outcome: Withdrawal,
}
