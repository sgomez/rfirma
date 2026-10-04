//! Lectura de los imports relativos de la interfaz, compartida por las guardas de dirección de `src/`.

use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Los `.ts` y `.tsx` versionados bajo una ruta, relativos a la raíz del repositorio.
pub(crate) fn tracked_modules(root: &Path, under: &str) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", under])
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

pub(crate) fn specifier_of(line: &str) -> Option<String> {
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

/// A dónde apunta un import relativo, como segmentos bajo `rfirma-app/src`; vacío si sale de ahí.
pub(crate) fn src_path_targeted(module: &str, specifier: &str) -> Option<Vec<String>> {
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
    Some(relative.iter().map(|part| (*part).to_owned()).collect())
}

/// La carpeta de `src/` a la que apunta un import relativo; vacía si es un módulo de la raíz.
pub(crate) fn src_folder_targeted(module: &str, specifier: &str) -> Option<String> {
    let path = src_path_targeted(module, specifier)?;
    Some(path.first().cloned().unwrap_or_default())
}
