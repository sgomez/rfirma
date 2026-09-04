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

use std::fs::{self, File};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use super::error::{RubricError, Situation};
use super::normalize::{normalize, NormalizedRubric, MAX_INPUT_BYTES};

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
        // Del origen se nombra **el último segmento y nada más**: bajo el
        // sandbox `source` es el enlace del portal
        // (`/run/user/<uid>/doc/<id>/firma.png`), y este detalle crudo cruza a
        // la ventana dentro de `RubricChoiceView` (ADR-0011, ID-186). El
        // nombre del fichero es el que la persona acaba de elegir y no dice
        // dónde está.
        let named = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let unreadable = |error: std::io::Error| {
            RubricError::new(Situation::SourceUnreadable, format!("{named}: {error}"))
        };

        // Se lee **con tope**, no entero y luego se mide. `adopt` es el único
        // camino real hasta `normalize`, así que un `fs::read` a secas metería
        // en memoria lo que el usuario haya elegido en el portal —un vídeo, una
        // imagen de disco— solo para acabar devolviendo `ImageTooLarge`. Se lee
        // un byte de más que el tope: si aparece, el fichero lo pasa. `take`
        // sobre el fichero ya abierto y no un `metadata` previo, para no dejar
        // una ventana entre comprobar el tamaño y leerlo.
        let mut bytes = Vec::new();
        File::open(source)
            .map_err(&unreadable)?
            .take(MAX_INPUT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(unreadable)?;
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RubricError::new(
                Situation::ImageTooLarge,
                format!("{named} pasa del tope de {MAX_INPUT_BYTES} bytes"),
            ));
        }

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
        fs::rename(&temporary, &self.path).map_err(|error| {
            // El `rename` que falla deja el temporal escrito; barrerlo es parte
            // de fallar sin dejar rastro. Si el borrado tampoco puede, el fallo
            // que se cuenta sigue siendo el del `rename`.
            let _ = fs::remove_file(&temporary);
            self.unwritable(&error)
        })
    }

    /// La rúbrica guardada, si la hay.
    ///
    /// `Ok(None)` significa **una** cosa y solo una: el usuario no ha elegido
    /// rúbrica todavía. Un almacén que existe pero no se deja leer sale como
    /// `Err`, no como `None`: fundir las dos en `None` sería justo el
    /// desaparecer en silencio que este módulo evita en todas partes.
    pub fn stored(&self) -> Result<Option<Vec<u8>>, RubricError> {
        match fs::read(&self.path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(RubricError::new(
                Situation::StoreUnreadable,
                format!("{}: {error}", self.path.display()),
            )),
        }
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

        assert_eq!(
            store
                .stored()
                .expect("el almacen deberia leerse")
                .as_deref(),
            Some(adopted.bytes())
        );
    }

    #[test]
    fn what_is_stored_is_the_normalised_jpeg_and_not_the_original_png() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let source = directory.path().join("rubrica.png");
        a_png(&source);
        let store = RubricStore::at(directory.path().join("rubric.jpg"));

        store.adopt(&source).expect("deberia adoptarse");

        let stored = store
            .stored()
            .expect("el almacen deberia leerse")
            .expect("deberia haber rubrica guardada");
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

        let saved = store
            .stored()
            .expect("el almacen deberia leerse")
            .expect("deberia haber rubrica");
        assert_eq!(saved, adopted.bytes());
        let stored = image::load_from_memory(&saved).expect("deberia decodificarse");
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
        assert_eq!(store.stored().expect("el almacen deberia leerse"), None);
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
        assert!(!error.detail().contains(directory.path().to_str().unwrap()));
    }

    /// El detalle crudo cruza a la ventana dentro de `RubricChoiceView`, así
    /// que **no** puede llevar el enlace del portal (ADR-0011, ID-186): la
    /// rúbrica elegida se nombra por su último segmento.
    #[test]
    fn a_source_from_the_portal_is_named_without_its_grant_path() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = RubricStore::at(directory.path().join("rubric.jpg"));

        let error = store
            .adopt(Path::new("/run/user/1000/doc/1e8b83b9/firma.png"))
            .expect_err("la concesion del portal no existe fuera del sandbox");

        assert_eq!(error.situation(), Situation::SourceUnreadable);
        assert!(error.detail().contains("firma.png"));
        assert!(!error.detail().contains("/run/user/"));
    }

    #[test]
    fn a_source_over_the_input_cap_is_rejected_without_reading_it_whole() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = RubricStore::at(directory.path().join("rubric.jpg"));
        let source = directory.path().join("enorme.png");
        fs::write(&source, vec![0_u8; MAX_INPUT_BYTES + 4096]).expect("deberia escribirse");

        let error = store
            .adopt(&source)
            .expect_err("un fichero por encima del tope deberia rechazarse");

        assert_eq!(error.situation(), Situation::ImageTooLarge);
        assert!(error.detail().contains("enorme.png"));
        assert_eq!(store.stored().expect("el almacen deberia leerse"), None);
    }

    #[test]
    fn a_store_that_cannot_be_read_is_not_the_same_as_having_no_rubric() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        // Un directorio en el sitio del fichero: existe, pero `read` no puede.
        let taken = directory.path().join("rubric.jpg");
        fs::create_dir(&taken).expect("deberia crearse el directorio");

        let error = RubricStore::at(&taken)
            .stored()
            .expect_err("un almacen ilegible deberia fallar, no salir como None");

        assert_eq!(error.situation(), Situation::StoreUnreadable);
        assert!(error.detail().contains("rubric.jpg"));
    }

    #[test]
    fn a_store_that_cannot_be_written_says_so_and_leaves_no_temporary_behind() {
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
        assert!(
            !directory.path().join("rubric.jpg.tmp").exists(),
            "el temporal deberia barrerse cuando el rename falla"
        );
    }
}
