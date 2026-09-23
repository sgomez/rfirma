//! Modelo de módulo y reglas de dirección que consultan las pruebas de `module_directions.rs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Un fichero de producción del backend, listo para mirarle los `use`.
pub(crate) struct Module {
    /// Su ruta relativa a `src/`, que es como lo nombra el mapa y el mensaje.
    pub(crate) name: String,
    /// Su sitio en el árbol por contextos, si vive en él (RD-02).
    pub(crate) place: Option<Place>,
    pub(crate) source: String,
}

/// Capa de un contexto, leída de su ruta (RD-02).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tier {
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

    pub(crate) fn folder(self) -> &'static str {
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
pub(crate) struct Place {
    pub(crate) context: String,
    pub(crate) tier: Tier,
}

/// Carpetas de `src/` que tienen al menos una capa dentro: esas son contextos.
pub(crate) fn contexts_among(names: &[String]) -> BTreeSet<String> {
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
pub(crate) fn place_of(name: &str, contexts: &BTreeSet<String>) -> Option<Place> {
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
        Tier::Ports if target.is_none() => None,
        Tier::Ports => Some(format!(
            "un puerto de `{context}` solo habla en tipos de dominio: mueve lo que \
             necesitaba de `crate::{path}` a un `domain/` o hazlo entrar por el puerto ya decidido"
        )),
        Tier::Application if target.is_none() => None,
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
pub(crate) fn tracked_modules() -> Vec<Module> {
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
pub(crate) fn place_modules(modules: &mut [Module]) {
    let names: Vec<String> = modules.iter().map(|module| module.name.clone()).collect();
    let contexts = contexts_among(&names);
    for module in modules.iter_mut() {
        module.place = place_of(&module.name, &contexts);
    }
}

/// Caminos crate:: que importa una línea, ya desplegados.
pub(crate) fn crate_imports(line: &str) -> Vec<String> {
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

/// Lo del mundo que la guarda de dirección no ve, porque no se nombra con `crate::` (ADR-0017).
pub(crate) const THE_WORLD: [(&str, &str); 6] = [
    ("std::fs", "el disco"),
    ("std::env", "el entorno del proceso"),
    ("std::process", "el proceso"),
    ("libloading", "una biblioteca cargada en memoria"),
    ("tauri_plugin_", "un complemento de Tauri"),
    ("tauri", "Tauri"),
];

/// Capas donde el mundo no entra: solo por un puerto (ADR-0017).
pub(crate) fn keeps_the_world_out(tier: Tier) -> bool {
    matches!(tier, Tier::Domain | Tier::Ports | Tier::Application)
}

/// Lo del mundo que nombra una línea de código, si nombra algo.
fn world_named_in(line: &str) -> Option<(&'static str, &'static str)> {
    let code = line.trim();
    if code.starts_with("//") {
        return None;
    }
    THE_WORLD.into_iter().find(|(name, _)| code.contains(name))
}

/// Aristas hacia el mundo desde una capa que no puede tocarlo.
pub(crate) fn world_offences_in(modules: &[Module]) -> Vec<Offence> {
    let mut offences = Vec::new();
    for module in modules.iter() {
        let Some(place) = &module.place else {
            continue;
        };
        if !keeps_the_world_out(place.tier) {
            continue;
        }
        for line in module.source.lines() {
            let Some((name, what)) = world_named_in(line) else {
                continue;
            };
            offences.push(Offence {
                edge: format!("{} -> {name}", module.name),
                message: format!(
                    "el mundo entra por la puerta de atras: `{}` nombra `{name}`\n    {}\n                       {} de `{}` no puede tocar {what} (RD-02)\n                       declara un puerto en `{}/ports.rs`, ponle su adaptador en `{}/adapters/`                      y que el caso de uso lo reciba como `&dyn Puerto`",
                    module.name,
                    line.trim(),
                    place.tier.folder(),
                    place.context,
                    place.context,
                    place.context,
                ),
            });
        }
    }
    offences
}

/// Una arista contra el RD-03: la línea que la nombra en la lista de deuda, y el mensaje entero.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Offence {
    pub(crate) edge: String,
    pub(crate) message: String,
}

/// Aristas contra la dirección en un árbol, cada una con su motivo y adónde mover la decisión.
pub(crate) fn offences_in(modules: &[Module]) -> Vec<Offence> {
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
