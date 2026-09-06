//! Guarda de dirección de dependencias entre módulos del backend (ADR-0017).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Capas del backend (ADR-0017).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layer {
    CompositionRoot,
    Adapter,
    UseCases,
    Domain,
}

/// Ámbito de aplicación de una arista prohibida.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Origin {
    Layer(Layer),
    Module(&'static str),
    Under(&'static str),
}

/// Arista prohibida entre módulos.
struct Direction {
    from: Origin,
    forbidden: &'static str,
    except: &'static [&'static str],
    instead: &'static str,
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
    /// Su sitio en el árbol por contextos, si vive en él (RD-02).
    place: Option<Place>,
    source: String,
}

/// Capa de un contexto, leída de su ruta (RD-02).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tier {
    Root,
    Domain,
    Ports,
    Application,
    Adapters,
}

impl Tier {
    fn of_segment(segment: &str) -> Option<Self> {
        match segment.trim_end_matches(".rs") {
            "domain" => Some(Self::Domain),
            "ports" => Some(Self::Ports),
            "application" => Some(Self::Application),
            "adapters" => Some(Self::Adapters),
            _ => None,
        }
    }

    fn folder(self) -> &'static str {
        match self {
            Self::Root => "mod.rs",
            Self::Domain => "domain/",
            Self::Ports => "ports.rs",
            Self::Application => "application/",
            Self::Adapters => "adapters/",
        }
    }
}

/// Contexto y capa de un módulo del árbol nuevo.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Place {
    context: String,
    tier: Tier,
}

/// Carpetas de `src/` que tienen al menos una capa dentro: esas son contextos.
fn contexts_among(names: &[String]) -> BTreeSet<String> {
    names
        .iter()
        .filter_map(|name| {
            let (folder, rest) = name.split_once('/')?;
            let segment = rest.split('/').next()?;
            Tier::of_segment(segment).map(|_| folder.to_owned())
        })
        .collect()
}

/// Sitio de un módulo en el árbol por contextos, o nada si es del árbol antiguo.
fn place_of(name: &str, contexts: &BTreeSet<String>) -> Option<Place> {
    let (folder, rest) = name.split_once('/')?;
    if !contexts.contains(folder) {
        return None;
    }
    let segment = rest.split('/').next().unwrap_or_default();
    let tier = Tier::of_segment(segment).unwrap_or(Tier::Root);
    Some(Place {
        context: folder.to_owned(),
        tier,
    })
}

/// Adónde apunta un camino `crate::…`.
fn target_of(path: &str, contexts: &BTreeSet<String>) -> Option<Place> {
    let mut segments = path.split("::");
    let folder = segments.next().unwrap_or_default();
    if !contexts.contains(folder) {
        return None;
    }
    let tier = segments
        .next()
        .and_then(Tier::of_segment)
        .unwrap_or(Tier::Root);
    Some(Place {
        context: folder.to_owned(),
        tier,
    })
}

