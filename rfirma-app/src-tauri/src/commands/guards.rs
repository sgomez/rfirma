//! Las pruebas que necesitan ver **todas** las órdenes a la vez (ID-85).
//!
//! **Grada A**: leen la fuente de este módulo y nada más. Sin token, sin
//! librería nativa y sin red. El ciclo contra el token y `pdfsig` es la grada C
//! de `tests/native_cycle.rs`.
//!
//! Están en un fichero aparte porque no son de ninguno de los otros: la de
//! rutas construye los tipos de salida vivan donde vivan, la de la lista cerrada
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
const SOURCES: [(&str, &str); 7] = [
    ("mod.rs", include_str!("mod.rs")),
    ("failure.rs", include_str!("failure.rs")),
    ("orders.rs", include_str!("orders.rs")),
    ("rubric.rs", include_str!("rubric.rs")),
    ("site_window.rs", include_str!("site_window.rs")),
    ("views.rs", include_str!("views.rs")),
    ("views_site.rs", include_str!("views_site.rs")),
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

/// La misma fuente con cada atributo en **una sola línea**.
///
/// `rustfmt` parte un `#[derive(...)]` en varias líneas en cuanto la lista se
/// alarga, y una lectura línea a línea vería el `#[derive(` sin el `Serialize`
/// que va debajo: el tipo dejaría de descubrirse y la guarda seguiría en verde.
/// Juntar el atributo antes de mirarlo cierra esa rendija.
fn attributes_on_one_line(source: &str) -> String {
    let mut joined = String::new();
    let mut open = 0i32;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let starts_an_attribute = open == 0 && trimmed.starts_with("#[");
        if starts_an_attribute || open > 0 {
            if open == 0 {
                joined.push_str(line);
            } else {
                joined.push_str(trimmed);
            }
            open += line.matches('[').count() as i32 - line.matches(']').count() as i32;
            if open <= 0 {
                open = 0;
                joined.push('\n');
            }
            continue;
        }
        joined.push_str(line);
        joined.push('\n');
    }
    joined
}

/// Cuántos `derive` con `Serialize` hay en una fuente ya aplanada.
///
/// Es el contraste de [`outputs`]: si el módulo declara siete tipos
/// serializables y el descubrimiento encuentra seis, uno se está escapando.
fn serialising_derives(flattened: &str) -> usize {
    flattened
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("#[derive(") && line.contains("Serialize"))
        .count()
}

/// Un tipo que cruza a la ventana: el que se serializa.
struct Output<'a> {
    file: &'a str,
    name: String,
}

/// Todos los tipos de salida del módulo, **descubiertos y no enumerados**.
///
/// Cruza a la ventana lo que se serializa, así que lo que se busca es un
/// `derive` con `Serialize` dentro y el `struct` o el `enum` que va detrás. Un
/// tipo de salida nuevo queda cubierto **por existir**, esté en el fichero que
/// esté: eso es el ID-84, y es lo que la lista fija de nombres de antes no
/// hacía.
///
/// De cada uno se guarda **el nombre y nada más**: lo que se mira ya no es su
/// cuerpo escrito, sino el valor que produce (ID-186).
fn outputs() -> Vec<Output<'static>> {
    let mut found = Vec::new();
    for (file, source) in SOURCES {
        let flattened = attributes_on_one_line(production_half(source));
        let mut serialisable = false;
        let mut open: Option<String> = None;
        for line in flattened.lines() {
            let trimmed = line.trim_start();
            if let Some(name) = open.as_mut() {
                if line == "}" {
                    found.push(Output {
                        file,
                        name: std::mem::take(name),
                    });
                    open = None;
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
                    open = Some(name);
                }
            }
            serialisable = false;
        }
    }
    found
}

