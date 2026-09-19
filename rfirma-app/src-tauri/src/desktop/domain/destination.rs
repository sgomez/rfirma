//! Destino externo reconocido por la aplicación (ADR-0011).

pub const DISCUSSIONS: &str = "discussions";
pub const DISCUSSIONS_URL: &str = "https://github.com/sgomez/rfirma/discussions";
pub const RELEASES: &str = "releases";
pub const RELEASES_URL: &str = "https://github.com/sgomez/rfirma/releases";
pub const REPOSITORY: &str = "repository";
pub const REPOSITORY_URL: &str = "https://rfirma.sgomez.me/";
pub const CERTIFICATE_ISSUANCE: &str = "certificateIssuance";
pub const CERTIFICATE_ISSUANCE_URL: &str =
    "https://www.sede.fnmt.gob.es/certificados/persona-fisica/obtener-certificado-software";

pub fn resolve_destination(target: &str) -> Option<&'static str> {
    match target {
        DISCUSSIONS => Some(DISCUSSIONS_URL),
        RELEASES => Some(RELEASES_URL),
        REPOSITORY => Some(REPOSITORY_URL),
        CERTIFICATE_ISSUANCE => Some(CERTIFICATE_ISSUANCE_URL),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
