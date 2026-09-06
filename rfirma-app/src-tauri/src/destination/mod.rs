//! Determinación y comprobación de la carpeta de destino del documento firmado (ADR-0011).

pub mod error;
pub mod naming;
pub mod portal;

pub use error::{DestinationError, Situation};
pub use naming::{numbered, signed_name, FIRST_NUMBER, MAX_NAMESAKES, SIGNED_SUFFIX};
pub use portal::{the_original_folder_can_be_offered, PortalDocument};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Carpeta configurada para guardar los documentos firmados.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinationFolder {
    path: PathBuf,
}

impl DestinationFolder {
    /// Construye una carpeta de destino con la ruta indicada.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Ruta de la carpeta de destino.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nombre del segmento final de la carpeta para visualización.
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }
}

/// Carpeta de destino cuya existencia ha sido verificada en el sistema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedFolder {
    path: PathBuf,
}

impl CheckedFolder {
    /// Comprueba la existencia y validez de la carpeta de destino (ADR-0011).
    pub fn check(folder: &DestinationFolder) -> Result<Self, DestinationError> {
        Self::at(folder.path())
    }

    /// Comprueba la existencia y validez de una ruta de destino.
    pub fn at(path: impl AsRef<Path>) -> Result<Self, DestinationError> {
        let path = path.as_ref();
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Ok(Self {
                path: path.to_path_buf(),
            }),
            Ok(_) => Err(DestinationError::about(Situation::NotAFolder, path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(DestinationError::about(Situation::FolderMissing, path))
            }
            Err(error) => Err(DestinationError::caused_by(
                Situation::FolderUnreadable,
                path,
                &error,
            )),
        }
    }

    /// Ruta verificada de la carpeta de destino.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nombre del segmento final de la carpeta para visualización (ADR-0011).
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }

    /// Calcula la ruta final del documento firmado resolviendo homónimos (ADR-0011).
    pub fn landing_for(&self, document: &PortalDocument) -> Result<PathBuf, DestinationError> {
        let name = signed_name(document.name());
        let first = self.path.join(&name);
        if !first.exists() {
            return Ok(first);
        }
        for number in FIRST_NUMBER..=MAX_NAMESAKES {
            let candidate = self.path.join(numbered(&name, number));
            if !candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(DestinationError::about(Situation::NoFreeName, &first))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn a_folder() -> tempfile::TempDir {
        tempfile::tempdir().expect("deberia haber directorio temporal")
    }

    fn a_document() -> PortalDocument {
        PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf")
    }

    #[test]
    fn a_folder_that_is_there_can_be_checked_and_shows_its_name() {
        let root = a_folder();
        let path = root.path().join("Documentos");
        fs::create_dir(&path).expect("deberia crearse");

        let checked = CheckedFolder::at(&path).expect("deberia comprobarse");

        assert_eq!(checked.path(), path);
        assert_eq!(checked.name(), "Documentos");
    }

    #[test]
    fn the_folder_is_never_created_here() {
        let root = a_folder();
        let missing = root.path().join("Documentos");

        let failure = CheckedFolder::at(&missing).expect_err("no deberia comprobarse");

        assert_eq!(failure.situation(), Situation::FolderMissing);
        assert!(failure.detail().contains("Documentos"));
        assert!(
            !missing.exists(),
            "comprobar una carpeta que falta no puede crearla"
        );
    }

    #[test]
    fn a_file_where_the_folder_should_be_is_not_a_destination() {
        let root = a_folder();
        let path = root.path().join("Documentos");
        fs::write(&path, b"no soy una carpeta").expect("deberia escribirse");

        let failure = CheckedFolder::at(&path).expect_err("no deberia comprobarse");

        assert_eq!(failure.situation(), Situation::NotAFolder);
    }

    #[test]
    fn the_signed_document_lands_in_the_destination_folder_with_no_dialogue() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");

        let landing = checked
            .landing_for(&a_document())
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado.pdf"));
    }

    #[test]
    fn the_landing_never_falls_next_to_the_original() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        let document = a_document();

        let landing = checked.landing_for(&document).expect("deberia haber sitio");

        assert_eq!(landing.parent(), Some(checked.path()));
        assert_ne!(landing.parent(), document.reading_path().parent());
    }

    #[test]
    fn a_second_signature_is_numbered_instead_of_overwriting_the_first() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        fs::write(root.path().join("contrato-firmado.pdf"), b"la primera")
            .expect("deberia escribirse");

        let landing = checked
            .landing_for(&a_document())
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado-2.pdf"));
        assert_eq!(
            fs::read(root.path().join("contrato-firmado.pdf")).expect("deberia leerse"),
            b"la primera",
            "la primera firma sigue donde estaba"
        );
    }

    #[test]
    fn cosigning_the_signed_document_does_not_stack_a_second_suffix() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        let already_signed = PortalDocument::opened("/run/user/1000/doc/aa/contrato-firmado.pdf");

        let landing = checked
            .landing_for(&already_signed)
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado.pdf"));
    }

    #[test]
    fn the_third_cosignature_keeps_counting_instead_of_stacking() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        fs::write(root.path().join("contrato-firmado.pdf"), b"la primera")
            .expect("deberia escribirse");
        fs::write(root.path().join("contrato-firmado-2.pdf"), b"la segunda")
            .expect("deberia escribirse");
        let signed_twice = PortalDocument::opened("/run/user/1000/doc/aa/contrato-firmado-2.pdf");

        let landing = checked
            .landing_for(&signed_twice)
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado-3.pdf"));
    }

    #[test]
    fn deciding_where_it_lands_writes_nothing() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");

        let landing = checked
            .landing_for(&a_document())
            .expect("deberia haber sitio");

        assert!(
            !landing.exists(),
            "quien escribe es la orquestacion, no esto"
        );
    }

    #[test]
    fn the_destination_shows_its_name_and_not_its_path() {
        let folder = DestinationFolder::at("/home/quien/Documentos/Firmados");

        assert_eq!(folder.name(), "Firmados");
    }
}
