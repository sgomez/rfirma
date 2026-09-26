//! El contrato de `just contract` es el de la instantánea, descubre las órdenes en cualquier `adapters/` y saca los tipos del registro.

use std::path::Path;

#[allow(dead_code)]
#[path = "../examples/contract.rs"]
mod example;

#[test]
fn the_contract_of_the_sources_is_the_snapshot() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshot = std::fs::read_to_string(manifest.join("tests/contract.snapshot"))
        .expect("deberia leerse la instantanea");
    let contract = example::contract(&manifest.join("src"));

    let frozen: Vec<&str> = snapshot.lines().collect();
    let generated: Vec<&str> = contract.lines().collect();
    let first_difference = (0..frozen.len().max(generated.len()))
        .map(|line| (line, (frozen.get(line), generated.get(line))))
        .find(|(_, (frozen, generated))| frozen != generated);
    if let Some((line, (frozen, generated))) = first_difference {
        panic!(
            "el contrato ventana-backend ha cambiado en la linea {}:\n  instantanea: {frozen:?}\n  fuentes:     {generated:?}\n\
             si es a proposito, `just contract > rfirma-app/src-tauri/tests/contract.snapshot`",
            line + 1
        );
    }
}

const AN_ADAPTER_IN_A_NEW_CONTEXT: &str = "\
#[tauri::command(async)]
pub fn synthetic_order(
    app: tauri::AppHandle,
    root: State<'_, SyntheticRoot>,
) -> Result<(), Failure> {
    Ok(())
}
";

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("tiene carpeta")).expect("carpeta");
    std::fs::write(path, source).expect("fichero");
}

#[test]
fn an_order_in_a_new_context_appears_in_the_contract_and_the_types_come_from_the_registry() {
    let tree = tempfile::tempdir().expect("deberia crearse un directorio temporal");
    write(
        tree.path(),
        "crossing/guards.rs",
        "#[tauri::command]\npub fn not_this() {}\n",
    );
    write(
        tree.path(),
        "synthetic/adapters/tauri.rs",
        AN_ADAPTER_IN_A_NEW_CONTEXT,
    );
    write(
        tree.path(),
        "synthetic/adapters/tauri/tests.rs",
        "#[tauri::command]\npub fn not_this_either() {}\n",
    );
    write(
        tree.path(),
        "synthetic/domain/thing.rs",
        "#[tauri::command]\npub fn not_an_adapter() {}\n",
    );

    let contract = example::contract(tree.path());

    assert!(
        contract.contains("async synthetic_order() -> Result<(), Failure>"),
        "{contract}"
    );
    assert!(contract.contains("pub struct Failure"), "{contract}");
    assert!(contract.contains("attemptsLeft: Option<u32>"), "{contract}");
    for excluded in ["not_this", "not_this_either", "not_an_adapter"] {
        assert!(
            !contract.contains(excluded),
            "{excluded} no es un adaptador: {contract}"
        );
    }
}
