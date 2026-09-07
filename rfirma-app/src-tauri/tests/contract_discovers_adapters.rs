//! `just contract` descubre las órdenes en cualquier `adapters/` de un contexto (RD-02) y saca los tipos del registro, no del fuente (#441).

use std::path::{Path, PathBuf};
use std::process::Command;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

const A_LEGACY_ORDER: &str =
    "#[tauri::command]\npub fn legacy_order(name: String) -> String {\n    name\n}\n";

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
    write(tree.path(), "commands/mod.rs", A_LEGACY_ORDER);
    write(
        tree.path(),
        "commands/guards.rs",
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

    let output = Command::new("just")
        .args(["--justfile", "justfile", "contract"])
        .arg(tree.path())
        .current_dir(repository_root())
        .output()
        .expect("just deberia estar: `just tools` lo exige");
    let contract = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        contract.contains("legacy_order(name: String)"),
        "{contract}"
    );
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
