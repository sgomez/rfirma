//! La Selección: un texto de ids y nombres de conjunto resuelto contra el catálogo y, si lo hay, el alcance de un informe.

use std::collections::BTreeSet;

use serde::Serialize;
use ts_rs::TS;

use crate::catalogue::Check;
use crate::checks::the_greeting_opens;
use crate::errand::ErrandKey;

/// Lo que resuelve un texto de selección, en el orden del catálogo salvo los nombres, que van en el del texto.
#[derive(Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) struct SelectionResolution {
    /// Lo nombrado más los saludos añadidos.
    pub ids: Vec<String>,
    pub greetings_added: Vec<String>,
    pub unknown: Vec<String>,
    pub out_of_scope: Vec<String>,
    /// Las de fuera de la selección, dentro del alcance, con el mismo trámite que alguna de dentro.
    pub sharing_an_errand: Vec<String>,
}

/// Un conjunto del catálogo con los ids de sus comprobaciones, para el árbol de la consola.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct CatalogueSet<'a> {
    pub name: &'a str,
    pub checks: Vec<&'a str>,
}

/// Los conjuntos del catálogo en su orden, cada uno con sus comprobaciones.
pub(crate) fn the_sets_of(catalogue: &[Check]) -> Vec<CatalogueSet<'_>> {
    let mut sets: Vec<CatalogueSet> = Vec::new();
    for check in catalogue {
        match sets.last_mut() {
            Some(set) if set.name == check.requirement.set => set.checks.push(&check.id),
            _ => sets.push(CatalogueSet {
                name: &check.requirement.set,
                checks: vec![&check.id],
            }),
        }
    }
    sets
}

/// Resuelve `text` contra el catálogo; con `scope`, solo dentro de lo que resuelve el alcance.
pub(crate) fn resolve(text: &str, catalogue: &[Check], scope: Option<&str>) -> SelectionResolution {
    let reachable: Vec<&Check> = match scope {
        Some(scope) => {
            let scoped = resolve(scope, catalogue, None).ids;
            catalogue
                .iter()
                .filter(|check| scoped.contains(&check.id))
                .collect()
        }
        None => catalogue.iter().collect(),
    };
    let mut resolution = SelectionResolution::default();
    let mut named = BTreeSet::new();
    for name in names_in(text) {
        let matches: Vec<&str> = reachable
            .iter()
            .filter(|check| is_named_by(check, name))
            .map(|check| check.id.as_str())
            .collect();
        if !matches.is_empty() {
            named.extend(matches);
        } else if catalogue.iter().any(|check| is_named_by(check, name)) {
            push_once(&mut resolution.out_of_scope, name);
        } else {
            push_once(&mut resolution.unknown, name);
        }
    }
    let is_named = |check: &Check| named.contains(check.id.as_str());
    let added: BTreeSet<&str> = reachable
        .iter()
        .filter(|greeting| greeting.greeting() && !is_named(greeting))
        .filter(|greeting| {
            reachable
                .iter()
                .any(|check| is_named(check) && the_greeting_opens(greeting, check))
        })
        .map(|greeting| greeting.id.as_str())
        .collect();
    let selected = |check: &Check| is_named(check) || added.contains(check.id.as_str());
    let keys: BTreeSet<ErrandKey> = reachable
        .iter()
        .filter(|check| selected(check))
        .filter_map(|check| ErrandKey::of(check))
        .collect();
    for check in &reachable {
        if selected(check) {
            resolution.ids.push(check.id.clone());
        } else if ErrandKey::of(check).is_some_and(|key| keys.contains(&key)) {
            resolution.sharing_an_errand.push(check.id.clone());
        }
        if added.contains(check.id.as_str()) {
            resolution.greetings_added.push(check.id.clone());
        }
    }
    resolution
}

fn names_in(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|name| !name.is_empty())
}

