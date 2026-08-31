//! Los fallos de la rúbrica **se clasifican, no se traducen** (ADR-0009), igual
//! que los del token en [`crate::pkcs11::error`].
//!
//! El ADR-0012 cuenta **tres** fallos de la imagen —no es PNG ni JPEG, está
//! dañada, es demasiado grande— y ninguno más: el reescalado es silencioso
//! porque es la operación que el usuario habría pedido de todos modos. A esos
//! tres se suman [`Situation::SourceUnreadable`] y
//! [`Situation::StoreUnwritable`], que no hablan de la imagen sino del disco: el
//! ADR no los enumera porque no son fallos *de la rúbrica*, pero leer el
//! fichero elegido y escribir la copia pueden fallar, y desaparecer no es una
//! opción. Decir «no es PNG ni JPEG» de un fichero que no se ha podido ni leer
//! sería mentirle al usuario.

use std::fmt;

/// Situación que el usuario puede entender, y que el catálogo traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El fichero no es ni PNG ni JPEG. El mensaje dice qué formatos valen.
    NotAnAcceptedImage,
    /// Es PNG o JPEG, pero el decodificador no ha podido con él.
    DamagedImage,
    /// El fichero pasa del tope de entrada ([`super::MAX_INPUT_BYTES`]).
    ImageTooLarge,
    /// El fichero que eligió el usuario no se ha podido leer.
    SourceUnreadable,
    /// La rúbrica ya normalizada no se ha podido escribir en el almacén.
    StoreUnwritable,
}

/// Un fallo al preparar la rúbrica: la situación traducible y el detalle crudo.
///
/// [`RubricError::detail`] nunca está vacío y **nunca** está traducido: es lo
/// que se pega en un informe de fallo. El mensaje que ve el usuario lo compone
/// el catálogo de cadenas a partir de [`RubricError::situation`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RubricError {
    situation: Situation,
    detail: String,
}

impl RubricError {
    /// Un fallo con su detalle técnico, sin traducir.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// La situación que la interfaz enseña, ya clasificada.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// El detalle técnico crudo. Nunca vacío, nunca traducido.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RubricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for RubricError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = RubricError::new(Situation::DamagedImage, "invalid JPEG marker");

        assert_eq!(error.situation(), Situation::DamagedImage);
        assert_eq!(error.detail(), "invalid JPEG marker");
        assert!(error.to_string().contains("DamagedImage"));
        assert!(error.to_string().contains("invalid JPEG marker"));
    }
}
