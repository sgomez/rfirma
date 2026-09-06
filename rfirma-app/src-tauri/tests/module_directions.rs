//! **La dirección de las dependencias entre módulos, escrita como prueba**
//! (ID-81, ID-82, ID-83, ADR-0017).
//!
//! **Grada A**: lee ficheros del repositorio y nada más. Sin token, sin
//! librería nativa y sin red.
//!
//! Rust no vigila la dirección *dentro* de un crate: `signing` podía importar
//! `ffi` mientras `ffi` importaba `signing`, y `destination` importar `memory`
//! mientras `memory` importaba `destination`, y todo compilaba. Por eso los dos
//! ciclos vivieron ahí sin que nadie se enterara hasta el #136, y por eso hace
//! falta una guarda: sin ella la pregunta «¿dónde pongo este tipo?» vuelve a no
//! tener respuesta deducible en cuanto alguien añada el `use` cómodo.
//!
//! Lo que se comprueba son las **aristas prohibidas**, no las permitidas. Son
//! de dos clases:
//!
//! - **Contra la capa** (ID-81): un módulo de dominio o de infraestructura que
//!   nombra a `app/` o a `commands/`, y `app/` que nombra a un cuerpo de orden
//!   en vez de a los tipos de frontera. La flecha va siempre hacia dentro.
//! - **Contra un hermano**: las dos direcciones que cerraron los ciclos del
//!   #136. `ffi` sigue importando `signing::SessionSeal` y `memory` sigue
//!   importando `destination`, que son infraestructura mirando al dominio y son
//!   las direcciones correctas.
//!
//! Se mira **la mitad de producción** de cada módulo, cortando por `mod tests`.
//! Un `use` dentro de `#[cfg(test)]` no participa en el grafo que se compila y
//! no cierra ningún ciclo: `destination/portal.rs` comprueba contra
//! `memory::RecentDocument` que el identificador del portal sigue a la ruta y no
//! al inodo, y esa comprobación es de las dos cosas a la vez.
//!
//! Se leen **las líneas `use crate::`** (TD-22), no los caminos absolutos escritos
//! en medio de una expresión: un `crate::app::algo()` en línea se le escapa. Es un
//! agujero conocido y estrecho —el estilo del backend es importar arriba y usar el
//! nombre corto— y taparlo pediría un análisis del árbol sintáctico, que es
//! justamente lo que `cargo-pup` habría dado y el ADR-0017 descarta por el
//! toolchain nightly que exige.
//!
//! Si esta prueba te ha puesto el PR en rojo, el `use` que has añadido no es el
//! problema: es el síntoma. Lo que has escrito pertenece al otro lado de la
//! flecha —normalmente a `app/`, que es donde viven las decisiones—.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Las capas del backend, de fuera adentro (ID-81, ADR-0017).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layer {
    /// `lib.rs` y `main.rs`: el cableado. Ve a todo el mundo por definición.
    CompositionRoot,
    /// `commands/`: el adaptador de Tauri. Puede nombrar a cualquiera.
    Adapter,
    /// `app/`: los casos de uso.
    UseCases,
    /// El resto: dominio e infraestructura. No miran hacia fuera.
    Domain,
}

/// Quién tiene prohibida la arista.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Origin {
    /// Una capa entera.
    Layer(Layer),
    /// Una carpeta concreta de `src/`, para las dos direcciones del #136.
    Module(&'static str),
    /// Todo lo que cuelga de esa ruta de `src/`, para las dos aristas del
    /// trámite (RD-12, #406).
    Under(&'static str),
}

/// Una arista prohibida: quién no puede nombrar a quién, adónde debería ir en
/// su lugar y qué decisión lo dice.
struct Direction {
    from: Origin,
    /// El primer segmento de `crate::…` que no puede aparecer dentro.
    forbidden: &'static str,
    /// Los caminos de ese segmento que **sí** están permitidos, por prefijo.
    except: &'static [&'static str],
    /// Hacia dónde apunta la flecha correcta, para que el fallo se lea sin
    /// abrir el ADR.
    instead: &'static str,
    /// La decisión que lo dice.
    reason: &'static str,
}

