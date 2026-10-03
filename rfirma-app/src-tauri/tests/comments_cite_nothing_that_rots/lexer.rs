//! El lector de comentarios de la guarda: separa los de Rust, TypeScript y Java del código y de las cadenas.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Java,
}

/// Un comentario, o una línea de un comentario de bloque, con su número de línea.
#[derive(Debug, PartialEq, Eq)]
pub struct CommentLine {
    pub line: usize,
    pub text: String,
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Los comentarios del código, una entrada por línea, sin confundir un `//` de una cadena con uno.
pub fn comments_of(source: &str, language: Language) -> Vec<CommentLine> {
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