/// Los tipos de salida **detrás de los cuales no hay ningún documento**: lo
/// que llevan sale del token, de una imagen ya normalizada o de un
/// rectángulo, así que una ruta del portal no puede llegar hasta ellos por
/// ningún camino. `RubricView` lleva Base64 y medidas; `StatusView`,
/// `CertificateView` y `PlacementView`, lo que dicen sus nombres.
///
/// `SecretView` tampoco puede llevarla: son dos banderas de `CK_TOKEN_INFO` y
/// un contador vacío, y el almacén del que salen no aparece por ningún lado.
///
/// `RubricChoiceView` **no** está aquí, aunque lo estuvo: lleva un `Failure`
/// con el detalle crudo del `RubricError`, y por ahí sí hay un camino desde
/// el enlace del portal. Se construye con los demás.
///
/// Es la otra mitad de [`the_portal_path_never_crosses_to_the_window`]: entre
/// las dos tienen que sumar **todos** los tipos que el descubrimiento
/// encuentra, y por eso un tipo de salida nuevo obliga a decidir en cuál de
/// las dos entra. Sin esta lista, «no lo he construido» y «no puede llevar una
/// ruta» serían indistinguibles, que es como una guarda se queda en verde sin
/// mirar nada.
const OUTPUTS_WITH_NO_DOCUMENT_BEHIND: [&str; 15] = [
    "StatusView",
    "CertificateView",
    "PlacementView",
    "RubricView",
    "SecretView",
    // Detrás de una versión publicada no hay ningún documento: lo que lleva es
    // un número que vino de GitHub (ID-182).
    "NewVersionView",
    // Detrás de quién atiende `afirma://` no hay documento ninguno: lo que
    // lleva son ficheros `.desktop` que dio el escritorio (ID-238).
    "UrlHandlersView",
    "UrlHandlerView",
    // Detrás del trámite de sede no hay ningún documento **del portal**: en la
    // espera no hay ninguno todavía (ID-338), y el que manda la sede se nombra
    // con un asa acuñada y nunca con la ruta de su fichero de paso, que además
    // se borra al contestar (ID-286).
    "SiteErrandView",
    "SiteStageView",
    // Y detrás de si lo que se pide es firmar o cofirmar no hay más que el
    // verbo con el que la sede lo pidió.
    "SignatureRoundView",
    // Detrás de un callejón sin salida no hay ningún documento, y no puede
    // haberlo (ID-341): los tres ocurren **antes** de que la sede haya mandado
    // nada —el canal no se abrió, o el rechazo no tenía por dónde salir—, y lo
    // que llevan es un nombre de situación, un motivo y el detalle crudo de un
    // rechazo del protocolo, que nace de la URL y no de ningún fichero.
    "NoChannelView",
    // `SiteOutcomeView::Refused` lleva el detalle **crudo** de un `Refusal`, y
    // un `Refusal` sí puede llevar dentro el asa del portal (los hay sobre
    // `Parameter::Data`, el fichero de paso). Aquí no hay documento detrás
    // **mientras el único constructor sea el del callejón sin salida**:
    // `SiteErrandView::refused`, llamado sólo desde `SiteErrandView::from` con el `Refusal`
    // de `DeadEnd::RefusedWithoutChannel`, que nace de leer la URL de arranque.
    // Si `SiteStageView::Outcome` acaba enseñando rechazos nacidos de un
    // documento, esta línea sale de aquí y el tipo se construye en
    // `crossings_from_a_portal_document`.
    "SiteOutcomeView",
    "RefusalSituationView",
    // Y detrás de no tener ningún certificado tampoco: lo que lleva es un
    // motivo y un recuento del almacén de la persona (ID-277, ID-278).
    "NoCertificateView",
];

/// El enlace que el portal concede, que es lo que **no** puede salir.
const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

/// Un segundo enlace del portal, en la misma concesión que [`A_PORTAL_HANDLE`].
///
/// Existe para que `also_entering` de `DroppedDocumentView` no salga vacío: con
/// una sola ruta soltada la guarda del ADR-0011 nunca recorre ese campo.
const ANOTHER_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/segundo.pdf";

/// Un valor que ya ha cruzado: el tipo del que salió y su JSON.
struct Crossing {
    name: &'static str,
    json: serde_json::Value,
}

