//! Lo que la firma de curva elíptica exige y la de RSA no: su mecanismo crudo y el reempaquetado de `r` y `s`.

use cryptoki::mechanism::MechanismType;
use openssl::bn::BigNum;
use openssl::ecdsa::EcdsaSig;

use crate::identity::domain::error::{Situation, TokenError};

/// El mecanismo que firma un resumen ya calculado, el único que ofrecen las ranuras sin el compuesto.
pub const OVER_A_DIGEST: MechanismType = MechanismType::ECDSA;

/// El `r || s` que devuelve PKCS#11, en el `SEQUENCE { INTEGER r, INTEGER s }` que esperan CMS y XAdES.
pub fn der_encoded(concatenated: &[u8]) -> Result<Vec<u8>, TokenError> {
    if concatenated.is_empty() || !concatenated.len().is_multiple_of(2) {
        return Err(unusable(format!(
            "el token ha devuelto una firma de {} bytes, y r||s ocupa un numero par de bytes que no es cero",
            concatenated.len()
        )));
    }

    let (r, s) = concatenated.split_at(concatenated.len() / 2);
    BigNum::from_slice(r)
        .and_then(|r| Ok((r, BigNum::from_slice(s)?)))
        .and_then(|(r, s)| EcdsaSig::from_private_components(r, s))
        .and_then(|signature| signature.to_der())
        .map_err(|error| unusable(format!("r||s no se ha podido reempaquetar en DER: {error}")))
}

fn unusable(detail: String) -> TokenError {
    TokenError::new(Situation::Unknown, detail)
}

#[cfg(test)]
mod tests;
