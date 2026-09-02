//! Las pruebas que necesitan ver **todas** las órdenes a la vez (ID-85).
//!
//! **Grada A**: leen la fuente de este módulo y nada más. Sin token, sin
//! librería nativa y sin red. El ciclo contra el token y `pdfsig` es la grada C
//! de `tests/native_cycle.rs`.
//!
//! Están en un fichero aparte porque no son de ninguno de los otros: la de
//! rutas recorre los tipos de salida vivan donde vivan, la de la lista cerrada
//! cuenta las órdenes, la del hilo del portal mira cómo se declaran dos de
//! ellas, y la del PIN comprueba que solo una lo recibe. Un fichero solo con
//! pruebas es lo que hace que ninguna de las cuatro dependa de en qué fichero
//! del módulo esté lo que vigila (ID-84).

use std::collections::BTreeSet;
use std::path::Path;

/// Los ficheros del módulo, con su fuente.
///
/// La guarda [`the_list_of_files_covers_the_whole_module`] comprueba que aquí
/// están **todos**: sin ella, un fichero nuevo con un tipo de salida dentro
/// pasaría sin que nadie lo mirara, que es justo el fallo que el ID-84 viene a
/// arreglar.
const SOURCES: [(&str, &str); 4] = [
    ("mod.rs", include_str!("mod.rs")),
    ("failure.rs", include_str!("failure.rs")),
    ("orders.rs", include_str!("orders.rs")),
    ("views.rs", include_str!("views.rs")),
];

/// Este mismo fichero no se recorre: no tiene producción, solo las guardas, y
/// leerlo encontraría los literales con los que están escritas.
const THIS_FILE: &str = "guards.rs";

/// La fuente de un fichero del módulo, por su nombre.
fn source_of(file: &str) -> &'static str {
    SOURCES
        .iter()
        .find(|(name, _)| *name == file)
        .map(|(_, source)| *source)
        .unwrap_or_else(|| panic!("«{file}» tiene que estar en la lista del modulo"))
}

/// La mitad de producción de una fuente, sin sus pruebas: si no, estas
/// comprobaciones leerían los literales de los tests y se creerían cualquier
/// cosa.
fn production_half(source: &str) -> &str {
    source
        .split_once("\nmod tests {")
        .map(|(before, _)| before)
        .unwrap_or(source)
}

/// Un tipo que cruza a la ventana: el que se serializa.
struct Output<'a> {
    file: &'a str,
    name: String,
    body: String,
}

/// Todos los tipos de salida del módulo, **descubiertos y no enumerados**.
///
/// Cruza a la ventana lo que se serializa, así que lo que se busca es un
/// `derive` con `Serialize` dentro y el `struct` o el `enum` que va detrás. Un
/// tipo de salida nuevo queda cubierto **por existir**, esté en el fichero que
/// esté: eso es el ID-84, y es lo que la lista fija de nombres de antes no
/// hacía.
fn outputs() -> Vec<Output<'static>> {
    let mut found = Vec::new();
    for (file, source) in SOURCES {
        let mut serialisable = false;
        let mut open: Option<(String, String)> = None;
        for line in production_half(source).lines() {
            let trimmed = line.trim_start();
            if let Some((name, body)) = open.as_mut() {
                if line == "}" {
                    found.push(Output {
                        file,
                        name: std::mem::take(name),
                        body: std::mem::take(body),
                    });
                    open = None;
                } else {
                    body.push_str(line);
                    body.push('\n');
                }
                continue;
            }
            if trimmed.starts_with("#[derive(") {
                serialisable = trimmed.contains("Serialize");
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with("#[") || trimmed.starts_with("///") {
                continue;
            }
            let declaration = trimmed
                .strip_prefix("pub struct ")
                .or_else(|| trimmed.strip_prefix("pub enum "));
            if let Some(rest) = declaration {
                if serialisable {
                    let name = rest
                        .split(|letter: char| !letter.is_alphanumeric() && letter != '_')
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    open = Some((name, String::new()));
                }
            }
            serialisable = false;
        }
    }
    found
}

