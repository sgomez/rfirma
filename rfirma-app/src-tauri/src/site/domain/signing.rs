//! Lo que vuelve de la firma que pidió la sede: la firma en memoria o el rechazo ya decidido.

use crate::site::domain::protocol::SafCode;

/// Firma de un trámite de sede lista para transmitir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteSignature {
    /// Bytes de la firma, en el formato que pidió la sede.
    pub signature: Vec<u8>,
    /// Certificado firmante en formato DER.
    pub signer_der: Vec<u8>,
}

/// Por qué la firma no ha salido, con el código para la sede y la situación para la ventana ya decididos por quien firma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SigningRefusal {
    /// El código que recibe la sede.
    pub code: SafCode,
    /// La situación con la que la ventana lo nombra.
    pub situation: String,
    /// El detalle crudo.
    pub detail: String,
    /// Los intentos de PIN que quedan, si el token los dijo.
    pub attempts_left: Option<u32>,
}
