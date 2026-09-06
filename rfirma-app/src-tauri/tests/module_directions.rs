//! Guarda de dirección entre capas y contextos del backend, leída de las rutas (ADR-0017, RD-03).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Un fichero de producción del backend, listo para mirarle los `use`.
struct Module {
    /// Su ruta relativa a `src/`, que es como lo nombra el mapa y el mensaje.
    name: String,
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
            let source = fs::read_to_string(root.join(entry))
                .unwrap_or_else(|error| panic!("deberia poder leerse {entry}: {error}"));
            Module {
                name: relative,
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

/// Una arista contra el RD-03: la línea que la nombra en la lista de deuda, y el mensaje entero.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Offence {
    edge: String,
    message: String,
}

/// Aristas contra la dirección en un árbol, cada una con su motivo y adónde mover la decisión.
fn offences_in(modules: &[Module]) -> Vec<Offence> {
    let mut offences = Vec::new();
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
                offences.push(Offence {
                    edge: format!("{} -> {}", module.name, path),
                    message: format!(
                        "arista sobrante: `{}` -> `crate::{}`\n    {}\n  {} de `{}` no puede nombrar eso (RD-03)\n  la flecha va al reves: {}",
                        module.name,
                        path,
                        line.trim(),
                        place.tier.folder(),
                        place.context,
                        instead,
                    ),
                });
            }
        }
    }
    offences
}

/// La lista de deuda: aristas `application -> adapters` (y hermanas) que la ola 2 vacía, una por línea.
const DEBT_FILE: &str = "tests/module_directions_debt.txt";

/// Las aristas de la lista, ya sin comentarios ni líneas vacías.
fn debt_in(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Lo que queda por explicar entre un árbol y su lista de deuda: aristas nuevas y líneas fósiles.
fn what_the_debt_does_not_explain(
    offences: &[Offence],
    debt: &[String],
) -> (Vec<Offence>, Vec<String>) {
    let unlisted = offences
        .iter()
        .filter(|offence| !debt.contains(&offence.edge))
        .cloned()
        .collect();
    let fossils = debt
        .iter()
        .filter(|edge| !offences.iter().any(|offence| offence.edge == **edge))
        .cloned()
        .collect();
    (unlisted, fossils)
}

#[test]
fn no_module_imports_against_the_direction_of_the_layers() {
    let offences = offences_in(&tracked_modules());
    if std::env::var_os("MODULE_DIRECTIONS_DUMP").is_some() {
        for offence in &offences {
            println!("{}", offence.edge);
        }
    }
    let debt = fs::read_to_string(manifest_dir().join(DEBT_FILE))
        .map(|text| debt_in(&text))
        .unwrap_or_default();

    let (unlisted, fossils) = what_the_debt_does_not_explain(&offences, &debt);

    assert!(
        unlisted.is_empty(),
        "{} arista(s) apuntan contra la direccion del ADR-0017 y no estan en {DEBT_FILE}:\n\n{}\n\n\
         No relajes la regla ni anadas la arista a la lista: mueve la decision. Lo que \
         necesitaba ese `use` pertenece al otro lado de la flecha —casi siempre a \
         los casos de uso—, y este modulo debe recibirlo ya decidido como argumento. \
         La lista de deuda solo mengua.",
        unlisted.len(),
        unlisted
            .iter()
            .map(|offence| offence.message.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    assert!(
        fossils.is_empty(),
        "{} linea(s) de {DEBT_FILE} ya no son una infraccion; borralas en esta misma PR:\n  {}",
        fossils.len(),
        fossils.join("\n  ")
    );
}

#[test]
fn an_edge_that_is_not_in_the_debt_list_turns_it_red() {
    let offences = offences_in(&synthetic_tree(
        "site/application/thing.rs",
        "use crate::site::adapters::channel::OpenChannel;\n",
    ));
    let listed =
        vec!["site/application/thing.rs -> site::adapters::channel::OpenChannel".to_owned()];

    let (unlisted, fossils) = what_the_debt_does_not_explain(&offences, &listed);
    assert!(
        unlisted.is_empty() && fossils.is_empty(),
        "{unlisted:?} {fossils:?}"
    );

    let (unlisted, _) = what_the_debt_does_not_explain(&offences, &[]);
    assert_eq!(
        unlisted, offences,
        "sin la linea en la lista, la arista sigue siendo roja"
    );
}

#[test]
fn a_debt_line_that_is_no_longer_an_offence_turns_it_red() {
    let offences = offences_in(&synthetic_tree(
        "site/application/thing.rs",
        "use crate::site::ports::Transport;\n",
    ));
    assert!(offences.is_empty(), "el puerto ya esta puesto");

    let fossil = "site/application/thing.rs -> site::adapters::channel::OpenChannel".to_owned();
    let (_, fossils) = what_the_debt_does_not_explain(&offences, std::slice::from_ref(&fossil));
    assert_eq!(fossils, [fossil]);
}

#[test]
fn the_debt_list_reads_past_comments_and_blank_lines() {
    assert_eq!(
        debt_in("# la ola 2 vacia esto\n\n  a/application/x.rs -> a::adapters::y  \n"),
        ["a/application/x.rs -> a::adapters::y"]
    );
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
fn a_module_outside_every_context_is_left_alone() {
    let offences = |name: &str, source: &str| offences_in(&synthetic_tree(name, source));

    assert!(
        offences(
            "commands/failure.rs",
            "use crate::site::application::errand::Errand;\n"
        )
        .is_empty(),
        "`commands/` es raiz y la guarda lo tolera"
    );
    assert_eq!(
        offences(
            "site/application/thing.rs",
            "use crate::commands::Failure;\n"
        )
        .len(),
        1,
        "pero un caso de uso sigue sin poder nombrarlo"
    );
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
                    let offence = &offences[0].message;
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
