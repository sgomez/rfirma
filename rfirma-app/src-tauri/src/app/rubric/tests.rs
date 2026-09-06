use std::fs;
use std::io::Cursor;

use image::{ImageFormat, Rgba, RgbaImage};
use tauri_plugin_dialog::FilePath;

use super::{choose, stored};
use crate::rubric::RubricStore;

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

    let normalized =
        choose(&store, FilePath::from(source.as_path())).expect("deberia adoptar la imagen");

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

    let error =
        choose(&store, FilePath::from(source.as_path())).expect_err("deberia rechazarse");

    assert_eq!(
        error.situation(),
        crate::rubric::Situation::NotAnAcceptedImage
    );
    assert_eq!(store.stored().expect("deberia leerse"), None);
}

#[test]
fn stored_reads_back_what_a_previous_session_adopted() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let source = home.path().join("firma-escaneada.png");
    a_png(&source);
    let store = RubricStore::at(home.path().join("rubric.jpg"));
    let normalized =
        choose(&store, FilePath::from(source.as_path())).expect("deberia adoptar la imagen");

    let bytes = stored(&store).expect("deberia leerse");

    assert_eq!(bytes, Some(normalized.bytes().to_vec()));
}

#[test]
fn stored_is_none_when_nothing_has_been_adopted_yet() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = RubricStore::at(home.path().join("rubric.jpg"));

    assert_eq!(stored(&store).expect("deberia leerse"), None);
}
