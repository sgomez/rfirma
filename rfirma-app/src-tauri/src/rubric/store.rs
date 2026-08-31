//! El almacén de la rúbrica: **se copia, no se referencia** (ID-33, ADR-0010).
//!
//! AutoFirma guarda la ruta del PNG que eligió el usuario y pierde la rúbrica
//! en silencio en cuanto ese fichero se mueve o se borra. Aquí se guarda el
//! JPEG ya normalizado dentro del directorio de la aplicación, así que el
//! original deja de importar en cuanto se elige.
//!
//! Es **una sola** rúbrica y se sustituye al elegir otra: por eso el almacén es
//! un fichero y no una carpeta con historia.
//!
//! Este módulo no sabe **dónde** está ese fichero: la ruta se la dan hecha. El
//! `paths.rs` del ADR-0010 —el único sitio del código con un `cfg!` de sistema
//! operativo— es quien resolverá `rubric_path()` contra el resolutor de rutas
//! de Tauri; hasta que exista, cualquier ruta sirve, y las pruebas se apoyan en
//! eso.

use std::fs;
use std::path::{Path, PathBuf};

use super::error::{RubricError, Situation};
use super::normalize::{normalize, NormalizedRubric};

/// El sitio donde vive la rúbrica de la aplicación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RubricStore {
    path: PathBuf,
}

impl RubricStore {
    /// El almacén que guarda la rúbrica en `path`.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// La ruta del JPEG guardado. Es la que lee la miniatura del panel de firma
    /// y la que lee la firma para codificarla en Base64.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Lee la imagen que ha elegido el usuario, la normaliza y **se queda con
    /// una copia**.
    ///
    /// Se valida al elegir, no al firmar (ADR-0010): un fichero que no vale
    /// falla con el diálogo del usuario todavía abierto.
    pub fn adopt(&self, source: &Path) -> Result<NormalizedRubric, RubricError> {
        let bytes = fs::read(source).map_err(|error| {
            RubricError::new(
                Situation::SourceUnreadable,
                format!("{}: {error}", source.display()),
            )
        })?;
        let normalized = normalize(&bytes)?;
        self.save(&normalized)?;
        Ok(normalized)
    }

    /// Escribe la rúbrica ya normalizada, sustituyendo la que hubiera.
    ///
    /// Escritura atómica —temporal y `rename`— como los otros dos ficheros de
    /// la memoria entre sesiones (ADR-0010): una rúbrica a medio escribir es
    /// una miniatura rota que nadie sabría de dónde viene.
    pub fn save(&self, rubric: &NormalizedRubric) -> Result<(), RubricError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| self.unwritable(&error))?;
        }
        let temporary = self.path.with_extension("jpg.tmp");
        fs::write(&temporary, rubric.bytes()).map_err(|error| self.unwritable(&error))?;
        fs::rename(&temporary, &self.path).map_err(|error| self.unwritable(&error))
    }

    /// La rúbrica guardada, si la hay. `None` cuando el usuario no ha elegido
    /// ninguna todavía.
    pub fn stored(&self) -> Option<Vec<u8>> {
        fs::read(&self.path).ok()
    }

    fn unwritable(&self, error: &std::io::Error) -> RubricError {
        RubricError::new(
            Situation::StoreUnwritable,
            format!("{}: {error}", self.path.display()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    /// **Grada A**: escribe en un directorio temporal, sin token ni puente.
    fn a_png(path: &Path) {
        let mut image = RgbaImage::new(10, 10);
        for pixel in image.pixels_mut() {
            *pixel = Rgba([30, 60, 90, 255]);
        }
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");
        fs::write(path, bytes).expect("el PNG de prueba deberia escribirse");
    }

    #[test]
    fn adopting_a_rubric_survives_deleting_the_original() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let source = directory.path().join("firma-escaneada.png");
        a_png(&source);
        let store = RubricStore::at(directory.path().join("almacen/rubric.jpg"));

        let adopted = store.adopt(&source).expect("deberia adoptarse");
        fs::remove_file(&source).expect("el original deberia poder borrarse");

        assert_eq!(store.stored().as_deref(), Some(adopted.bytes()));
    }

    #[test]
    fn what_is_stored_is_the_normalised_jpeg_and_not_the_original_png() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let source = directory.path().join("rubrica.png");
        a_png(&source);
        let store = RubricStore::at(directory.path().join("rubric.jpg"));

        store.adopt(&source).expect("deberia adoptarse");

        let stored = store.stored().expect("deberia haber rubrica guardada");
        assert_eq!(image::guess_format(&stored).ok(), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn choosing_another_rubric_replaces_the_only_one_there_is() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = RubricStore::at(directory.path().join("rubric.jpg"));
        let first = directory.path().join("primera.png");
        a_png(&first);
        store.adopt(&first).expect("deberia adoptarse la primera");

        let second = directory.path().join("segunda.png");
        let mut image = RgbaImage::new(40, 12);
        for pixel in image.pixels_mut() {
            *pixel = Rgba([250, 250, 250, 255]);
        }
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");
        fs::write(&second, bytes).expect("el PNG de prueba deberia escribirse");
        let adopted = store.adopt(&second).expect("deberia adoptarse la segunda");

        assert_eq!(store.stored().as_deref(), Some(adopted.bytes()));
        let stored = image::load_from_memory(&store.stored().expect("deberia haber rubrica"))
            .expect("deberia decodificarse");
        assert_eq!((stored.width(), stored.height()), (40, 12));
    }

    #[test]
    fn a_file_that_is_not_an_image_leaves_the_store_untouched() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = RubricStore::at(directory.path().join("rubric.jpg"));
        let source = directory.path().join("documento.pdf");
        fs::write(&source, b"%PDF-1.7 no es una rubrica").expect("deberia escribirse");

        let error = store.adopt(&source).expect_err("un PDF deberia rechazarse");

        assert_eq!(error.situation(), Situation::NotAnAcceptedImage);
        assert_eq!(store.stored(), None);
    }

    #[test]
    fn a_source_that_is_not_there_is_rejected_naming_it() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = RubricStore::at(directory.path().join("rubric.jpg"));

        let error = store
            .adopt(&directory.path().join("no-existe.png"))
            .expect_err("un fichero ausente deberia rechazarse");

        assert_eq!(error.situation(), Situation::SourceUnreadable);
        assert!(error.detail().contains("no-existe.png"));
    }

    #[test]
    fn a_store_that_cannot_be_written_says_so_instead_of_disappearing() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        // Un directorio donde debería ir el fichero: `rename` no puede con él.
        let taken = directory.path().join("rubric.jpg");
        fs::create_dir(&taken).expect("deberia crearse el directorio");
        let store = RubricStore::at(&taken);
        let source = directory.path().join("rubrica.png");
        a_png(&source);

        let error = store
            .adopt(&source)
            .expect_err("deberia fallar al escribir");

        assert_eq!(error.situation(), Situation::StoreUnwritable);
        assert!(error.detail().contains("rubric.jpg"));
    }
}
