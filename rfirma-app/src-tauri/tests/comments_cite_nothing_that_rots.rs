//! Guarda de los comentarios: ninguno cita un ID-NN, un issue, un número de línea ni una ruta que ya no existe.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    Rust,
    TypeScript,
    Java,
}

/// Zona de código cuyos comentarios se vigilan.
struct Zone {
    /// Raíz de la zona, relativa a la raíz del repositorio.
    root: &'static str,
    language: Language,
    /// Extensiones que cuentan como módulo.
    extensions: &'static [&'static str],
}

const ZONES: [Zone; 3] = [
    Zone {
        root: "rfirma-app/src-tauri/src",
        language: Language::Rust,
        extensions: &["rs"],
    },
    Zone {
        root: "rfirma-app/src",
        language: Language::TypeScript,
        extensions: &["ts", "tsx"],
    },
    Zone {
        root: "rfirma-native-bridge/src/main/java",
        language: Language::Java,
        extensions: &["java"],
    },
];

/// Prefijos de los identificadores de la especificación, que mueren con ella.
const SPEC_PREFIXES: [&str; 4] = ["ID-", "TD-", "RD-", "RT-"];

/// Extensiones que hacen de una cadena entre comillas invertidas una ruta de fichero.
const PATH_EXTENSIONS: [&str; 15] = [
    "rs", "ts", "tsx", "mjs", "cjs", "md", "json", "toml", "sh", "java", "yml", "yaml", "css",
    "html", "po",
];

/// Ficheros que el código cita y que no se versionan: los generados al construir y el manifiesto del actualizador.
const UNTRACKED_FILES: [&str; 4] = [
    "locales/index.ts",
    "locales/es.ts",
    "i18n/resources.d.ts",
    "latest.json",
];

/// Ficheros que citan «línea NN» del Java de AutoFirma 1.9.2, una versión fija: los números no se pudren; `Foo.java:NN` pasa en cualquier fichero.
const FIXED_JAVA_LINE_REFERENCES: [&str; 2] = [
    "rfirma-app/src-tauri/src/site/domain/protocol/framing.rs",
    "rfirma-app/src-tauri/src/site/adapters/service/mod.rs",
];

/// Un comentario, o una línea de un comentario de bloque, con su número de línea.
#[derive(Debug, PartialEq, Eq)]
struct CommentLine {
    line: usize,
    text: String,
}

/// Lo que está mal en una línea de comentario y qué hacer.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    file: String,
    line: usize,
    what: String,
}

fn is_a_test_file(relative: &str) -> bool {
    let rooted = format!("/{relative}");
    let a_rust_test = relative.ends_with(".rs")
        && (rooted.ends_with("/tests.rs")
            || rooted.ends_with("_tests.rs")
            || rooted.contains("/tests/"));
    a_rust_test || relative.ends_with(".test.ts") || relative.ends_with(".test.tsx")
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Los comentarios del código, una entrada por línea, sin confundir un `//` de una cadena con uno.
fn comments_of(source: &str, language: Language) -> Vec<CommentLine> {
    let chars: Vec<char> = source.chars().collect();
    let mut comments = Vec::new();
    let mut line = 1;
    let mut i = 0;
    let mut last_significant = None;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match (c, next) {
            ('/', Some('/')) => {
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                comments.push(CommentLine {
                    line,
                    text: chars[start..i].iter().collect(),
                });
                continue;
            }
            ('/', Some('*')) => {
                let mut depth = 1;
                let mut text = String::from("/*");
                i += 2;
                while i < chars.len() && depth > 0 {
                    let (here, after) = (chars[i], chars.get(i + 1).copied());
                    if here == '/' && after == Some('*') && language == Language::Rust {
                        depth += 1;
                    } else if here == '*' && after == Some('/') {
                        depth -= 1;
                    }
                    if here == '\n' {
                        comments.push(CommentLine {
                            line,
                            text: std::mem::take(&mut text),
                        });
                        line += 1;
                    } else {
                        text.push(here);
                    }
                    i += 1;
                }
                comments.push(CommentLine { line, text });
                continue;
            }
            ('"', Some('"')) if language == Language::Java && chars.get(i + 2) == Some(&'"') => {
                i = skip_text_block(&chars, i, &mut line);
                last_significant = Some('"');
                continue;
            }
            ('"', _) => {
                i = skip_quoted(&chars, i, '"', language == Language::Rust, &mut line);
                last_significant = Some('"');
                continue;
            }
            ('`', _) if language == Language::TypeScript => {
                i = skip_quoted(&chars, i, '`', true, &mut line);
                last_significant = Some('`');
                continue;
            }
            ('\'', _) => {
                i = if language == Language::Rust {
                    skip_rust_char_or_lifetime(&chars, i)
                } else {
                    skip_quoted(&chars, i, '\'', false, &mut line)
                };
                last_significant = Some('\'');
                continue;
            }
            ('r', Some('"' | '#'))
                if language == Language::Rust
                    && !(i > 0 && is_word_char(chars[i - 1]))
                    && raw_string_hashes(&chars, i).is_some() =>
            {
                let hashes = raw_string_hashes(&chars, i).unwrap_or(0);
                i = skip_raw_string(&chars, i, hashes, &mut line);
                last_significant = Some('"');
                continue;
            }
            ('/', _)
                if language == Language::TypeScript
                    && last_significant.is_none_or(|p| "(,=:[!&|?{};>".contains(p)) =>
            {
                i = skip_regex(&chars, i);
                last_significant = Some('/');
                continue;
            }
            _ => {}
        }
        if c == '\n' {
            line += 1;
        } else if !c.is_whitespace() {
            last_significant = Some(c);
        }
        i += 1;
    }
    comments
}

