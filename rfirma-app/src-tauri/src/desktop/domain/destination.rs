//! Destino externo reconocido por la aplicación (ADR-0011).

pub const DISCUSSIONS: &str = "discussions";
pub const DISCUSSIONS_URL: &str = "https://github.com/sgomez/rfirma/discussions";

pub fn resolve_destination(target: &str) -> Option<&'static str> {
    match target {
        DISCUSSIONS => Some(DISCUSSIONS_URL),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
