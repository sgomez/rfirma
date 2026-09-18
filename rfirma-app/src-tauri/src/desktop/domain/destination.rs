//! Destino externo reconocido por la aplicación (ADR-0011).

pub const DISCUSSIONS: &str = "discussions";
pub const DISCUSSIONS_URL: &str = "https://github.com/sgomez/rfirma/discussions";
pub const RELEASES: &str = "releases";
pub const RELEASES_URL: &str = "https://github.com/sgomez/rfirma/releases";
pub const REPOSITORY: &str = "repository";
pub const REPOSITORY_URL: &str = "https://rfirma.sgomez.me/";

pub fn resolve_destination(target: &str) -> Option<&'static str> {
    match target {
        DISCUSSIONS => Some(DISCUSSIONS_URL),
        RELEASES => Some(RELEASES_URL),
        REPOSITORY => Some(REPOSITORY_URL),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
