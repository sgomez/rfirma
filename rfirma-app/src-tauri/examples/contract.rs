//! El contrato ventana-backend: las órdenes, leídas del fuente, y los tipos que cruzan, leídos del registro.

use std::path::{Path, PathBuf};

fn main() {
    let src = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    print!("{}", contract(&src));
}

fn contract(src: &Path) -> String {
    let text = format!(
        "ORDENES DE TAURI                          (<contexto>/adapters/tauri*.rs)\n\
         \x20 Sin el estado inyectado (State<...>, AppHandle): no cruza.\n\
         \n{}\n\
         \nTIPOS QUE CRUZAN                          (<contexto>/adapters/views*.rs y orders.rs)\n\
         \x20 Campos con el nombre que ve la ventana.\n\
         {}\n\
         \n-- generado de las fuentes en cada ejecucion: no puede quedarse obsoleto --\n",
        orders_section(src),
        rfirma_lib::crossing::types_section(),
    );
    squeezed(&text)
}

fn squeezed(text: &str) -> String {
    let mut out = String::new();
    let mut blank = false;
    for line in text.lines() {
        if line.is_empty() {
            if blank {
                continue;
            }
            blank = true;
        } else {
            blank = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn orders_section(src: &Path) -> String {
    let mut files = rust_files_under(src, "");
    files.retain(|relative| is_an_adapter(relative));
    files.sort();
    files
        .iter()
        .flat_map(|relative| {
            let source = std::fs::read_to_string(src.join(relative))
                .unwrap_or_else(|error| panic!("deberia leerse {relative}: {error}"));
            orders_in(&source)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn is_an_adapter(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or_default();
    if name == "tests.rs" || name == "guards.rs" {
        return false;
    }
    let mut segments = relative.split('/');
    if segments.next() == Some("commands") {
        return true;
    }
    segments.next() == Some("adapters")
        && ["tauri", "views", "orders"]
            .iter()
            .any(|stem| name.starts_with(stem))
}

fn rust_files_under(directory: &Path, prefix: &str) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if entry.path().is_dir() {
            found.extend(rust_files_under(&entry.path(), &relative));
        } else if name.ends_with(".rs") {
            found.push(relative);
        }
    }
    found
}

fn orders_in(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut taking: Option<(bool, String)> = None;
    for line in source.lines() {
        if line.contains("#[tauri::command") {
            taking = Some((line.contains("async"), String::new()));
            continue;
        }
        let Some((is_async, buffer)) = taking.as_mut() else {
            continue;
        };
        buffer.push(' ');
        buffer.push_str(line);
        if !line.trim_end().ends_with('{') {
            continue;
        }
        let signature = buffer.split_whitespace().collect::<Vec<&str>>().join(" ");
        let signature = signature.trim_end_matches('{').trim_end();
        let signature = signature.strip_prefix("pub fn ").unwrap_or(signature);
        found.push(format!(
            "  {:<6}{}",
            if *is_async { "async " } else { "" },
            without_injected_state(signature)
        ));
        taking = None;
    }
    found
}

fn without_injected_state(signature: &str) -> String {
    let Some(open) = signature.find('(') else {
        return signature.to_owned();
    };
    let Some(close) = closing_paren(signature, open) else {
        return signature.to_owned();
    };
    let kept: Vec<&str> = split_at_top_level(&signature[open + 1..close])
        .into_iter()
        .filter(|parameter| {
            let ty = parameter
                .split_once(':')
                .map(|(_, ty)| ty.trim())
                .unwrap_or_default();
            !(ty.starts_with("State<") || ty == "tauri::AppHandle")
        })
        .collect();
    format!(
        "{}({}){}",
        &signature[..open],
        kept.join(", "),
        &signature[close + 1..]
    )
}

fn closing_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (at, letter) in text.char_indices().skip(open) {
        match letter {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_at_top_level(parameters: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (at, letter) in parameters.char_indices() {
        match letter {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                pieces.push(parameters[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    let last = parameters[start..].trim();
    if !last.is_empty() {
        pieces.push(last);
    }
    pieces
}