/// Bajo el arenero la aplicación no conoce la ruta real de un documento, así
/// que devolver una sería devolver una mentira (ADR-0011). Lo que sale son
/// nombres.
///
/// La lista de fugas mira **cómo se escribe una ruta en Rust** —un `PathBuf`,
/// un `&Path`, un campo que se llame `path`— y los tres datos del almacén que
/// son rutas del anfitrión disfrazadas.
#[test]
fn no_output_of_any_command_carries_a_host_path() {
    let outputs = outputs();
    assert!(
        outputs.len() >= 6,
        "los tipos de salida no se han encontrado: {}",
        outputs.len()
    );

    for output in outputs {
        for leak in [
            "PathBuf",
            "&Path",
            "path:",
            "module:",
            "reading_path",
            "init_args",
            "configdir",
        ] {
            assert!(
                !output.body.contains(leak),
                "«{}» ({}) ha ganado un «{leak}»: eso es una ruta del anfitrion saliendo",
                output.name,
                output.file
            );
        }
    }
}

/// Y la lista de ficheros que recorre la guarda de arriba es **la del módulo
/// entero**: un fichero nuevo sin dar de alta aquí se llevaría sus tipos de
/// salida sin que nadie los mirara.
#[test]
fn the_list_of_files_covers_the_whole_module() {
    let listed: BTreeSet<&str> = SOURCES.iter().map(|(file, _)| *file).collect();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
    let present: BTreeSet<String> = std::fs::read_dir(&directory)
        .expect("el modulo de ordenes tiene que estar donde dice el manifiesto")
        .map(|entry| entry.expect("deberia leerse la entrada").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".rs"))
        .filter(|name| name != THIS_FILE)
        .collect();

    let missing: Vec<&String> = present
        .iter()
        .filter(|name| !listed.contains(name.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "estos ficheros del modulo no los recorre la guarda de rutas: {missing:?}"
    );
}

/// La lista sigue cerrada (ID-59): ocho órdenes más las tres de los ajustes.
///
/// Cuenta el prefijo `#[tauri::command` y no el atributo entero porque varias
/// llevan `(async)`: lo que se cierra es cuántas órdenes hay, no cómo se
/// ejecuta cada una.
#[test]
fn the_list_of_commands_grew_to_eleven_and_no_further() {
    let orders: usize = SOURCES
        .iter()
        .map(|(_, source)| production_half(source).matches("#[tauri::command").count())
        .sum();

    assert_eq!(orders, 11, "la lista de ordenes es cerrada a proposito");
}

/// Y las dos que hablan con el disco o con el portal **no bloquean el hilo
/// principal**: `#[tauri::command]` a secas genera un cuerpo `Blocking` que
/// corre dentro del manejador del IPC —el hilo del bucle GTK—, y
/// `blocking_pick_file()` espera allí a un cierre que solo ese hilo puede
/// ejecutar. Punto muerto: la ventana se clava y el diálogo no aparece.
#[test]
fn the_two_commands_that_touch_the_portal_run_off_the_main_thread() {
    for command in ["pub fn open_document(", "pub fn read_document("] {
        let source = production_half(source_of("mod.rs"));
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        let before = &source[..declaration];
        assert!(
            before.ends_with("#[tauri::command(async)]\n"),
            "«{command}» tiene que ser #[tauri::command(async)]"
        );
    }
}

/// El PIN entra por **una sola orden**, se usa en el token y no vuelve.
///
/// Que no se guarde en el ciclo a medias lo comprueba
/// `the_pin_is_never_kept_in_the_half_open_cycle`, en
/// [`crate::app::signing`]; lo que solo se puede ver desde aquí es que ninguna
/// otra orden lo reciba.
#[test]
fn the_pin_is_taken_by_a_single_command() {
    let takers: usize = SOURCES
        .iter()
        .map(|(_, source)| production_half(source).matches("pin: String").count())
        .sum();

    assert_eq!(takers, 1, "el PIN entra por una sola orden");
}
