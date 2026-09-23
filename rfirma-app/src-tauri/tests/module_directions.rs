//! Guarda de dirección entre capas y contextos del backend, leída de las rutas (ADR-0017, RD-03).

#[path = "module_directions/support.rs"]
mod support;

use support::{
    contexts_among, crate_imports, keeps_the_world_out, offences_in, place_modules, place_of,
    tracked_modules, world_offences_in, Module, Tier, THE_WORLD,
};

#[test]
fn the_world_only_gets_in_through_a_port() {
    let offences = world_offences_in(&tracked_modules());

    assert!(
        offences.is_empty(),
        "{} sitio(s) tocan el mundo desde dentro:\n\n{}\n\n\
         No relajes la regla: el dominio y los casos de uso reciben hechos y puertos, \
         nunca el disco, el entorno ni un tipo de la ventana.",
        offences.len(),
        offences
            .iter()
            .map(|offence| offence.message.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    );
}

#[test]
fn no_module_imports_against_the_direction_of_the_layers() {
    let offences = offences_in(&tracked_modules());
    if std::env::var_os("MODULE_DIRECTIONS_DUMP").is_some() {
        for offence in &offences {
            println!("{}", offence.edge);
        }
    }

    assert!(
        offences.is_empty(),
        "{} arista(s) apuntan contra la direccion del ADR-0017:\n\n{}\n\n\
         No relajes la regla: mueve la decision. Lo que necesitaba ese `use` pertenece \
         al otro lado de la flecha —casi siempre a los casos de uso—, y este modulo \
         debe recibirlo ya decidido como argumento.",
        offences.len(),
        offences
            .iter()
            .map(|offence| offence.message.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
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
        "use crate::crossing::{failure, Failure};",
        &["crossing::failure", "crossing::Failure"],
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
    "    let path = \"crate::crossing::Failure\";",
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
        "crossing.rs".to_owned(),
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
    assert_eq!(place("crossing.rs"), None);
    assert_eq!(place("lib.rs"), None);
}

#[test]
fn a_module_outside_every_context_is_left_alone() {
    let offences = |name: &str, source: &str| offences_in(&synthetic_tree(name, source));

    assert!(
        offences(
            "crossing/guards.rs",
            "use crate::site::application::errand::Errand;\n"
        )
        .is_empty(),
        "lo que cuelga de la raiz junta contextos, y la guarda lo tolera"
    );
    assert!(
        offences(
            "site/application/thing.rs",
            "use crate::crossing::Failure;\n"
        )
        .is_empty(),
        "lo que cruza a la ventana no es de ningun contexto: un caso de uso puede nombrarlo"
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

#[test]
fn naming_the_world_from_a_layer_that_cannot_touch_it_turns_red_with_a_hint() {
    for context in CONTEXTS {
        for tier in TIERS {
            for (name, _) in THE_WORLD {
                let source = format!("use {name}::whatever;");
                let tree = synthetic_tree(&synthetic_path(context, tier), &source);

                let offences = world_offences_in(&tree);

                if keeps_the_world_out(tier) {
                    let message = offences
                        .first()
                        .unwrap_or_else(|| {
                            panic!("`{name}` en {}/{tier:?} tenia que ponerse rojo", context)
                        })
                        .message
                        .clone();
                    assert!(
                        message.contains(&format!("{context}/ports.rs"))
                            && message.contains(&format!("{context}/adapters/")),
                        "el mensaje tiene que decir adonde mover la decision: {message}"
                    );
                } else {
                    assert!(
                        offences.is_empty(),
                        "`{name}` en {context}/{tier:?} es donde tiene que estar: {offences:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn what_only_looks_like_the_world_is_left_alone() {
    for source in [
        "use std::io::ErrorKind;",
        "use std::path::PathBuf;",
        "/// El adaptador de Tauri traduce esto con std::fs.",
        "// std::env::temp_dir() vive en la raiz de composicion",
    ] {
        let tree = synthetic_tree(&synthetic_path("documents", Tier::Application), source);

        assert!(
            world_offences_in(&tree).is_empty(),
            "«{source}» no toca el mundo"
        );
    }
}