impl Crossing {
    /// Ese valor, tal y como sale por el IPC.
    fn of(name: &'static str, value: &impl serde::Serialize) -> Self {
        Self {
            name,
            json: serde_json::to_value(value).expect("un tipo de salida tiene que serializar"),
        }
    }
}

/// Si esa cadena es una ruta del portal de documentos: `/run/user/<uid>/doc/…`.
///
/// Mira la **cadena entera** y no su principio: el detalle crudo de un fallo
/// lleva el texto de un error del sistema, y ahí la ruta aparecería en medio de
/// una frase y no como el valor del campo.
fn is_a_portal_path(text: &str) -> bool {
    text.match_indices("/run/user/").any(|(at, _)| {
        text[at..]
            .split('/')
            .nth(4)
            .is_some_and(|segment| segment == "doc")
    })
}

/// La primera ruta del portal que haya **en cualquier punto** del valor: un
/// campo, un campo de un campo o un elemento de una lista.
fn the_portal_path_inside(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => is_a_portal_path(text).then(|| text.clone()),
        serde_json::Value::Array(items) => items.iter().find_map(the_portal_path_inside),
        serde_json::Value::Object(fields) => fields.values().find_map(the_portal_path_inside),
        _ => None,
    }
}

/// Todo lo que la producción produce **a partir de un documento del portal**,
/// ya serializado.
///
/// No se fabrica ningún valor a mano: cada uno sale del caso de uso que lo
/// produce de verdad, alimentado con el enlace del portal. Uno inventado campo
/// a campo solo probaría que quien escribió la prueba no puso una ruta dentro.
fn crossings_from_a_portal_document() -> Vec<Crossing> {
    use crate::app::fixtures::a_memory;
    use crate::app::{configuration, documents, recents};
    use crate::destination::{CheckedFolder, DestinationFolder, PortalDocument};
    use crate::memory::{Badge, Configuration, OpenedDocuments, RecentDocument, State};

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let document = PortalDocument::opened(A_PORTAL_HANDLE);
    // El destino elegido es el **directorio de la concesión**, que es el peor
    // caso: si de una carpeta saliera algo más que su último segmento, saldría
    // aquí.
    let configuration = Configuration {
        destination: Some(DestinationFolder::at(
            Path::new(A_PORTAL_HANDLE)
                .parent()
                .expect("la concesion tiene directorio"),
        )),
        remember_activity: true,
        ..Configuration::default()
    };

    let opened_view = documents::note_opened(
        &memory,
        &configuration,
        &opened,
        std::path::PathBuf::from(A_PORTAL_HANDLE),
    );
    let failure = documents::bytes_of(&opened, &opened_view.id)
        .expect_err("el enlace del portal no existe fuera del sandbox");
    let dropped = documents::dropped_document(
        &[
            std::path::PathBuf::from(A_PORTAL_HANDLE),
            std::path::PathBuf::from(ANOTHER_PORTAL_HANDLE),
        ],
        &opened,
    )
    .expect("se ha soltado un fichero");
    let folder = CheckedFolder::at(home.path()).expect("el temporal esta ahi");
    // Elegir como rúbrica el enlace del portal: el caso de uso de verdad, con
    // el `FilePath` que concede el diálogo. Falla porque la concesión no
    // existe fuera del sandbox, y ese fallo es lo que cruza.
    let refused_rubric = crate::app::rubric::choose(
        &crate::rubric::RubricStore::at(home.path().join("rubric.jpg")),
        tauri_plugin_dialog::FilePath::Path(std::path::PathBuf::from(A_PORTAL_HANDLE)),
    )
    .expect_err("el enlace del portal no existe fuera del sandbox");

    let mut crossings = vec![
        Crossing::of("OpenedDocumentView", &opened_view),
        Crossing::of("Failure", &failure),
        Crossing::of("DroppedDocumentView", &dropped),
        Crossing::of(
            "DestinationView",
            &documents::where_it_lands(&configuration, home.path(), &document),
        ),
        Crossing::of(
            "SignedDocumentView",
            &documents::told_as(document.reading_path(), &folder, 42),
        ),
        Crossing::of(
            "ConfigurationView",
            &configuration::shown(&configuration, home.path()),
        ),
        Crossing::of(
            "RubricChoiceView",
            &crate::commands::RubricChoiceView::refused(&refused_rubric),
        ),
    ];

    // La bandeja se pinta de lo que hay **en el fichero de estado**, y bajo el
    // sandbox lo que hay ahí son enlaces del portal: la fila se construye
    // leyendo uno, que es por donde entra en producción. `RecentDocument::seen`
    // no vale aquí porque canonicaliza, y este equipo no tiene el portal
    // montado.
    let entry: RecentDocument = serde_json::from_value(serde_json::json!({
        "path": A_PORTAL_HANDLE,
        "name": "contrato.pdf",
        "badge": serde_json::to_value(Badge::Unsigned).expect("la insignia serializa"),
        "modified": 1_700_000_000_u64,
        "last_used": 1_700_000_100_u64,
    }))
    .expect("la fila del fichero de estado deberia leerse");
    let mut state = State::default();
    state.recents.record(entry);
    memory
        .remember_state(&configuration, &state)
        .expect("deberia guardarse el estado");
    for row in recents::listed_rows(&memory, &opened) {
        crossings.push(Crossing::of("RecentDocumentView", &row));
    }

    crossings
}

