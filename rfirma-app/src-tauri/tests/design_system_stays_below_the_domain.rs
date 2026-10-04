//! El sistema de diseño de la ventana solo importa de sí mismo y del catálogo de cadenas, nunca de una carpeta de dominio.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DESIGN_SYSTEM: &str = "rfirma-app/src/design-system";
const ALLOWED_FOLDERS: [&str; 2] = ["design-system", "i18n"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn tracked_design_system_modules(root: &Path) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", DESIGN_SYSTEM])
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git deberia estar: `just tools` lo exige");
    assert!(listing.status.success(), "git ls-files deberia funcionar");

    String::from_utf8(listing.stdout)
        .expect("las rutas deberian ser UTF-8")
        .split('\0')
        .filter(|path| path.ends_with(".ts") || path.ends_with(".tsx"))
        .map(str::to_owned)
        .collect()
}

fn quoted_after(text: &str, marker: &str) -> Option<String> {
    let rest = &text[text.find(marker)? + marker.len()..];
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let inner = &rest[1..];
    Some(inner[..inner.find(quote)?].to_owned())
}

fn specifier_of(line: &str) -> Option<String> {
    let code = line.trim();
    if code.starts_with("//") || code.starts_with('*') {
        return None;
    }
    if code.starts_with("import ") || code.starts_with("export ") || code.starts_with('}') {
        if let Some(specifier) = quoted_after(code, " from ") {
            return Some(specifier);
        }
        if let Some(rest) = code.strip_prefix("import ") {
            if rest.starts_with(['"', '\'']) {
                return quoted_after(code, "import ");
            }
        }
    }
    quoted_after(code, "import(")
}

fn src_folder_targeted(module: &str, specifier: &str) -> Option<String> {
    if !specifier.starts_with('.') {
        return None;
    }
    let mut segments: Vec<&str> = module.split('/').collect();
    segments.pop();
    for part in specifier.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    let relative = segments.strip_prefix(&["rfirma-app", "src"])?;
    Some(
        relative
            .first()
            .map(|folder| (*folder).to_owned())
            .unwrap_or_default(),
    )
}

fn offences_in(module: &str, source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let specifier = specifier_of(line)?;
            let folder = src_folder_targeted(module, &specifier)?;
            if ALLOWED_FOLDERS.contains(&folder.as_str()) {
                return None;
            }
            Some(format!(
                "`{module}` importa `{specifier}`, de `src/{folder}`: el sistema de diseño no \
                 conoce las carpetas de dominio; lo que necesitaba entra por props"
            ))
        })
        .collect()
}

#[test]
fn the_design_system_imports_only_from_itself_and_the_string_catalogue() {
    let root = repository_root();
    let modules = tracked_design_system_modules(&root);
    assert!(
        modules.len() > 10,
        "el listado no ha encontrado el sistema de diseño: {} ficheros",
        modules.len()
    );

    let offences: Vec<String> = modules
        .iter()
        .flat_map(|module| {
            let source = fs::read_to_string(root.join(module))
                .unwrap_or_else(|error| panic!("deberia leerse {module}: {error}"));
            offences_in(module, &source)
        })
        .collect();

    assert!(
        offences.is_empty(),
        "{} import(s) del sistema de diseño apuntan a una carpeta de dominio:\n{}",
        offences.len(),
        offences.join("\n")
    );
}

#[test]
fn an_import_of_a_domain_folder_turns_red() {
    let module = "rfirma-app/src/design-system/Button.tsx";
    for source in [
        "import { useSigning } from \"../signing/useSigning\";",
        "import type { Thing } from '../documents/thing';",
        "} from \"../identity/certificate\";",
        "const view = import(\"../site/View\");",
        "import \"../preferences/side-effect\";",
        "import { x } from \"../main\";",
    ] {
        assert_eq!(offences_in(module, source).len(), 1, "{source}");
    }
}

#[test]
fn what_stays_inside_or_goes_to_the_catalogue_is_left_alone() {
    let module = "rfirma-app/src/design-system/Dialog.test.tsx";
    for source in [
        "import { Button } from \"./Button\";",
        "import i18n from \"../i18n/i18n\";",
        "import { decorator } from \"../../.storybook/decorators/dialogWindow\";",
        "import { render } from \"@testing-library/react\";",
        "// import { x } from \"../signing/x\";",
        "const label = \"../signing/x\";",
    ] {
        assert!(offences_in(module, source).is_empty(), "{source}");
    }
}
