// ADR-0013: aquí no se llama ni a Maven ni a `native-image`. Un `cargo build`
// que dispare por sorpresa 1 m 22 s de `native-image` arruina el bucle de
// realimentación que el issue #11 decidió proteger. `just native` construye la
// librería; `just dev` solo comprueba que está.
#[path = "src/signing/domain/catalog/po_file.rs"]
mod po_file;

const PO_LANGUAGES: [&str; 5] = ["es", "ca", "eu", "gl", "en"];

fn main() {
    write_the_catalog();
    record_the_commit();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        tauri_build::build();
        return;
    }
    // ADR-0035: el manifiesto va también a las pruebas, que sin él no arrancan.
    let manifest = crate_dir().join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build debería preparar la aplicación");
}

// Con `env!` valdría la ruta del worktree que compiló este script en el target compartido.
fn crate_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo define CARGO_MANIFEST_DIR"),
    )
}

// ADR-0009: solo entran los idiomas al 100 %, la misma regla que `po-import`.
fn write_the_catalog() {
    let po = crate_dir().join("../po");
    println!("cargo:rerun-if-changed={}", po.display());
    let mut published = String::new();
    for tag in PO_LANGUAGES {
        let Ok(text) = std::fs::read_to_string(po.join(format!("{tag}.po"))) else {
            assert_ne!(tag, "es", "falta po/es.po, y es el original");
            continue;
        };
        let entries = po_file::entries_of(&text);
        if !po_file::is_complete(&entries) {
            assert_ne!(tag, "es", "po/es.po no está completo, y es el original");
            continue;
        }
        published.push_str(&format!("    ({tag:?}, &[\n"));
        for (key, text) in &entries {
            published.push_str(&format!("        ({key:?}, {text:?}),\n"));
        }
        published.push_str("    ]),\n");
    }
    let generated = format!(
        "/// Los catálogos de los idiomas al 100 %, con sus pares clave → texto.\n\
         const PUBLISHED: &[(&str, &[(&str, &str)])] = &[\n{published}];\n"
    );
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("cargo define OUT_DIR"));
    std::fs::write(out.join("catalog.rs"), generated).expect("se escribe el catálogo generado");
}

fn record_the_commit() {
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(crate_dir())
            .output()
            .ok()
            .filter(|output| output.status.success())?;
        Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    if let Some(head) = git(&["rev-parse", "--path-format=absolute", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(commit) = git(&["rev-parse", "--short", "HEAD"]).filter(|c| !c.is_empty()) {
        println!("cargo:rustc-env=RFIRMA_GIT_COMMIT={commit}");
    }
}