const DIRECTIONS: [Direction; 8] = [
    Direction {
        from: Origin::Layer(Layer::Domain),
        forbidden: "app",
        except: &[],
        instead: "`app/` nombra al dominio, nunca al reves: el caso de uso llama a este \
                  modulo y le pasa lo que haga falta como argumento",
        reason: "la dirección es hacia el dominio (ID-81)",
    },
    Direction {
        from: Origin::Layer(Layer::Domain),
        forbidden: "commands",
        except: &[],
        instead: "`commands/` nombra a `app/` y `app/` al dominio: lo que este modulo \
                  necesitaba de la ventana se lo tiene que dar quien le llama",
        reason: "la dirección es hacia el dominio, y el adaptador de Tauri es la capa \
                 mas externa de todas (ID-79, ID-81)",
    },
    Direction {
        from: Origin::Layer(Layer::UseCases),
        forbidden: "commands",
        // `app/` sí nombra los tipos de frontera —lo que la ventana manda y lo
        // que se le devuelve—, que viven en `commands/` por el ID-80. Lo que no
        // puede nombrar es un **cuerpo de orden**: eso sería el caso de uso
        // llamando a su propio adaptador.
        except: &["commands::views", "commands::orders", "commands::Failure"],
        instead: "el cuerpo de la orden llama al caso de uso, no al contrario; si hace \
                  falta compartir algo, baja a `app/` o a un tipo de frontera de \
                  `commands/views.rs`",
        reason: "`commands/` es el adaptador y `app/` lo que decide (ID-79, ID-81)",
    },
    Direction {
        from: Origin::Module("signing"),
        forbidden: "ffi",
        except: &[],
        instead: "`ffi` puede nombrar a `signing`, que es infraestructura mirando al \
                  dominio; lo que cruza la frontera nativa es `app/cycle.rs`",
        reason: "el ciclo trifásico es un caso de uso y vive en `app/cycle.rs`; \
                 `signing/` son reglas puras y no cruza la frontera nativa (ID-82)",
    },
    Direction {
        from: Origin::Module("destination"),
        forbidden: "memory",
        except: &[],
        instead: "`memory` puede nombrar a `destination`, que es la memoria guardando \
                  un concepto del destino; desenvolver la configuración lo hace `app/`",
        reason: "`DestinationFolder` es un concepto del destino y vive en \
                 `destination/`; desenvolver la configuración lo hace `app/` (ID-83)",
    },
    Direction {
        from: Origin::Under("app/errand/"),
        forbidden: "channel",
        // Los tres tipos que **cruzan** el puerto —el cometido con el que se
        // abre, el canal abierto y su error— son vocabulario del puerto aunque
        // vivan en `channel/`: los nombra `ports.rs` para declararlo, y nadie
        // mas del tramite los necesita.
        except: &[
            "channel::ChannelDuty",
            "channel::ChannelError",
            "channel::OpenChannel",
        ],
        instead: "el tramite recibe el transporte por su puerto (`app/errand/ports.rs`); \
                  el `wss` sobre el loopback es `app/transport.rs`, y quien lo instancia \
                  es la negociacion de arranque (`app/site.rs`)",
        reason: "el tramite no importa el transporte concreto, solo su puerto (RD-12, #406)",
    },
    Direction {
        from: Origin::Under("app/errand/"),
        forbidden: "app::codec",
        except: &[],
        instead: "el tramite recibe el codec por su puerto (`app/errand/ports.rs`); \
                  el de la version 4 es `app/codec.rs`, y quien lo instancia es la \
                  negociacion de arranque (`app/site.rs`)",
        reason: "el tramite no importa el codec concreto, solo su puerto (RD-12, #406)",
    },
    Direction {
        from: Origin::Under("app/errand/"),
        forbidden: "app::transport",
        except: &[],
        instead: "lo mismo que con `channel`: el transporte entra por el puerto",
        reason: "el tramite no importa el transporte concreto, solo su puerto (RD-12, #406)",
    },
];