/// Posición tras la cadena que abre `chars[start]`; las de una línea se cortan al llegar al salto.
fn skip_quoted(
    chars: &[char],
    start: usize,
    quote: char,
    multiline: bool,
    line: &mut usize,
) -> usize {
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                if chars.get(i) == Some(&'\n') {
                    *line += 1;
                }
            }
            c if c == quote => return i + 1,
            '\n' if !multiline => return i,
            '\n' => *line += 1,
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// Posición tras el bloque de texto `"""` de Java que empieza en `chars[start]`.
fn skip_text_block(chars: &[char], start: usize, line: &mut usize) -> usize {
    let mut i = start + 3;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '"' if chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'"') => {
                return i + 3;
            }
            '\n' => *line += 1,
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// Posición tras un literal de carácter de Rust; un apóstrofo de lifetime solo avanza uno.
fn skip_rust_char_or_lifetime(chars: &[char], start: usize) -> usize {
    match (chars.get(start + 1), chars.get(start + 2)) {
        (Some('\\'), _) => {
            let mut i = start + 2;
            while i < chars.len() && chars[i] != '\'' {
                i += 1;
            }
            i + 1
        }
        (Some(_), Some('\'')) => start + 3,
        _ => start + 1,
    }
}

/// Cuántas almohadillas abren la cadena cruda que empieza en `chars[start]`, si es una.
fn raw_string_hashes(chars: &[char], start: usize) -> Option<usize> {
    let hashes = chars[start + 1..].iter().take_while(|c| **c == '#').count();
    (chars.get(start + 1 + hashes) == Some(&'"')).then_some(hashes)
}

fn skip_raw_string(chars: &[char], start: usize, hashes: usize, line: &mut usize) -> usize {
    let mut i = start + 2 + hashes;
    while i < chars.len() {
        if chars[i] == '\n' {
            *line += 1;
        }
        let closes = chars[i] == '"'
            && chars[i + 1..]
                .iter()
                .take(hashes)
                .filter(|c| **c == '#')
                .count()
                == hashes;
        if closes {
            return i + 1 + hashes;
        }
        i += 1;
    }
    chars.len()
}

/// Posición tras un literal de expresión regular de TypeScript, que no cruza el salto de línea.
fn skip_regex(chars: &[char], start: usize) -> usize {
    let mut i = start + 1;
    let mut in_class = false;
    while i < chars.len() && chars[i] != '\n' {
        match chars[i] {
            '\\' => i += 1,
            '[' => in_class = true,
            ']' => in_class = false,
            '/' if !in_class => return i + 1,
            _ => {}
        }
        i += 1;
    }
    i
}

fn starts_a_word_at(text: &str, at: usize) -> bool {
    !text[..at].ends_with(is_word_char)
}