/// Por qué una arista del árbol por contextos está prohibida, y adónde mover la decisión (RD-03).
fn context_offence(from: &Place, path: &str, contexts: &BTreeSet<String>) -> Option<String> {
    let context = &from.context;
    let target = target_of(path, contexts);
    let same = target.as_ref().is_some_and(|to| to.context == *context);
    let to_tier = target.as_ref().map(|to| to.tier);
    let other = target
        .as_ref()
        .map(|to| to.context.clone())
        .unwrap_or_default();
    match from.tier {
        Tier::Domain if same && to_tier == Some(Tier::Domain) => None,
        Tier::Domain => Some(format!(
            "`{context}/domain/` no nombra nada del crate fuera de si mismo: lo que \
             necesitaba de `crate::{path}` se lo pasa como argumento `{context}/application/`"
        )),
        Tier::Ports if to_tier == Some(Tier::Domain) || (same && to_tier == Some(Tier::Ports)) => {
            None
        }
        Tier::Ports if target.is_none() && !path.starts_with("commands") => None,
        Tier::Ports => Some(format!(
            "un puerto de `{context}` solo habla en tipos de dominio: mueve lo que \
             necesitaba de `crate::{path}` a un `domain/` o hazlo entrar por el puerto ya decidido"
        )),
        Tier::Application if target.is_none() && !path.starts_with("commands") => None,
        Tier::Application if target.is_none() => Some(
            "el cuerpo de la orden llama al caso de uso, no al contrario: lo que \
             necesitaba de `commands/` se lo tiene que dar quien le llama"
                .to_owned(),
        ),
        Tier::Application
            if same
                && matches!(
                    to_tier,
                    Some(Tier::Domain | Tier::Ports | Tier::Application)
                ) =>
        {
            None
        }
        Tier::Application if to_tier == Some(Tier::Domain) => None,
        Tier::Application if to_tier == Some(Tier::Adapters) => Some(format!(
            "un caso de uso no conoce al adaptador: lo que hace `crate::{path}` entra por \
             un puerto de `{context}/ports.rs`, y lo instancia la raiz de composicion"
        )),
        Tier::Application if same => Some(format!(
            "la raiz de `{context}` compone los casos de uso, no al reves: lo que \
             necesitaba de `crate::{path}` entra como argumento"
        )),
        Tier::Application => Some(format!(
            "un contexto no importa los casos de uso ni los puertos de otro: lo que \
             `{context}` necesitaba de `{other}` es dominio de `{other}/domain/` o se lo \
             pasa la raiz de composicion ya decidido"
        )),
        Tier::Adapters | Tier::Root if !same && to_tier == Some(Tier::Application) => {
            Some(format!(
                "`{context}` recibe los casos de uso de `{other}` desde la raiz de composicion \
             global, que es la unica que junta contextos; no los importa"
            ))
        }
        Tier::Adapters | Tier::Root => None,
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Módulos versionados de src obtenidos desde git.
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
                place: None,
                source,
            }
        })
        .collect();
    place_modules(&mut modules);

    assert!(
        modules.len() > 20,
        "el backend tiene mas de veinte modulos; git ha listado {}",
        modules.len()
    );
    modules.sort_by(|one, other| one.name.cmp(&other.name));
    modules
}

/// Asigna a cada módulo su sitio en el árbol por contextos, si lo tiene.
fn place_modules(modules: &mut [Module]) {
    let names: Vec<String> = modules.iter().map(|module| module.name.clone()).collect();
    let contexts = contexts_among(&names);
    for module in modules.iter_mut() {
        module.place = place_of(&module.name, &contexts);
    }
}

fn layer_of(relative: &str, folder: &str) -> Layer {
    match (relative, folder) {
        ("lib.rs" | "main.rs", _) => Layer::CompositionRoot,
        (_, "commands") => Layer::Adapter,
        (_, "app") => Layer::UseCases,
        _ => Layer::Domain,
    }
}

/// Caminos crate:: que importa una línea, ya desplegados.
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

/// Despliega un camino de use con llaves.
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

/// Camino hasta el primer separador.
fn head_of(path: &str) -> String {
    path.split([' ', ',', ';', '}'])
        .next()
        .unwrap_or_default()
        .trim_end_matches("::")
        .to_owned()
}

/// Desplazamiento de la llave de cierre.
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

/// Divide por comas que no están dentro de llaves.
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

/// Comprueba si le toca a este módulo esta dirección.
fn applies_to(direction: &Direction, module: &Module) -> bool {
    if module.place.is_some() {
        return false;
    }
    match direction.from {
        Origin::Layer(layer) => module.layer == layer,
        Origin::Module(folder) => module.folder == folder,
        Origin::Under(prefix) => module.name.starts_with(prefix),
    }
}