/// **La ruta del portal no sale a la ventana, en ningún canal** (ADR-0011,
/// ID-186).
///
/// La guarda mira **valor y no texto**. La que había leía `commands/` como
/// fuente y buscaba `PathBuf`, `&Path` o un campo llamado `path`: eso vigila
/// cómo se declara un tipo, no lo que acaba dentro de él, así que un `String`
/// con una ruta pegada pasaba en verde. Y desde el ID-185 esa lectura además
/// estorba: fuera del sandbox la ruta real **sí** se enseña, y lo que está mal
/// no es que cruce una ruta, es que cruce la del portal.
///
/// Lo que se comprueba, entonces, es lo que de verdad manda el ADR-0011: bajo
/// el sandbox la aplicación **no conoce** la ruta del documento, así que
/// devolver el enlace de `/run/user/…` sería devolver una mentira.
#[test]
fn the_portal_path_never_crosses_to_the_window() {
    // No se cuenta cuántos son: la igualdad exacta de conjuntos de
    // `every_output_type_is_either_built_from_a_document_or_declared_without_one`
    // ya se pone roja si falta cualquiera, y sin un número que subir a mano.
    let crossings = crossings_from_a_portal_document();

    for crossing in &crossings {
        assert!(
            the_portal_path_inside(&crossing.json).is_none(),
            "«{}» ha cruzado con una ruta del portal dentro: {}",
            crossing.name,
            crossing.json
        );
    }
}

/// Y lo que se comprueba son **todos** los tipos de salida del módulo: los que
/// salen de un documento, construidos; los demás, declarados como tipos sin
/// documento detrás (ID-84).
#[test]
fn every_output_type_is_either_built_from_a_document_or_declared_without_one() {
    let outputs = outputs();
    let declared: usize = SOURCES
        .iter()
        .map(|(_, source)| serialising_derives(&attributes_on_one_line(production_half(source))))
        .sum();
    assert_eq!(
        outputs.len(),
        declared,
        "el modulo declara {declared} tipos serializables y el descubrimiento ha encontrado {}: \
         uno se esta escapando de la guarda",
        outputs.len()
    );
    assert!(
        outputs.len() >= 12,
        "los tipos de salida no se han encontrado: {}",
        outputs.len()
    );

    let built: BTreeSet<&str> = crossings_from_a_portal_document()
        .iter()
        .map(|crossing| crossing.name)
        .collect();
    let without: BTreeSet<&str> = OUTPUTS_WITH_NO_DOCUMENT_BEHIND.into_iter().collect();

    for output in &outputs {
        assert!(
            built.contains(output.name.as_str()) || without.contains(output.name.as_str()),
            "«{}» ({}) no se construye desde un documento del portal ni esta declarado como \
             tipo sin documento detras: la guarda de rutas no lo mira",
            output.name,
            output.file
        );
    }

    let known: BTreeSet<&str> = outputs.iter().map(|output| output.name.as_str()).collect();
    for name in built.iter().chain(without.iter()) {
        assert!(
            known.contains(name),
            "«{name}» ya no es un tipo de salida del modulo: sobra de la guarda"
        );
    }
}

