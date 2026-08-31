//! Los fallos de la memoria **se clasifican, no se traducen** (ADR-0009), igual
//! que los de la rúbrica en [`crate::rubric::error`].
//!
//! Aquí solo hay dos, y es a propósito: **un fichero que no se deja leer** y
//! **un fichero que no se deja escribir**. Que el contenido no parsee o venga
//! de una versión desconocida **no es un fallo**: es una
//! [`Recovery`](super::Recovery), se aparta a `.bak` y la aplicación arranca
//! con los valores por omisión. Una preferencia corrupta no puede impedir
//! firmar (ADR-0010), así que no puede ser un error.

use std::fmt;
use std::path::Path;

/// Situación que el usuario puede entender, y que el catálogo traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El fichero está pero no se ha podido leer. No es «no hay nada
    /// guardado»: eso son los valores por omisión y no se avisa de ello.
    Unreadable,
    /// No se ha podido escribir. Lo que hubiera guardado **sigue intacto**: la
    /// escritura es atómica y el fallo ocurre sobre el temporal o en el
    /// `rename`.
    Unwritable,
}

/// Un fallo de la memoria: la situación traducible y el detalle crudo.
///
/// [`MemoryError::detail`] nunca está vacío y **nunca** está traducido: nombra
/// el fichero y arrastra el error del sistema, que es lo que se pega en un
/// informe de fallo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryError {
    situation: Situation,
    detail: String,
}

impl MemoryError {
    /// Un fallo con su detalle técnico, sin traducir.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// El fallo de un fichero concreto, con la ruta dentro del detalle.
    pub fn about(situation: Situation, path: &Path, error: &std::io::Error) -> Self {
        Self::new(situation, format!("{}: {error}", path.display()))
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

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for MemoryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    /// **Grada A**: ni disco ni token.
    #[test]
    fn a_failure_names_the_file_next_to_the_situation() {
        let error = MemoryError::about(
            Situation::Unwritable,
            &PathBuf::from("/x/state.json"),
            &std::io::Error::new(ErrorKind::PermissionDenied, "denegado"),
        );

        assert_eq!(error.situation(), Situation::Unwritable);
        assert!(error.detail().contains("/x/state.json"));
        assert!(error.to_string().contains("denegado"));
    }
}