/// Comprueba si este camino importado cae dentro de lo prohibido.
fn is_forbidden(direction: &Direction, path: &str) -> bool {
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

/// Aristas contra la dirección en un árbol, cada una con su motivo y adónde mover la decisión.
fn offences_in(modules: &[Module]) -> Vec<String> {
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

    let names: Vec<String> = modules.iter().map(|module| module.name.clone()).collect();
    let contexts = contexts_among(&names);
    for module in modules.iter() {
        let Some(place) = &module.place else {
            continue;
        };
        for line in module.source.lines() {
            for path in crate_imports(line) {
                let Some(instead) = context_offence(place, &path, &contexts) else {
                    continue;
                };
                offences.push(format!(
                    "arista sobrante: `{}` -> `crate::{}`\n    {}\n  {} de `{}` no puede nombrar eso (RD-03)\n  la flecha va al reves: {}",
                    module.name,
                    path,
                    line.trim(),
                    place.tier.folder(),
                    place.context,
                    instead,
                ));
            }
        }
    }

    offences
}

#[test]
fn no_module_imports_against_the_direction_of_the_layers() {
    let offences = offences_in(&tracked_modules());

    assert!(
        offences.is_empty(),
        "{} arista(s) apuntan contra la direccion del ADR-0017:\n\n{}\n\n\
         No relajes la regla ni le anadas una excepcion: mueve la decision. Lo que \
         necesitaba ese `use` pertenece al otro lado de la flecha —casi siempre a \
         los casos de uso—, y este modulo debe recibirlo ya decidido como argumento.",
        offences.len(),
        offences.join("\n\n")
    );
}

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

/// Formas de use que la guarda lee.
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

/// Líneas que no son un importe.
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

/// Los cinco contextos del RD-01.
const CONTEXTS: [&str; 5] = ["site", "signing", "documents", "identity", "desktop"];

const TIERS: [Tier; 5] = [
    Tier::Root,
    Tier::Domain,
    Tier::Ports,
    Tier::Application,
    Tier::Adapters,
];

/// Ruta de un módulo sintético en esa capa de ese contexto.
fn synthetic_path(context: &str, tier: Tier) -> String {
    match tier {
        Tier::Root => format!("{context}/mod.rs"),
        Tier::Domain => format!("{context}/domain/thing.rs"),
        Tier::Ports => format!("{context}/ports.rs"),
        Tier::Application => format!("{context}/application/thing.rs"),
        Tier::Adapters => format!("{context}/adapters/thing.rs"),
    }
}

/// Camino `crate::` que apunta a esa capa de ese contexto.
fn synthetic_target(context: &str, tier: Tier) -> String {
    match tier {
        Tier::Root => format!("{context}::Root"),
        Tier::Domain => format!("{context}::domain::Thing"),
        Tier::Ports => format!("{context}::ports::Thing"),
        Tier::Application => format!("{context}::application::Thing"),
        Tier::Adapters => format!("{context}::adapters::Thing"),
    }
}

/// Un árbol con los cinco contextos y un solo módulo con código: el que importa.
fn synthetic_tree(name: &str, source: &str) -> Vec<Module> {
    let mut modules: Vec<Module> = CONTEXTS
        .iter()
        .map(|context| (format!("{context}/domain/mod.rs"), String::new()))
        .chain([(name.to_owned(), source.to_owned())])
        .map(|(name, source)| Module {
            folder: name.split('/').next().unwrap_or_default().to_owned(),
            layer: Layer::Domain,
            place: None,
            name,
            source,
        })
        .collect();
    place_modules(&mut modules);
    modules
}

/// La regla del RD-03 escrita del derecho, para contrastar la guarda.
fn rd03_allows(from: Tier, same_context: bool, to: Tier) -> bool {
    match from {
        Tier::Domain => same_context && to == Tier::Domain,
        Tier::Ports => to == Tier::Domain || (same_context && to == Tier::Ports),
        Tier::Application => {
            to == Tier::Domain || (same_context && matches!(to, Tier::Ports | Tier::Application))
        }
        Tier::Adapters | Tier::Root => same_context || to != Tier::Application,
    }
}

#[test]
fn a_context_is_recognised_by_a_layer_in_its_path() {
    let contexts = contexts_among(&[
        "site/domain/errand.rs".to_owned(),
        "site/mod.rs".to_owned(),
        "signing/mod.rs".to_owned(),
        "identity/ports.rs".to_owned(),
        "commands/mod.rs".to_owned(),
    ]);
    assert_eq!(
        contexts,
        ["site", "identity"]
            .map(str::to_owned)
            .into_iter()
            .collect()
    );

    let place = |name: &str| place_of(name, &contexts).map(|place| place.tier);
    assert_eq!(place("site/domain/errand.rs"), Some(Tier::Domain));
    assert_eq!(place("site/domain.rs"), Some(Tier::Domain));
    assert_eq!(place("site/ports.rs"), Some(Tier::Ports));
    assert_eq!(place("site/application/attend.rs"), Some(Tier::Application));
    assert_eq!(place("site/adapters/tauri/mod.rs"), Some(Tier::Adapters));
    assert_eq!(place("site/mod.rs"), Some(Tier::Root));
    assert_eq!(
        place("signing/mod.rs"),
        None,
        "una carpeta sin capas dentro sigue siendo del arbol antiguo"
    );
    assert_eq!(place("commands/mod.rs"), None);
    assert_eq!(place("lib.rs"), None);
}

#[test]
fn every_forbidden_edge_between_layers_and_contexts_turns_red_with_a_hint() {
    let mut red = 0usize;
    for from_context in CONTEXTS {
        for from_tier in TIERS {
            for to_context in CONTEXTS {
                for to_tier in TIERS {
                    let same = from_context == to_context;
                    let name = synthetic_path(from_context, from_tier);
                    let target = synthetic_target(to_context, to_tier);
                    let offences =
                        offences_in(&synthetic_tree(&name, &format!("use crate::{target};\n")));
                    let edge = format!("`{name}` -> `crate::{target}`");
                    if rd03_allows(from_tier, same, to_tier) {
                        assert!(offences.is_empty(), "{edge} esta permitida: {offences:?}");
                        continue;
                    }
                    red += 1;
                    assert_eq!(
                        offences.len(),
                        1,
                        "{edge} deberia dar una arista: {offences:?}"
                    );
                    let offence = &offences[0];
                    assert!(offence.contains(&edge), "{offence}");
                    assert!(offence.contains("RD-03"), "{offence}");
                    assert!(
                        offence.contains("la flecha va al reves: "),
                        "la arista tiene que decir adonde mover la decision: {offence}"
                    );
                }
            }
        }
    }
    assert!(
        red > 100,
        "la regla prohibe mas de cien combinaciones; se han visto {red}"
    );
}

#[test]
fn the_old_tree_is_reached_from_the_new_one_under_the_old_rule() {
    let offences = |name: &str, source: &str| offences_in(&synthetic_tree(name, source));

    assert_eq!(
        offences("site/domain/thing.rs", "use crate::memory::Memory;\n").len(),
        1,
        "el dominio no nombra nada del crate, tampoco del arbol antiguo"
    );
    assert!(
        offences("site/application/thing.rs", "use crate::memory::Memory;\n").is_empty(),
        "un caso de uso puede seguir usando un modulo antiguo mientras no se mueva"
    );
    assert_eq!(
        offences(
            "site/application/thing.rs",
            "use crate::commands::Failure;\n"
        )
        .len(),
        1,
        "el adaptador antiguo sigue siendo el adaptador"
    );
    assert!(
        offences("site/adapters/thing.rs", "use crate::commands::Failure;\n").is_empty(),
        "un adaptador puede nombrar lo que quiera"
    );
    assert!(
        offences(
            "site/adapters/thing.rs",
            "use crate::app::Environment;\nuse crate::commands::views::View;\n"
        )
        .is_empty(),
        "la regla de carpeta fija no alcanza al arbol por contextos"
    );
}
