//! El tamaño de fábrica y el mínimo de la ventana están **duplicados a
//! mano**: una vez en `tauri.conf.json`, que es lo que lee Tauri al abrir la
//! ventana, y otra en las constantes de [`rfirma_lib::app::window`], que es
//! lo que usa el resto del código (`initial_window`, `default_window`) para
//! saber a qué tamaño volver cuando no hay nada recordado.
//!
//! **Grada A**: lee un fichero del repositorio y una constante compilada,
//! nada más. Sin token, sin librería nativa y sin red.
//!
//! Nada aquí prueba que la ventana quepa de verdad en el escritorio: eso
//! depende del gestor de ventanas y no hay dónde afirmarlo. Lo único que se
//! prueba es que los dos sitios donde vive el número dicen lo mismo.

use std::fs;
use std::path::{Path, PathBuf};

use rfirma_lib::app::window::{DEFAULT_HEIGHT, DEFAULT_WIDTH, MIN_HEIGHT, MIN_WIDTH};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn configured_window() -> serde_json::Value {
    let path = crate_root().join("tauri.conf.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("deberia leerse {}: {error}", path.display()));
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).expect("tauri.conf.json deberia ser JSON valido");

    parsed["app"]["windows"][0].clone()
}

#[test]
fn the_factory_size_matches_the_window_memory_constants() {
    let window = configured_window();

    assert_eq!(
        window["width"].as_f64(),
        Some(DEFAULT_WIDTH),
        "tauri.conf.json declara un ancho de fabrica distinto de DEFAULT_WIDTH"
    );
    assert_eq!(
        window["height"].as_f64(),
        Some(DEFAULT_HEIGHT),
        "tauri.conf.json declara un alto de fabrica distinto de DEFAULT_HEIGHT"
    );
}

#[test]
fn the_minimum_size_matches_the_window_memory_constants() {
    let window = configured_window();

    assert_eq!(
        window["minWidth"].as_f64(),
        Some(MIN_WIDTH),
        "tauri.conf.json declara un ancho minimo distinto de MIN_WIDTH"
    );
    assert_eq!(
        window["minHeight"].as_f64(),
        Some(MIN_HEIGHT),
        "tauri.conf.json declara un alto minimo distinto de MIN_HEIGHT"
    );
}
