use std::collections::BTreeSet;

use super::*;

#[test]
fn the_portal_path_never_crosses_to_the_window() {
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

#[test]
fn every_output_type_is_either_built_from_a_document_or_declared_without_one() {
    let outputs = outputs();
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
            built.contains(output.name) || without.contains(output.name),
            "«{}» ({}) no se construye desde un documento del portal ni esta declarado como \
             tipo sin documento detras: la guarda de rutas no lo mira",
            output.name,
            output.file
        );
    }

    let known: BTreeSet<&str> = outputs.iter().map(|output| output.name).collect();
    for name in built.iter().chain(without.iter()) {
        assert!(
            known.contains(name),
            "«{name}» ya no es un tipo de salida del registro: sobra de la guarda"
        );
    }
}

#[test]
fn every_type_a_command_names_is_in_the_registry() {
    let registered: BTreeSet<&str> = registry().iter().map(|crossing| crossing.name).collect();
    let signatures: Vec<(String, String)> = sources()
        .iter()
        .flat_map(|(_, source)| commands_of(source))
        .map(|(_, name, block)| {
            let signature = block
                .split_once("pub fn ")
                .and_then(|(_, after)| after.split_once('{'))
                .map(|(signature, _)| signature.to_owned())
                .unwrap_or_default();
            (name, signature)
        })
        .collect();
    assert!(signatures.len() >= 14, "el troceado de ordenes no las ve");

    for (name, signature) in &signatures {
        for ty in types_named_by(signature) {
            assert!(
                registered.contains(ty),
                "«{name}» cruza «{ty}» y no esta en el registro: declaralo con `crossing!`, \
                 que un `impl Serialize` a mano no cruza"
            );
        }
    }
}

#[test]
fn every_type_a_crossing_names_is_in_the_registry_and_every_lent_one_is_named() {
    let registry = registry();
    let registered: BTreeSet<&str> = registry.iter().map(|crossing| crossing.name).collect();
    assert_eq!(
        registered.len(),
        registry.len(),
        "un tipo esta registrado dos veces"
    );

    let mut named: BTreeSet<&str> = BTreeSet::new();
    for crossing in registry
        .iter()
        .filter(|crossing| crossing.lent_from.is_none())
    {
        for ty in crossing.referenced_types() {
            if NOT_OURS.contains(&ty) {
                continue;
            }
            assert!(
                registered.contains(ty),
                "«{}» nombra «{ty}» y el contrato no lo explica: declaralo con `crossing!` \
                 o, si es de otro modulo, prestalo con `crossing! {{ lent from … }}`",
                crossing.name
            );
            named.insert(ty);
        }
    }
    for lent in registry
        .iter()
        .filter(|crossing| crossing.lent_from.is_some())
    {
        assert!(
            named.contains(lent.name),
            "«{}» esta prestado y ningun tipo de cruce lo nombra: sobra",
            lent.name
        );
    }
}

#[test]
fn a_type_declared_in_a_test_sibling_does_not_count_as_a_crossing() {
    assert!(all_crossings()
        .iter()
        .any(|crossing| crossing.file.ends_with("/tests.rs")));
    assert!(registry()
        .iter()
        .all(|crossing| !crossing.file.ends_with("/tests.rs")));
}

#[test]
fn the_types_named_by_a_signature_leave_out_the_injected_state_and_the_standard_library() {
    assert_eq!(
        types_named_by(
            "record_recent(id: String, placement: Option<PlacementView>, root: State<'_, DocumentsRoot>, app: tauri::AppHandle) -> Result<RecentDocumentView, Failure>"
        ),
        ["Failure", "PlacementView", "RecentDocumentView"]
    );
}

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

#[test]
fn an_adapter_in_a_new_context_is_discovered_without_editing_any_list() {
    let tree = tempfile::tempdir().expect("deberia crearse un directorio temporal");
    for (relative, source) in [
        ("crossing/guards.rs", "pub struct NotThis;\n"),
        ("crossing/tests.rs", "pub struct NotThisEither;\n"),
        (
            "site/adapters/tauri.rs",
            "#[tauri::command]\npub fn two() {}\n",
        ),
        ("site/adapters/tauri/tests.rs", ""),
        ("site/adapters/views.rs", "pub struct AView;\n"),
        (
            "site/adapters/channel/server.rs",
            "pub struct NotThisEither;\n",
        ),
        ("site/domain/errand.rs", "pub struct NotAnAdapter;\n"),
        ("site/application/attend.rs", ""),
        ("memory/recents.rs", ""),
    ] {
        let path = tree.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("tiene carpeta")).expect("carpeta");
        std::fs::write(path, source).expect("fichero");
    }

    let found: Vec<String> = adapter_sources_under(tree.path())
        .into_iter()
        .map(|(relative, _)| relative)
        .collect();

    assert_eq!(found, ["site/adapters/tauri.rs", "site/adapters/views.rs"]);
}

#[test]
fn the_list_of_commands_is_closed_and_this_is_how_long_it_is() {
    let orders: usize = sources()
        .iter()
        .map(|(_, source)| production_half(source).matches("#[tauri::command").count())
        .sum();

    assert_eq!(orders, 45, "la lista de ordenes es cerrada a proposito");
}

#[test]
fn every_command_that_touches_the_portal_runs_off_the_main_thread() {
    for (file, command) in [
        ("documents/adapters/tauri.rs", "pub fn open_document("),
        ("documents/adapters/tauri.rs", "pub fn read_document("),
        (
            "documents/adapters/tauri.rs",
            "pub fn open_signed_document(",
        ),
        ("documents/adapters/tauri.rs", "pub fn open_signed_folder("),
        ("desktop/adapters/tauri.rs", "pub fn check_for_new_version("),
        (
            "desktop/adapters/tauri.rs",
            "pub fn open_external_destination(",
        ),
    ] {
        let source = production_half(source_of(file));
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        let before = &source[..declaration];
        assert!(
            before.ends_with("#[tauri::command(async)]\n"),
            "«{command}» tiene que ser #[tauri::command(async)]"
        );
    }

    let commands: Vec<_> = sources()
        .iter()
        .flat_map(|(_, source)| commands_of(production_half(source)))
        .collect();
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

#[test]
fn every_command_of_the_site_errand_runs_off_the_main_thread() {
    let source = production_half(source_of("site/adapters/tauri.rs"));

    const OF_THE_ERRAND: [&str; 6] = [
        "pub fn close_site_window(",
        "pub fn site_identify(",
        "pub fn site_decline(",
        "pub fn install_local_ca(",
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

#[test]
fn the_command_that_takes_the_pin_runs_off_the_main_thread() {
    let source = production_half(source_of("signing/adapters/tauri.rs"));
    let declaration = source
        .find("pub fn sign_with_pin(")
        .expect("no esta la orden «sign_with_pin»");

    assert!(
        source[..declaration].ends_with("#[tauri::command(async)]\n"),
        "«sign_with_pin» firma el lote remoto —dos viajes HTTPS bloqueantes y N firmas de \
         token—: tiene que ser #[tauri::command(async)] o la ventana se clava sin error"
    );
}

#[test]
fn the_pin_is_taken_by_a_single_command() {
    let takers: usize = sources()
        .iter()
        .map(|(_, source)| production_half(source).matches("pin: String").count())
        .sum();

    assert_eq!(takers, 1, "el PIN entra por una sola orden");
}