/// El caso que **hoy pasa en verde y pasa a rojo** (TD-42): una ruta del portal
/// metida dentro de un campo, y no como el campo entero.
///
/// La guarda vieja no la veía —leía la fuente, no el valor—, y esta prueba es
/// la que dice que el cambio de texto a valor sirve para algo.
#[test]
fn a_portal_path_buried_inside_a_field_is_a_leak() {
    let value = serde_json::json!({
        "name": "contrato.pdf",
        "failure": {
            "situation": "documentUnreadable",
            "detail": format!("no se ha podido leer {A_PORTAL_HANDLE}: no such file"),
        },
        "rows": [{ "id": "0f1e", "note": A_PORTAL_HANDLE }],
    });

    assert!(
        the_portal_path_inside(&value).is_some(),
        "una ruta del portal dentro de un campo es una fuga, este donde este"
    );
}

/// Y el caso que hoy pasa en verde y **sigue pasando** (ID-185, TD-42): una
/// ruta de `$HOME` dentro de un campo.
///
/// Queda por escrito porque es el que cambia de significado: el argumento de
/// privacidad se retiró, así que la ruta real del documento se enseña como la
/// enseña cualquier aplicación de escritorio. Lo que la guarda persigue es la
/// del portal, que es la que no se conoce. `/run/user/1000/keyring` tampoco lo
/// es: el portal de documentos es `doc`, y no todo lo que cuelga del directorio
/// de ejecución.
#[test]
fn a_home_path_inside_a_field_is_not_a_leak() {
    let value = serde_json::json!({
        "name": "contrato.pdf",
        "path": "/home/quien/Contratos/contrato.pdf",
        "socket": "/run/user/1000/keyring/pkcs11",
    });

    assert_eq!(
        the_portal_path_inside(&value),
        None,
        "solo la ruta del portal es una fuga: la real se enseña"
    );
}

/// Los `.rs` que hay dentro de un directorio, **incluidos los de sus
/// subdirectorios**, con la ruta relativa al módulo (`views/mod.rs`).
///
/// Recorrer solo el primer nivel dejaría fuera un submódulo en directorio: sus
/// tipos de salida no aparecerían ni en `SOURCES` ni en esta guarda, que es la
/// forma silenciosa que tiene esto de degradarse.
fn rust_files_under(directory: &Path, prefix: &str) -> Vec<String> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(directory)
        .expect("el modulo de ordenes tiene que estar donde dice el manifiesto");
    for entry in entries {
        let entry = entry.expect("deberia leerse la entrada");
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        if entry.path().is_dir() {
            found.extend(rust_files_under(&entry.path(), &format!("{relative}/")));
        } else if name.ends_with(".rs") {
            found.push(relative);
        }
    }
    found
}

