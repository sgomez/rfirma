//! Elegir la rúbrica: adopta en el almacén lo que el diálogo del portal
//! concede (ID-82).
//!
//! No decide nada que [`crate::rubric::RubricStore::adopt`] no decida ya —leer
//! con tope, normalizar, copiar—: la única razón por la que este módulo existe
//! es la regla de dirección, que a [`crate::commands`] solo le deja llamar a
//! [`crate::app`] (ID-79, ID-81). Las seis situaciones de fallo se prueban a
//! fondo en `rubric::store` y `rubric::normalize`; aquí solo se prueba que la
//! orden llama a lo que tiene que llamar, que es lo que TD-21 pide de un caso
//! de uso nuevo.

use std::path::Path;

use crate::rubric::{NormalizedRubric, RubricError, RubricStore};

/// Adopta la imagen que el usuario acaba de elegir en el diálogo del portal.
pub fn choose(store: &RubricStore, source: &Path) -> Result<NormalizedRubric, RubricError> {
    store.adopt(source)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;

    use image::{ImageFormat, Rgba, RgbaImage};

    use super::choose;
    use crate::rubric::RubricStore;

    /// **Grada A**: escribe en un directorio temporal, sin token ni puente.
    fn a_png(path: &std::path::Path) {
        let mut image = RgbaImage::new(10, 10);
        for pixel in image.pixels_mut() {
            *pixel = Rgba([10, 20, 30, 255]);
        }
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");
        fs::write(path, bytes).expect("el PNG de prueba deberia escribirse");
    }

    #[test]
    fn choosing_adopts_the_picked_image_into_the_store() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let source = home.path().join("firma-escaneada.png");
        a_png(&source);
        let store = RubricStore::at(home.path().join("rubric.jpg"));

        let normalized = choose(&store, &source).expect("deberia adoptar la imagen");

        assert_eq!(
            store.stored().expect("deberia leerse"),
            Some(normalized.bytes().to_vec())
        );
    }

    #[test]
    fn choosing_a_file_that_is_not_an_image_fails_without_touching_the_store() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let source = home.path().join("no-es-una-imagen.txt");
        fs::write(&source, b"esto no es una imagen").expect("deberia escribirse");
        let store = RubricStore::at(home.path().join("rubric.jpg"));

        let error = choose(&store, &source).expect_err("deberia rechazarse");

        assert_eq!(
            error.situation(),
            crate::rubric::Situation::NotAnAcceptedImage
        );
        assert_eq!(store.stored().expect("deberia leerse"), None);
    }
}
