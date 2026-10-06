//! Ningún CSS fuera de los tokens del sistema declara `font-size`, `font-weight` ni `line-height` con un literal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SRC: &str = "rfirma-app/src";
const SYSTEM_FOLDERS: [&str; 2] = [
    "rfirma-app/src/design-system/bundle/tokens/",
    "rfirma-app/src/design-system/bundle/fonts/",
];
const CSS_WIDE_KEYWORDS: [&str; 5] = ["inherit", "initial", "unset", "revert", "revert-layer"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn tracked_stylesheets(root: &Path, under: &str) -> Vec<String> {
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
        .filter(|path| path.ends_with(".css"))
        .map(str::to_owned)
        .collect()
}

fn composition_of(property: &str) -> Option<&'static str> {
    match property {
        "font-size" => Some("`var(--rf-<rol>-size)`"),
        "font-weight" => Some("`var(--rf-<rol>-weight)`"),
        "line-height" => Some("`var(--rf-<rol>-leading)`"),
        _ => None,
    }
}

fn without_comments(source: &str) -> String {
    let mut kept = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("/*") {
        kept.push_str(&rest[..start]);
        rest = match rest[start..].find("*/") {
            Some(end) => &rest[start + end + 2..],
            None => "",
        };
    }
    kept.push_str(rest);
    kept
}

fn is_literal(value: &str) -> bool {
    let value = value.trim().trim_end_matches("!important").trim();
    !(value.starts_with("var(") || CSS_WIDE_KEYWORDS.contains(&value))
}

fn offences_in(stylesheet: &str, source: &str) -> Vec<String> {
    without_comments(source)
        .replace(['{', '}'], ";")
        .split(';')
        .filter_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            let property = property.trim();
            let composition = composition_of(property)?;
            if !is_literal(value) {
                return None;
            }
            Some(format!(
                "`{stylesheet}` declara `{property}: {}`: la tipografía se compone con un rol del \
                 sistema, {composition}; un tamaño de dibujo va en una variable local del componente",
                value.trim()
            ))
        })
        .collect()
}

#[test]
fn no_stylesheet_outside_the_tokens_declares_typography_literals() {
    let root = repository_root();
    let stylesheets = tracked_stylesheets(&root, SRC);
    assert!(
        stylesheets.len() > 10,
        "el listado no ha encontrado los CSS: {} ficheros",
        stylesheets.len()
    );

    let offences: Vec<String> = stylesheets
        .iter()
        .filter(|stylesheet| {
            !SYSTEM_FOLDERS
                .iter()
                .any(|folder| stylesheet.starts_with(folder))
        })
        .flat_map(|stylesheet| {
            let source = fs::read_to_string(root.join(stylesheet))
                .unwrap_or_else(|error| panic!("deberia leerse {stylesheet}: {error}"));
            offences_in(stylesheet, &source)
        })
        .collect();

    assert!(
        offences.is_empty(),
        "{} literal(es) tipográficos fuera de los tokens:\n{}",
        offences.len(),
        offences.join("\n")
    );
}

#[test]
fn a_typography_literal_turns_red() {
    let stylesheet = "rfirma-app/src/signing/Panel.css";
    for source in [
        ".a { font-size: 12px; }",
        ".a { font-weight: 600 }",
        ".a { line-height: 1.4; }",
        ".a{font-size:8px}",
        ".a {\n  font-weight:\n    700 !important;\n}",
    ] {
        assert_eq!(offences_in(stylesheet, source).len(), 1, "{source}");
    }
}

#[test]
fn a_role_variable_a_keyword_or_a_comment_is_left_alone() {
    let stylesheet = "rfirma-app/src/signing/Panel.css";
    for source in [
        ".a { font-size: var(--rf-body-size); }",
        ".a { font-weight: inherit; }",
        ".a { font: inherit; }",
        ".a { --thumbnail-size: 8px; font-size: var(--thumbnail-size); }",
        "/* font-size: 12px; */ .a { color: red; }",
        ".a { font-family: var(--rf-font-body); }",
    ] {
        assert!(offences_in(stylesheet, source).is_empty(), "{source}");
    }
}