/// Y la lista de ficheros que recorre la guarda de arriba es **la del módulo
/// entero**: un fichero nuevo sin dar de alta aquí se llevaría sus tipos de
/// salida sin que nadie los mirara.
#[test]
fn the_list_of_files_covers_the_whole_module() {
    let listed: BTreeSet<&str> = SOURCES.iter().map(|(file, _)| *file).collect();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
    let present: BTreeSet<String> = rust_files_under(&directory, "")
        .into_iter()
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

/// La lista sigue cerrada (ID-59): ocho órdenes, las tres de los ajustes, las
/// tres de la bandeja, las dos del resumen tras firmar, la esquina PAdES del
/// diálogo de páginas sin sello (ID-105), el documento con el que se invocó a
/// la aplicación (ID-157) y el momento en el que está el trámite de sede
/// (ID-338).
///
/// **El conteo vive en la aserción y no en el nombre** (TD-11): cambiar el
/// número es la información, y cuatro sub-issues que renombraran la misma
/// prueba serían cuatro diffs que no dicen nada.
///
/// Cuenta el prefijo `#[tauri::command` y no el atributo entero porque varias
/// llevan `(async)`: lo que se cierra es cuántas órdenes hay, no cómo se
/// ejecuta cada una.
#[test]
fn the_list_of_commands_is_closed_and_this_is_how_long_it_is() {
    let orders: usize = SOURCES
        .iter()
        .map(|(_, source)| production_half(source).matches("#[tauri::command").count())
        .sum();

    assert_eq!(orders, 37, "la lista de ordenes es cerrada a proposito");
}

/// Cada orden del módulo, desde su atributo `#[tauri::command…]` hasta la
/// siguiente: el atributo, el nombre y el cuerpo.
///
/// Se trocea el fichero en vez de nombrar órdenes una a una porque lo que se
/// vigila —quién se cuelga del hilo del bucle de eventos— es una propiedad de
/// **cualquier** orden, incluida la que alguien añada mañana.
fn commands_of(source: &str) -> Vec<(&str, String, &str)> {
    let marker = "#[tauri::command";
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(marker) {
        rest = &rest[start..];
        let end = rest[marker.len()..]
            .find(marker)
            .map_or(rest.len(), |offset| offset + marker.len());
        let block = &rest[..end];
        let attribute = block.lines().next().unwrap_or_default();
        let name = block
            .split_once("pub fn ")
            .and_then(|(_, after)| after.split_once('('))
            .map(|(name, _)| name.trim().to_owned())
            .unwrap_or_else(|| "sin nombre".to_owned());
        found.push((attribute, name, block));
        rest = &rest[end..];
    }
    found
}

/// Y las que hablan con el disco o con el portal **no bloquean el hilo
/// principal**: `#[tauri::command]` a secas genera un cuerpo `Blocking` que
/// corre dentro del manejador del IPC —el hilo del bucle GTK—, y
/// `blocking_pick_file()` espera allí a un cierre que solo ese hilo puede
/// ejecutar. Punto muerto: la ventana se clava y el diálogo no aparece.
///
/// Las cinco de hoy se nombran porque están escritas y se sabe lo que hacen.
/// Las dos del resumen entran en la lista **aunque no llamen a ningún
/// `blocking_*`**: `open_path` del complemento `opener` es una llamada síncrona
/// a D-Bus, y esperar ahí la respuesta del portal clava la ventana igual que el
/// diálogo, sin que la mitad que se descubre sola las vea (TD-10). Y
/// `check_for_new_version` entra por lo mismo: su cuerpo no dice `blocking_`,
/// pero el puerto de red abre una conexión síncrona y esperarla en el hilo del
/// bucle de eventos clava la ventana en cada arranque (ID-182).
///
/// La regla general, en cambio, se **descubre**: toda orden cuyo cuerpo llame a
/// un `blocking_*` de un plugin tiene que ser `(async)`, se llame como se
/// llame. Sin esa mitad, una orden nueva con un diálogo dentro entraría con la
/// ventana clavada y las guardas en verde.
#[test]
fn every_command_that_touches_the_portal_runs_off_the_main_thread() {
    let source = production_half(source_of("mod.rs"));

    for command in [
        "pub fn open_document(",
        "pub fn read_document(",
        "pub fn open_signed_document(",
        "pub fn open_signed_folder(",
        "pub fn check_for_new_version(",
    ] {
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        let before = &source[..declaration];
        assert!(
            before.ends_with("#[tauri::command(async)]\n"),
            "«{command}» tiene que ser #[tauri::command(async)]"
        );
    }

    // Cuántas órdenes hay lo cierra `the_list_of_commands_is_closed_and_this_is_how_long_it_is`;
    // aquí solo se comprueba que el troceado las ve, porque un troceado que se
    // queda corto dejaría esta guarda en verde sin haber mirado nada.
    let commands = commands_of(source);
    assert!(
        commands.len() >= 14,
        "el troceado de ordenes ha encontrado {}: si no las ve todas, no vigila nada",
        commands.len()
    );
    for (attribute, name, block) in commands {
        if !block.contains("blocking_") {
            continue;
        }
        assert_eq!(
            attribute.trim(),
            "#[tauri::command(async)]",
            "«{name}» llama a un blocking_* de un plugin desde el hilo del bucle de \
             eventos: tiene que ser #[tauri::command(async)] o la ventana se clava sin error"
        );
    }
}

/// **Todas las órdenes del trámite de sede son `(async)`** (ID-337, TD-76).
///
/// Es la misma trampa que la guarda de arriba y **ninguna otra la vigila
/// aquí**: la mitad que se descubre sola mira los cuerpos que llaman a un
/// `blocking_*` de un complemento, y las del trámite no llaman a ninguno. Lo
/// que las cuelga es otra cosa: una orden sobre una `fn` que no es `async`
/// corre dentro del manejador del IPC, en el hilo del bucle de eventos, y
/// cerrar desde ahí la ventana que está preguntando es pedirle al bucle que se
/// espere a sí mismo.
#[test]
fn every_command_of_the_site_errand_runs_off_the_main_thread() {
    let source = production_half(source_of("mod.rs"));

    // Una lista con nombre, y no un literal en el `for`: las órdenes del
    // trámite (ID-336) entran aquí según se escriben, y la lista dice cuáles se
    // han mirado ya.
    const OF_THE_ERRAND: [&str; 6] = [
        "pub fn close_site_window(",
        "pub fn site_identify(",
        "pub fn site_decline(",
        // La que instala la CA local, que es la acción principal de la
        // pantalla de reparación (ID-329, ID-341).
        "pub fn install_local_ca(",
        // Las dos del callejón sin salida (ID-341). La de instalar es además
        // la única que cae de lleno en el ID-337: por debajo hay un
        // `blocking_pick_file`, y sin el `(async)` la ventana se clava sin
        // error. No la ve la guarda que se descubre sola porque su cuerpo
        // delega en `install_certificate` y no dice `blocking_` en ninguna
        // parte.
        "pub fn site_install_certificate(",
        "pub fn site_look_again(",
    ];

    for command in OF_THE_ERRAND {
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        assert!(
            source[..declaration].ends_with("#[tauri::command(async)]\n"),
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

/// Y el descubrimiento no se rompe porque `rustfmt` parta un `derive`.
///
/// Es la rendija por la que un tipo de salida volvía a pasar sin que nadie lo
/// mirara: la lista de `derive` se alarga, el formateador la parte en varias
/// líneas, y una lectura línea a línea ve el `#[derive(` sin su `Serialize`.
#[test]
fn a_derive_broken_across_lines_is_still_seen() {
    let broken = "#[derive(\n    Clone,\n    Debug,\n    Serialize,\n)]\npub struct Leaky {\n}\n";

    let flattened = attributes_on_one_line(broken);

    assert_eq!(
        serialising_derives(&flattened),
        1,
        "un derive partido en varias lineas sigue siendo un tipo de salida"
    );
    assert!(
        flattened
            .lines()
            .next()
            .is_some_and(|line| line.starts_with("#[derive(") && line.contains("Serialize")),
        "el atributo tiene que quedar en una sola linea: {flattened}"
    );
}

/// Y lo que no es un atributo se queda como estaba: aplanar no toca nada más.
#[test]
fn what_is_not_an_attribute_is_left_alone() {
    let source = "pub struct Plain {\n    name: String,\n}\n";

    assert_eq!(attributes_on_one_line(source), source);
}
