//! Los fallos del material del canal **se clasifican, no se traducen**
//! (ADR-0009), igual que los de la rúbrica en [`crate::rubric::error`].
//!
//! Son tres situaciones y no más, porque desde fuera solo hay tres remedios
//! distintos: no se ha podido fabricar el material, no se ha podido guardar, y
//! lo que había guardado ya no se entiende. Esta última no se confunde con
//! «no había nada»: un `$HOME` sin CA local es el primer arranque y no un
//! fallo, así que la ausencia se dice con `None` y no con un error.

use std::fmt;

/// Situación que la persona puede entender, y que el catálogo traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// No se ha podido generar la CA local o el certificado del servidor
    /// local. Es OpenSSL diciendo que no, y no depende de nada que la persona
    /// haya elegido.
    MaterialNotGenerated,
    /// La CA local no se ha podido escribir en el directorio de datos.
    MaterialUnwritable,
    /// Hay una CA local guardada, pero no se puede leer o ya no es un
    /// certificado con su clave. Se rehace: es material que la aplicación sí
    /// puede volver a fabricar, al precio de volver a registrarla.
    MaterialDamaged,
}

/// Un fallo del material del canal: la situación traducible y el detalle crudo.
///
/// [`TlsError::detail`] nunca está vacío y **nunca** está traducido: es lo que
/// se pega en un informe de fallo. El mensaje que ve la persona lo compone el
/// catálogo de cadenas a partir de [`TlsError::situation`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsError {
    situation: Situation,
    detail: String,
}

impl TlsError {
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

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for TlsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = TlsError::new(Situation::MaterialDamaged, "PEM routines::no start line");

        assert_eq!(error.situation(), Situation::MaterialDamaged);
        assert_eq!(error.detail(), "PEM routines::no start line");
        assert!(error.to_string().contains("MaterialDamaged"));
        assert!(error.to_string().contains("no start line"));
    }
}
