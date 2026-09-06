//! `just contract` descubre órdenes y tipos de cruce en cualquier `adapters/` de un contexto (RD-02).

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
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = \"camelCase\")]
pub struct SyntheticView {
    pub holder_name: String,
}

#[tauri::command(async)]
pub fn synthetic_order(app: tauri::AppHandle) -> SyntheticView {
    SyntheticView { holder_name: String::new() }
}
";

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("tiene carpeta")).expect("carpeta");
    std::fs::write(path, source).expect("fichero");
}

#[test]
fn an_adapter_in_a_new_context_appears_in_the_contract_without_editing_any_list() {
    let tree = tempfile::tempdir().expect("deberia crearse un directorio temporal");
    write(tree.path(), "commands/mod.rs", A_LEGACY_ORDER);
    write(
        tree.path(),
        "commands/guards.rs",
        "#[derive(Serialize)]\npub struct NotThis;\n",
    );
    write(
        tree.path(),
        "synthetic/adapters/tauri.rs",
        AN_ADAPTER_IN_A_NEW_CONTEXT,
    );
    write(
        tree.path(),
        "synthetic/adapters/tauri/tests.rs",
        "#[derive(Serialize)]\npub struct NotThisEither;\n",
    );
    write(
        tree.path(),
        "synthetic/domain/thing.rs",
        "#[derive(Serialize)]\npub struct NotAnAdapter;\n",
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
        contract.contains("async synthetic_order() -> SyntheticView"),
        "{contract}"
    );
    assert!(contract.contains("pub struct SyntheticView"), "{contract}");
    assert!(contract.contains("holderName: String"), "{contract}");
    for excluded in ["NotThis", "NotThisEither", "NotAnAdapter"] {
        assert!(
            !contract.contains(excluded),
            "{excluded} no es un adaptador: {contract}"
        );
    }
}
