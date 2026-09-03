//! Los fallos del destino **se clasifican, no se traducen** (ADR-0009), igual
//! que los de la memoria en [`crate::memory::error`].
//!
//! Solo hay tres, y los tres dicen lo mismo con distinto detalle: **ahí no se
//! puede dejar el documento**. Ninguno de ellos autoriza a arreglar la
//! situación creando nada (ID-38): la carpeta se comprueba y, si no está, se
//! avisa. El pie del panel sustituye el destino por «No se puede escribir en
//! *Documents*» y el botón de firmar **no se apaga** (ADR-0011).

use std::fmt;
use std::path::Path;

/// Situación que el usuario puede entender, y que el catálogo traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// La carpeta de destino **no existe en el anfitrión**.
    ///
    /// Bajo el sandbox esto no se nota escribiendo: `mkdir` y la escritura
    /// contestan OK, el fichero se relee bien y en el anfitrión no hay nada
    /// (#27, `docs/research/flatpak-canal-unico.md`). Por eso la única
    /// respuesta correcta es decirlo, nunca crearla.
    FolderMissing,
    /// La ruta está, pero no es una carpeta. Un fichero llamado `Documents`
    /// no es un destino, y tratarlo como tal acabaría machacándolo.
    NotAFolder,
    /// No se ha podido ni consultar la ruta: permisos, un montaje que no
    /// responde. No es [`Situation::FolderMissing`], porque no se sabe si
    /// está.
    FolderUnreadable,
    /// La carpeta está pero todos los nombres razonables están ocupados
    /// ([`super::MAX_NAMESAKES`] homónimos). Sin diálogo por firma no hay
    /// ningún «¿reemplazar?» que avise, así que antes que machacar un fichero
    /// del usuario se para.
    NoFreeName,
}

/// Un fallo del destino: la situación traducible y el detalle crudo.
///
/// [`DestinationError::detail`] nunca está vacío y **nunca** está traducido:
/// nombra la ruta y arrastra el error del sistema, que es lo que se pega en un
/// informe de fallo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestinationError {
    situation: Situation,
    detail: String,
}

impl DestinationError {
    /// Un fallo con su detalle técnico, sin traducir.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// El fallo de una ruta concreta, con la ruta dentro del detalle.
    pub fn about(situation: Situation, path: &Path) -> Self {
        Self::new(situation, path.display().to_string())
    }

    /// El fallo de una ruta concreta que además arrastra un error del sistema.
    pub fn caused_by(situation: Situation, path: &Path, error: &std::io::Error) -> Self {
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

impl fmt::Display for DestinationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for DestinationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    /// **Grada A**: ni disco ni token.
    #[test]
    fn a_missing_folder_names_the_path_it_could_not_find() {
        let error =
            DestinationError::about(Situation::FolderMissing, &PathBuf::from("/home/quien/Docs"));

        assert_eq!(error.situation(), Situation::FolderMissing);
        assert_eq!(error.detail(), "/home/quien/Docs");
        assert!(error.to_string().contains("FolderMissing"));
    }

    #[test]
    fn an_unreadable_folder_drags_the_system_error_along() {
        let error = DestinationError::caused_by(
            Situation::FolderUnreadable,
            &PathBuf::from("/mnt/red/Docs"),
            &std::io::Error::new(ErrorKind::PermissionDenied, "denegado"),
        );

        assert!(error.detail().contains("/mnt/red/Docs"));
        assert!(error.detail().contains("denegado"));
    }
}