/// La cita de especificación o de issue que hay en la línea, si hay.
fn spec_or_issue_citation(text: &str) -> Option<String> {
    for prefix in SPEC_PREFIXES {
        for (at, _) in text.match_indices(prefix) {
            let digits: String = text[at + prefix.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if starts_a_word_at(text, at) && !digits.is_empty() {
                return Some(format!("{prefix}{digits}"));
            }
        }
    }
    for (at, _) in text.match_indices('#') {
        let digits: String = text[at + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let before_ok = !text[..at].ends_with(|c: char| is_word_char(c) || c == '&');
        if before_ok && !digits.is_empty() {
            return Some(format!("#{digits}"));
        }
    }
    None
}

/// Un número de línea que cita un comentario, y de qué forma.
#[derive(Debug, PartialEq, Eq)]
enum LineReference {
    /// `fichero.ext:42`.
    OfAFile(String),
    /// «línea 57» o «líneas 10-20».
    InWords(String),
    /// `L12`.
    Abbreviated(String),
}

/// La referencia a un número de línea que hay en la línea, si hay.
fn line_number_reference(text: &str) -> Option<LineReference> {
    for (at, _) in text.match_indices(':') {
        let digits: String = text[at + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let token: String = text[..at]
            .chars()
            .rev()
            .take_while(|c| is_word_char(*c) || matches!(c, '.' | '/' | '-'))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let extension = token.rsplit_once('.').map(|(_, extension)| extension);
        if !digits.is_empty()
            && extension.is_some_and(|e| e != "java" && PATH_EXTENSIONS.contains(&e))
        {
            return Some(LineReference::OfAFile(format!("{token}:{digits}")));
        }
    }
    let lower = text.to_lowercase();
    for word in ["línea ", "líneas ", "linea ", "lineas "] {
        for (at, _) in lower.match_indices(word) {
            let after = &lower[at + word.len()..];
            if after.starts_with(|c: char| c.is_ascii_digit()) && starts_a_word_at(&lower, at) {
                let digits: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '-')
                    .collect();
                return Some(LineReference::InWords(format!("{word}{digits}")));
            }
        }
    }
    for (at, _) in text.match_indices('L') {
        let digits: String = text[at + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let after = &text[at + 1 + digits.len()..];
        if !digits.is_empty() && starts_a_word_at(text, at) && !after.starts_with(is_word_char) {
            return Some(LineReference::Abbreviated(format!("L{digits}")));
        }
    }
    None
}

/// Las rutas entre comillas invertidas de la línea que ningún fichero versionado tiene por sufijo.
fn paths_that_lead_nowhere(text: &str, tracked: &[String]) -> Vec<String> {
    let mut broken = Vec::new();
    for (index, span) in text.split('`').enumerate() {
        if index % 2 == 0 {
            continue;
        }
        let Some(path) = as_a_repository_path(span) else {
            continue;
        };
        let exists = tracked
            .iter()
            .any(|file| file == path || file.ends_with(&format!("/{path}")));
        let untracked = UNTRACKED_FILES
            .iter()
            .any(|file| *file == path || file.ends_with(&format!("/{path}")));
        if !(exists || untracked || is_autofirma_java(path)) {
            broken.push(path.to_owned());
        }
    }
    broken
}

/// Comprueba si la ruta es un `.java` de AutoFirma, cuyo código no está en este repositorio: un nombre suelto o uno con su paquete.
fn is_autofirma_java(path: &str) -> bool {
    path.ends_with(".java") && (!path.contains('/') || path.starts_with("es/gob/afirma/"))
}

/// La ruta de fichero que es el contenido de unas comillas invertidas, si lo es.
fn as_a_repository_path(span: &str) -> Option<&str> {
    let without_line = match span.rsplit_once(':') {
        Some((path, digits)) if digits.chars().all(|c| c.is_ascii_digit() || c == '-') => path,
        _ => span,
    };
    let mut path = without_line;
    while let Some(rest) = path.strip_prefix("./").or_else(|| path.strip_prefix("../")) {
        path = rest;
    }
    let plain = path
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'));
    let (stem, extension) = path.rsplit_once('.')?;
    let known = PATH_EXTENSIONS.contains(&extension);
    (plain && known && !stem.is_empty() && !path.starts_with(['/', '~'])).then_some(path)
}

/// Lo que está mal en los comentarios de un fichero.
fn findings_in(
    file: &str,
    source: &str,
    language: Language,
    tracked: &[String],
    fixed_line_references: &[&str],
) -> Vec<Finding> {
    let lines_are_fixed = fixed_line_references.contains(&file);
    let mut findings = Vec::new();
    for comment in comments_of(source, language) {
        let mut report = |what: String| {
            findings.push(Finding {
                file: file.to_owned(),
                line: comment.line,
                what,
            });
        };
        if let Some(cited) = spec_or_issue_citation(&comment.text) {
            report(format!(
                "cita `{cited}`, que muere antes que el codigo: di lo que hace o por que, o borra la frase"
            ));
        }
        match line_number_reference(&comment.text) {
            Some(LineReference::InWords(_)) if lines_are_fixed => {}
            Some(
                LineReference::OfAFile(cited)
                | LineReference::InWords(cited)
                | LineReference::Abbreviated(cited),
            ) => {
                report(format!(
                    "cita un numero de linea (`{cited}`), que se corre con cada edicion: nombra el simbolo"
                ));
            }
            None => {}
        }
        for path in paths_that_lead_nowhere(&comment.text, tracked) {
            report(format!(
                "cita la ruta `{path}`, que no es sufijo de ningun fichero versionado: corrigela o borrala"
            ));
        }
    }
    findings
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn tracked_files(root: &Path, pathspec: &str) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", pathspec])
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
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn no_comment_in_the_code_cites_what_rots() {
    let root = repository_root();
    let tracked = tracked_files(&root, ".");
    let mut findings = Vec::new();
    let mut scanned = 0;
    for zone in &ZONES {
        for file in tracked_files(&root, zone.root) {
            let is_module = zone
                .extensions
                .iter()
                .any(|extension| file.ends_with(&format!(".{extension}")));
            let relative = file
                .strip_prefix(&format!("{}/", zone.root))
                .unwrap_or(&file);
            if !is_module || is_a_test_file(relative) {
                continue;
            }
            let source = std::fs::read_to_string(root.join(&file))
                .unwrap_or_else(|error| panic!("deberia leerse {file}: {error}"));
            scanned += 1;
            findings.extend(findings_in(
                &file,
                &source,
                zone.language,
                &tracked,
                &FIXED_JAVA_LINE_REFERENCES,
            ));
        }
    }
    assert!(scanned > 200, "la guarda solo ha leido {scanned} modulos");
    let report: Vec<String> = findings
        .iter()
        .map(|f| format!("{}:{}: {}", f.file, f.line, f.what))
        .collect();
    assert!(
        report.is_empty(),
        "un comentario cita algo que se pudre (ID-NN, TD-NN, #NNN, numeros de linea, rutas que no \
         existen); lo que importa lo dicen el nombre, el codigo o un ADR por numero:\n  {}",
        report.join("\n  ")
    );
}

fn tree() -> Vec<String> {
    ["rfirma-app/src/signing/flow.ts", "docs/adr/0001-x.md"]
        .map(str::to_owned)
        .into()
}

fn rust_findings(source: &str) -> Vec<Finding> {
    findings_in("a.rs", source, Language::Rust, &tree(), &[])
}

fn java_findings(source: &str) -> Vec<Finding> {
    findings_in("A.java", source, Language::Java, &tree(), &[])
}

fn ts_findings(source: &str) -> Vec<Finding> {
    findings_in("a.ts", source, Language::TypeScript, &tree(), &[])
}

#[test]
fn a_spec_or_issue_citation_in_a_comment_is_caught_with_its_line() {
    let found = ts_findings("const a = 1;\n// Lo manda la sede (ID-63, #188).\n");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, 2);
    assert!(found[0].what.contains("ID-63"));
    assert!(!ts_findings("/** Segun el TD-13. */").is_empty());
    assert!(!rust_findings("//! Del issue #123.").is_empty());
    assert!(!ts_findings("{/* Ver #7 */}").is_empty());
}

#[test]
fn pkcs_adr_and_entities_are_not_citations() {
    assert!(ts_findings("// El modulo PKCS#11 y el PKCS#12 (ADR-0017).").is_empty());
    assert!(ts_findings("// El `&#123;` no es un issue ni un color #fff.").is_empty());
    assert!(ts_findings("// Un ID-abc o un RID-5.").is_empty());
}

#[test]
fn a_citation_in_code_or_in_a_string_is_not_a_comment() {
    assert!(ts_findings("const a = \"ID-63 #188\";\n").is_empty());
    assert!(ts_findings("const a = 'x // ID-63';\n").is_empty());
    assert!(ts_findings("const a = `x // ID-63`;\n").is_empty());
    assert!(rust_findings("let a = \"https://x.y/#12 // ID-3\";\n").is_empty());
    assert!(rust_findings("let a = r#\"// ID-3 \"quoted\" \"#;\n").is_empty());
    assert!(rust_findings("let c = '\"'; let b = &'a str; // ok\n").is_empty());
    assert!(ts_findings("const re = /[\"'] \\/ #12/;\nconst b = 1; // ok\n").is_empty());
}

#[test]
fn a_string_continued_with_a_backslash_keeps_the_line_count() {
    let found = rust_findings("let a = \"uno \\\n    dos\";\n// Ver #7.\n");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, 3);
}

#[test]
fn a_block_comment_is_read_line_by_line() {
    let found = ts_findings("/**\n * Primera.\n * Segunda (#188).\n */\nconst a = 1;\n");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, 3);
    let nested = rust_findings("/* a /* b */ #9 */\nfn f() {}\n");
    assert_eq!(nested.len(), 1);
}

#[test]
fn a_line_number_reference_is_caught() {
    assert!(!ts_findings("// Mira flow.ts:42 para ver.").is_empty());
    assert!(!ts_findings("// Mira la linea 57.").is_empty());
    assert!(!ts_findings("// Las lineas 10-20 hacen eso.").is_empty());
    assert!(!ts_findings("// Mira L12-20.").is_empty());
    assert!(ts_findings("// Escucha en localhost:3000 a las 10:30 con L2c.").is_empty());
}

#[test]
fn a_line_of_an_autofirma_java_file_passes_in_any_file() {
    assert!(ts_findings("// Ver `Foo.java:295,336` y `Bar.java:10-20`.").is_empty());
    assert!(rust_findings("/// Como `Foo.java:42` (1.9.2).").is_empty());
    assert!(!ts_findings("// Ver `foo.ts:42`.").is_empty());
}

#[test]
fn the_fixed_java_line_references_pass_only_in_the_listed_files() {
    let source = "/// Tamaño (`RESPONSE_MAX_SIZE`, línea 57).\n";
    assert!(findings_in("a.rs", source, Language::Rust, &tree(), &["a.rs"]).is_empty());
    assert!(!findings_in("b.rs", source, Language::Rust, &tree(), &["a.rs"]).is_empty());
    let path_line = "/// En `flow.ts:42`.\n";
    assert!(
        !findings_in("a.rs", path_line, Language::Rust, &tree(), &["a.rs"]).is_empty(),
        "la excepcion es de la linea del Java, no de una ruta con linea"
    );
}

#[test]
fn a_backticked_path_must_end_a_tracked_file() {
    assert!(ts_findings("// Mira `signing/flow.ts` y `flow.ts`.").is_empty());
    assert!(ts_findings("// Mira `0001-x.md`.").is_empty());
    let found = ts_findings("// Mira `signing/gone.ts`.");
    assert_eq!(found.len(), 1);
    assert!(found[0].what.contains("signing/gone.ts"));
    assert!(!ts_findings("// Mira `nosigning/flow.ts`.").is_empty());
    assert!(!ts_findings("// Mira `flow.ts:42`.").is_empty());
}

#[test]
fn things_in_backticks_that_are_not_repository_paths_are_left_alone() {
    assert!(
        ts_findings("// `.ts`, `*.rs`, `<dir>/x.md`, `a b.rs`, `x.value`, `npm i`.").is_empty()
    );
    assert!(ts_findings("// `/etc/x.json`, `~/x.json`, `foo()`.").is_empty());
}

#[test]
fn generated_files_and_autofirma_java_are_exempt() {
    assert!(
        ts_findings("// `locales/index.ts` y `locales/es.ts` salen de `just po-import`.")
            .is_empty()
    );
    assert!(!ts_findings("// `locales/other.ts`.").is_empty());
    assert!(
        ts_findings("// `es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java`.")
            .is_empty()
    );
    assert!(ts_findings("// `Launcher.java`.").is_empty());
    assert!(!ts_findings("// `src/Launcher.java`.").is_empty());
    assert!(ts_findings("// `pdf.js`, `Cargo.lock`, `latest.json`.").is_empty());
}

#[test]
fn a_citation_in_a_java_comment_is_caught_but_not_in_a_string_or_a_text_block() {
    assert_eq!(
        java_findings("class A {\n    // ver ID-63\n}\n").len(),
        1,
        "un comentario de linea Java con una cita"
    );
    assert_eq!(
        java_findings("/**\n * Falla (#12).\n */\nclass A { }\n").len(),
        1,
        "un Javadoc con una cita"
    );
    assert!(java_findings("class A { String s = \"#12 ID-63\"; }\n").is_empty());
    assert!(
        java_findings("class A { String s = \"\"\"\n  #12 // ID-63\n  \"\"\"; }\n").is_empty(),
        "un bloque de texto no es un comentario"
    );
    assert!(
        java_findings("class A { int c = a / b; // ID-63\n}\n").len() == 1,
        "una division no abre una expresion regular que se coma el comentario"
    );
    assert!(java_findings("class A { char q = '\\''; char h = '#'; }\n").is_empty());
}