/// Un fichero de producción del backend, listo para mirarle los `use`.
struct Module {
    /// Su ruta relativa a `src/`, que es como lo nombra el mapa y el mensaje.
    name: String,
    /// La carpeta de `src/` en la que vive, o `""` si cuelga de la raíz.
    folder: String,
    layer: Layer,
    /// Lo que hay antes de `mod tests`.
    source: String,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Los módulos **versionados** de `src/`, preguntándoselo a git.
///
/// Se le pregunta a git y no se recorre el árbol a mano por la misma razón que
/// en `single_cfg_os_site.rs`: un repositorio con árboles de trabajo enlazados
/// dentro —como los que usan los agentes— tiene copias enteras del código en
/// otras ramas, que no son este PR.
fn tracked_modules() -> Vec<Module> {
    let root = manifest_dir();
    let listing = Command::new("git")
        .args(["ls-files", "--", "src"])
        .current_dir(&root)
        .output()
        .expect("git deberia poder listar los ficheros versionados");
    assert!(
        listing.status.success(),
        "git ls-files ha fallado: {}",
        String::from_utf8_lossy(&listing.stderr)
    );

    let mut modules: Vec<Module> = String::from_utf8(listing.stdout)
        .expect("la lista de git deberia ser UTF-8")
        .lines()
        .filter(|entry| entry.ends_with(".rs"))
        // Un `tests.rs` es la mitad de pruebas de su carpeta, escrita en un
        // fichero aparte en vez de tras `mod tests`: no participa en el grafo
        // que se compila, igual que un `#[cfg(test)]` al pie.
        .filter(|entry| !entry.ends_with("/tests.rs"))
        .map(|entry| {
            let relative = entry
                .strip_prefix("src/")
                .expect("git deberia listar dentro de src/")
                .to_owned();
            let folder = relative
                .split_once('/')
                .map(|(folder, _)| folder.to_owned())
                .unwrap_or_default();
            let source = fs::read_to_string(root.join(entry))
                .unwrap_or_else(|error| panic!("deberia poder leerse {entry}: {error}"));
            Module {
                layer: layer_of(&relative, &folder),
                name: relative,
                folder,
                source: production_half(&source),
            }
        })
        .collect();

    assert!(
        modules.len() > 20,
        "el backend tiene mas de veinte modulos; git ha listado {}",
        modules.len()
    );
    modules.sort_by(|one, other| one.name.cmp(&other.name));
    modules
}

fn layer_of(relative: &str, folder: &str) -> Layer {
    match (relative, folder) {
        ("lib.rs" | "main.rs", _) => Layer::CompositionRoot,
        (_, "commands") => Layer::Adapter,
        (_, "app") => Layer::UseCases,
        _ => Layer::Domain,
    }
}

/// La mitad de producción de un módulo: lo que hay antes de `mod tests`.
fn production_half(source: &str) -> String {
    source
        .split_once("\nmod tests {")
        .map(|(before, _)| before)
        .unwrap_or(source)
        .to_owned()
}

/// Los caminos `crate::…` que **importa** una línea, ya desplegados.
///
/// Solo son importes las líneas `use`: los nombres prohibidos aparecen también
/// en los comentarios de contrato, y ahí decir «no importa `ffi`» es justamente
/// lo contrario de importarlo.
///
/// Se despliegan las llaves porque `use crate::{app, memory};` y
/// `use crate::commands::{views, Failure};` son la misma arista escrita corta, y
/// una guarda que solo mire el primer segmento acusa a la segunda sin motivo y
/// deja pasar la primera.
fn crate_imports(line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    if !(trimmed.starts_with("use ") || trimmed.starts_with("pub use ")) {
        return Vec::new();
    }
    let Some((_, after)) = trimmed.split_once("crate::") else {
        return Vec::new();
    };
    expand(after.trim_end().trim_end_matches(';'))
}

/// Despliega un camino de `use` con llaves en los caminos que nombra.
fn expand(path: &str) -> Vec<String> {
    let path = path.trim();
    let Some(brace) = path.find('{') else {
        return vec![head_of(path)];
    };

    let prefix = path[..brace].trim().trim_end_matches("::").to_owned();
    let inner = match closing_brace(&path[brace..]) {
        Some(end) => &path[brace + 1..brace + end],
        None => return vec![prefix],
    };

    split_at_top_level(inner)
        .into_iter()
        .flat_map(|item| expand(item.trim()))
        .map(|item| match item.as_str() {
            "self" | "" => prefix.clone(),
            other if prefix.is_empty() => other.to_owned(),
            other => format!("{prefix}::{other}"),
        })
        .collect()
}

/// El camino que hay hasta el primer separador: corta el alias (`as`), la coma
/// y lo que venga detrás.
fn head_of(path: &str) -> String {
    path.split([' ', ',', ';', '}'])
        .next()
        .unwrap_or_default()
        .trim_end_matches("::")
        .to_owned()
}

/// El desplazamiento de la llave que cierra la que abre en `0`.
fn closing_brace(from: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, character) in from.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Parte por las comas que **no** están dentro de unas llaves.
fn split_at_top_level(inner: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (offset, character) in inner.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&inner[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    items.push(&inner[start..]);
    items
        .into_iter()
        .filter(|item| !item.trim().is_empty())
        .collect()
}

/// ¿Le toca a este módulo esta dirección?
fn applies_to(direction: &Direction, module: &Module) -> bool {
    match direction.from {
        Origin::Layer(layer) => module.layer == layer,
        Origin::Module(folder) => module.folder == folder,
        Origin::Under(prefix) => module.name.starts_with(prefix),
    }
}

/// ¿Este camino importado cae dentro de lo que la dirección prohíbe?
fn is_forbidden(direction: &Direction, path: &str) -> bool {
    // Un segmento prohibe la carpeta entera; un camino con `::` prohibe solo
    // ese modulo y lo que cuelga de el (`app::codec`, `app::transport`).
    let hits = if direction.forbidden.contains("::") {
        path == direction.forbidden || path.starts_with(&format!("{}::", direction.forbidden))
    } else {
        path.split("::").next().unwrap_or_default() == direction.forbidden
    };
    if !hits {
        return false;
    }
    !direction
        .except
        .iter()
        .any(|allowed| path == *allowed || path.starts_with(&format!("{allowed}::")))
}

#[test]
fn no_module_imports_against_the_direction_of_the_layers() {
    let modules = tracked_modules();
    let mut offences = Vec::new();

    for direction in &DIRECTIONS {
        for module in modules
            .iter()
            .filter(|module| applies_to(direction, module))
        {
            for line in module.source.lines() {
                let against: Vec<String> = crate_imports(line)
                    .into_iter()
                    .filter(|path| is_forbidden(direction, path))
                    .collect();
                if against.is_empty() {
                    continue;
                }
                offences.push(format!(
                    "arista sobrante: `{}` -> `crate::{}`\n    {}\n  {}\n  la flecha va al reves: {}",
                    module.name,
                    against.join("`, `crate::"),
                    line.trim(),
                    direction.reason,
                    direction.instead,
                ));
            }
        }
    }

    assert!(
        offences.is_empty(),
        "{} arista(s) apuntan contra la direccion del ADR-0017:\n\n{}\n\n\
         No relajes la regla ni le anadas una excepcion: mueve la decision. Lo que \
         necesitaba ese `use` pertenece al otro lado de la flecha —casi siempre a \
         `app/`—, y este modulo debe recibirlo ya decidido como argumento.",
        offences.len(),
        offences.join("\n\n")
    );
}

/// Las direcciones **buenas** que los dos ciclos dejaron en pie.
///
/// Sin esto la guarda sería feliz con un backend en el que `ffi` y `memory`
/// hubieran dejado de hablar con el dominio: no habría aristas prohibidas
/// porque no habría aristas. Que la de al lado siga existiendo es lo que hace
/// que la prohibición signifique algo.
#[test]
fn the_directions_that_are_allowed_are_still_there() {
    let modules = tracked_modules();
    let imports = |name: &str| -> BTreeSet<String> {
        modules
            .iter()
            .find(|module| module.name == name)
            .unwrap_or_else(|| panic!("`{name}` deberia existir en src/"))
            .source
            .lines()
            .flat_map(crate_imports)
            .collect()
    };

    assert!(
        imports("ffi.rs")
            .iter()
            .any(|path| path.starts_with("signing::")),
        "`ffi.rs` deberia seguir importando de `signing` —hoy `SessionSeal`—: es la \
         direccion correcta, infraestructura mirando al dominio (ADR-0016)"
    );
    assert!(
        imports("memory/configuration.rs")
            .iter()
            .any(|path| path.starts_with("destination")),
        "`memory/configuration.rs` deberia seguir importando `destination`: es la \
         direccion correcta, la memoria guardando un concepto del destino (ID-83)"
    );
    assert!(
        imports("app/transport.rs")
            .iter()
            .any(|path| path.starts_with("channel")),
        "`app/transport.rs` deberia seguir importando `channel`: es el adaptador del \
         transporte, y es el unico sitio de `app/` que lo nombra por el tramite (RD-04)"
    );
    assert!(
        imports("app/site.rs")
            .iter()
            .any(|path| path == "app::codec::V4Codec"),
        "`app/site.rs` deberia seguir instanciando `V4Codec`: la negociacion de arranque \
         es el unico sitio que decide el codec (RD-05)"
    );
    assert!(
        imports("app/errand/desk.rs")
            .iter()
            .any(|path| path.starts_with("app::filtering")),
        "`app/errand/desk.rs` deberia seguir nombrando `app::filtering` con `crate::`: \
         la guarda solo lee `use crate::`, y el tramite escribe asi sus importaciones \
         para que las dos aristas prohibidas del RD-12 no se le escapen"
    );
}

/// Cada capa tiene módulos de verdad. Si `layer_of` deja de clasificar —porque
/// alguien renombra `app/`, por ejemplo—, la guarda pasaría en verde sin mirar
/// nada, que es la única forma en la que una prueba así puede mentir.
#[test]
fn every_layer_has_modules_to_watch() {
    let modules = tracked_modules();
    for layer in [
        Layer::CompositionRoot,
        Layer::Adapter,
        Layer::UseCases,
        Layer::Domain,
    ] {
        assert!(
            modules.iter().any(|module| module.layer == layer),
            "ninguna ruta de src/ cae en la capa {layer:?}; la guarda se ha quedado ciega"
        );
    }
}

/// Las formas de `use` que la guarda dice leer, escritas enteras.
const THE_FORMS_IT_READS: [(&str, &[&str]); 7] = [
    ("use crate::ffi::Bridge;", &["ffi::Bridge"]),
    (
        "pub use crate::app::signing::SigningSession;",
        &["app::signing::SigningSession"],
    ),
    (
        "use crate::app::{self, Environment};",
        &["app", "app::Environment"],
    ),
    ("use crate::{app, memory};", &["app", "memory"]),
    (
        "use crate::commands::{views, Failure};",
        &["commands::views", "commands::Failure"],
    ),
    (
        "    use crate::memory::Memory as Store;",
        &["memory::Memory"],
    ),
    (
        "use crate::commands::views::{store_name, CertificateView};",
        &[
            "commands::views::store_name",
            "commands::views::CertificateView",
        ],
    ),
];

#[test]
fn the_reader_understands_every_form_of_use_it_claims_to() {
    for (line, expected) in THE_FORMS_IT_READS {
        assert_eq!(
            crate_imports(line),
            expected
                .iter()
                .map(|path| path.to_string())
                .collect::<Vec<_>>(),
            "la guarda no lee bien esta linea: {line}"
        );
    }
}

/// Lo que **no** es un importe. Ninguna de estas líneas puede disparar la
/// guarda, o el PR rojo lo daría cualquier fichero con un comentario honesto.
const WHAT_MUST_NOT_TRIP_IT: [&str; 5] = [
    "//! `signing/mod.rs` **no importa** `crate::ffi` (ID-82).",
    "/// Ver `crate::app::cycle` para el recorrido entero.",
    "// use crate::app::Environment;",
    "    let path = \"crate::commands::views\";",
    "mod app;",
];

#[test]
fn the_reader_leaves_alone_what_is_not_an_import() {
    for line in WHAT_MUST_NOT_TRIP_IT {
        assert!(
            crate_imports(line).is_empty(),
            "la guarda toma por importe una linea que no lo es: {line}"
        );
    }
}