/// Si `name` es su id, su conjunto o un prefijo de su conjunto por segmentos completos.
fn is_named_by(check: &Check, name: &str) -> bool {
    let set = check.requirement.set.as_str();
    check.id == name
        || set
            .strip_prefix(name)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

fn push_once(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|known| known == name) {
        names.push(name.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::the_catalogue_in;
    use crate::manifest::Family;

    fn an_entry(id: &str, set: &str, script: &str, extra: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"{set}\"\nchapter = \"01\"\ncitation = \"A.java:1\"\n\
             statement = \"Algo.\"\n\n[check.drive]\nmode = \"v4\"\nscript = \"{script}\"\n\
             expects.completes = {{}}\n{extra}\n"
        )
    }

    const CLICKED: &str = "act.consent = \"Elige.\"";

    fn a_small_catalogue() -> Vec<Check> {
        let entries = [
            an_entry(
                "ws_greeting",
                "canal.websocket",
                "protocol-v4",
                "greeting = true",
            ),
            an_entry("ws_echo", "canal.websocket", "protocol-v4-echo", ""),
            an_entry("service_one", "canal.servicio", "protocol-service", ""),
            an_entry("relay_one", "canal.intermedio", "relay", ""),
            an_entry(
                "sign_unattended_greeting",
                "firma",
                "signbad",
                "greeting = true",
            ),
            an_entry(
                "sign_greeting",
                "firma",
                "signcades",
                &format!("greeting = true\n{CLICKED}"),
            ),
            an_entry("sign_unattended", "firma", "signworse", ""),
            an_entry("sign_cades", "firma", "signcades", CLICKED),
            an_entry("sign_and_save", "firma-y-guarda", "signcades", CLICKED),
        ];
        let mut catalogue = the_catalogue_in(&entries.concat()).unwrap();
        for check in &mut catalogue {
            check.family = Some(match check.requirement.set.as_str() {
                "canal.websocket" => Family::V4Echo,
                "canal.servicio" => Family::Service,
                "canal.intermedio" => Family::Intermediate,
                _ => Family::EndToEnd,
            });
        }
        catalogue
    }

    fn resolved(text: &str) -> SelectionResolution {
        resolve(text, &a_small_catalogue(), None)
    }

    fn within(scope: &str, text: &str) -> SelectionResolution {
        resolve(text, &a_small_catalogue(), Some(scope))
    }

    #[test]
    fn the_sets_of_the_catalogue_keep_its_order_with_their_check_ids() {
        let catalogue = a_small_catalogue();
        let sets = the_sets_of(&catalogue);
        let names: Vec<&str> = sets.iter().map(|set| set.name).collect();
        assert_eq!(
            names,
            [
                "canal.websocket",
                "canal.servicio",
                "canal.intermedio",
                "firma",
                "firma-y-guarda"
            ]
        );
        assert_eq!(sets[0].checks, ["ws_greeting", "ws_echo"]);
    }

    #[test]
    fn a_set_name_is_a_prefix_by_whole_segments() {
        assert_eq!(
            resolved("canal").ids,
            ["ws_greeting", "ws_echo", "service_one", "relay_one"]
        );
        assert!(!resolved("firma").ids.contains(&"sign_and_save".to_owned()));
        let half_a_segment = resolved("can");
        assert!(half_a_segment.ids.is_empty());
        assert_eq!(half_a_segment.unknown, ["can"]);
    }

    #[test]
    fn ids_and_sets_separated_by_spaces_or_commas_resolve_together() {
        assert_eq!(
            resolved("relay_one, canal.servicio  ws_greeting").ids,
            ["ws_greeting", "service_one", "relay_one"]
        );
    }

    #[test]
    fn an_unknown_id_or_set_is_named_once_in_the_order_written() {
        let resolution = resolved("nope canal.servicio typo.set nope");
        assert_eq!(resolution.ids, ["service_one"]);
        assert_eq!(resolution.unknown, ["nope", "typo.set"]);
    }

    #[test]
    fn the_greeting_of_each_family_touched_is_added_and_said() {
        let echo = resolved("ws_echo");
        assert_eq!(echo.ids, ["ws_greeting", "ws_echo"]);
        assert_eq!(echo.greetings_added, ["ws_greeting"]);

        assert_eq!(
            resolved("sign_unattended").greetings_added,
            ["sign_unattended_greeting"]
        );
        assert_eq!(
            resolved("sign_cades").greetings_added,
            ["sign_unattended_greeting", "sign_greeting"]
        );
    }

    #[test]
    fn a_greeting_named_is_not_said_to_be_added() {
        assert!(resolved("ws_greeting ws_echo").greetings_added.is_empty());
    }

    #[test]
    fn the_checks_outside_that_share_an_errand_are_named() {
        let resolution = resolved("sign_cades");
        assert_eq!(resolution.sharing_an_errand, ["sign_and_save"]);
        assert!(resolved("relay_one").sharing_an_errand.is_empty());
    }

    #[test]
    fn with_a_scope_the_names_outside_it_are_refused() {
        let resolution = within("canal.websocket firma", "ws_echo service_one canal nope");
        assert_eq!(resolution.ids, ["ws_greeting", "ws_echo"]);
        assert_eq!(resolution.out_of_scope, ["service_one"]);
        assert_eq!(resolution.unknown, ["nope"]);
    }

    #[test]
    fn with_a_scope_nothing_outside_it_shares_an_errand() {
        assert!(within("firma", "sign_cades").sharing_an_errand.is_empty());
    }
}
