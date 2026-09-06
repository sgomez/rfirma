//! Reglas de nomenclatura para el documento firmado y resolución de homónimos (ADR-0011).

/// Sufijo añadido al nombre del documento original.
pub const SIGNED_SUFFIX: &str = "-firmado";

/// Primer número para desempate de homónimos.
pub const FIRST_NUMBER: u32 = 2;

/// Límite máximo de comprobaciones de homónimos antes de desistir.
pub const MAX_NAMESAKES: u32 = 999;

const FALLBACK_STEM: &str = "documento";

/// Genera el nombre canónico del documento firmado a partir del nombre original.
pub fn signed_name(original: &str) -> String {
    let (stem, extension) = split_extension(original);
    let stem = if stem.is_empty() { FALLBACK_STEM } else { stem };
    let unnumbered = without_the_number(stem);
    if ends_with_the_suffix(unnumbered) {
        return format!("{unnumbered}{extension}");
    }
    format!("{stem}{SIGNED_SUFFIX}{extension}")
}

/// Añade el número de desempate al nombre: `contrato-firmado-2.pdf`.
pub fn numbered(name: &str, number: u32) -> String {
    let (stem, extension) = split_extension(name);
    format!("{stem}-{number}{extension}")
}

fn ends_with_the_suffix(stem: &str) -> bool {
    let (stem, suffix) = (stem.as_bytes(), SIGNED_SUFFIX.as_bytes());
    stem.len() >= suffix.len() && stem[stem.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

fn without_the_number(stem: &str) -> &str {
    match stem.rsplit_once('-') {
        Some((head, number))
            if !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            head
        }
        _ => stem,
    }
}

fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 => name.split_at(dot),
        _ => (name, ""),
    }
}

#[cfg(test)]
mod tests;
