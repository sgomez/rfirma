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
    record_the_rdnie_version();
    record_the_rades_version();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    link_libxslt(&target_os);
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

// Linux carga la libxslt del sistema con `dlopen` y no enlaza nada.
fn link_libxslt(target_os: &str) {
    match target_os {
        "macos" => {
            println!("cargo:rustc-link-lib=dylib=xslt");
            println!("cargo:rustc-link-lib=dylib=xml2");
        }
        "windows" => link_vcpkg_libxslt(),
        _ => {}
    }
}

fn link_vcpkg_libxslt() {
    println!("cargo:rerun-if-env-changed=VCPKG_INSTALLED_DIR");
    let installed = std::env::var_os("VCPKG_INSTALLED_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate_dir().join("vcpkg_installed"));
    let lib = installed.join("x64-windows-static").join("lib");
    for name in ["libxslt", "libxml2"] {
        let file = lib.join(format!("{name}.lib"));
        assert!(
            file.is_file(),
            "falta {}: compila libxml2 y libxslt con `just vcpkg`",
            file.display()
        );
        println!("cargo:rerun-if-changed={}", file.display());
        println!("cargo:rustc-link-lib=static={name}");
    }
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib=bcrypt");
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

fn record_the_rdnie_version() {
    let version = locked_package("rdnie")
        .and_then(|package| field(&package, "version"))
        .unwrap_or_else(|| "desconocida".to_owned());
    println!("cargo:rustc-env=RFIRMA_RDNIE_VERSION={version}");
}

fn record_the_rades_version() {
    if std::env::var_os("CARGO_FEATURE_RADES").is_none() {
        return;
    }
    let Some(package) = locked_package("rades") else {
        return;
    };
    let version = field(&package, "version").unwrap_or_else(|| "desconocida".to_owned());
    let commit = field(&package, "source").and_then(|source| {
        source
            .rsplit_once('#')
            .map(|(_, sha)| sha.chars().take(7).collect::<String>())
    });
    let line = match commit {
        Some(commit) => format!("{version} · {commit}"),
        None => version,
    };
    println!("cargo:rustc-env=RFIRMA_RADES_VERSION={line}");
}

fn locked_package(name: &str) -> Option<String> {
    let lock = crate_dir().join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = std::fs::read_to_string(lock).ok()?;
    text.split("[[package]]")
        .find(|package| package.contains(&format!("\nname = \"{name}\"\n")))
        .map(str::to_owned)
}

fn field(package: &str, key: &str) -> Option<String> {
    package
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key} = ")))
        .map(|quoted| quoted.trim_matches('"').to_owned())
}
