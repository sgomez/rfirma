//! Cómo se llama el documento firmado, y qué pasa cuando ese nombre ya está
//! ocupado (ADR-0011).
//!
//! Aquí no hay disco: son cadenas. Quien mira si el nombre está libre es
//! [`super::CheckedFolder::landing_for`], y por eso el conteo empieza en
//! **2**: el primero no lleva número.
//!
//! El detalle que parece un capricho y no lo es: **no se apila un segundo
//! sufijo**. Cofirmar `contrato-firmado.pdf` da `contrato-firmado-2.pdf`, no
//! `contrato-firmado-firmado.pdf`. Un documento que se cofirma tres veces es
//! el caso normal de rFirma, no el raro, y el nombre no puede crecer con cada
//! firma.

/// Lo que se le añade al nombre del original.
///
/// Está en castellano y **no se traduce** con el idioma de la interfaz: es
/// parte del nombre de un fichero que el usuario va a mandar por correo y a
/// buscar dentro de seis meses, no un rótulo de la ventana. Un nombre que
/// cambia porque alguien cambió el idioma deja dos familias de ficheros en la
/// misma carpeta.
pub const SIGNED_SUFFIX: &str = "-firmado";

/// El primer número que se prueba cuando el nombre está ocupado. El primero no
/// lleva número, así que el segundo es el 2.
pub const FIRST_NUMBER: u32 = 2;

/// Cuántos homónimos se prueban antes de rendirse. No es un límite del
/// usuario: es la garantía de que la búsqueda del hueco **termina**, y de que
/// un directorio con mil `contrato-firmado-N.pdf` da un error y no una espera.
pub const MAX_NAMESAKES: u32 = 999;

/// El nombre que se usa cuando el original no tiene ninguno utilizable.
const FALLBACK_STEM: &str = "documento";

/// El nombre del documento firmado a partir del nombre del original.
///
/// ```text
/// contrato.pdf          -> contrato-firmado.pdf
/// contrato-firmado.pdf  -> contrato-firmado.pdf   (no se apila el sufijo)
/// informe               -> informe-firmado
/// ```
pub fn signed_name(original: &str) -> String {
    let (stem, extension) = split_extension(original);
    let stem = if stem.is_empty() { FALLBACK_STEM } else { stem };
    if ends_with_the_suffix(stem) {
        return format!("{stem}{extension}");
    }
    format!("{stem}{SIGNED_SUFFIX}{extension}")
}

/// El mismo nombre con su número de desempate: `contrato-firmado-2.pdf`.
pub fn numbered(name: &str, number: u32) -> String {
    let (stem, extension) = split_extension(name);
    format!("{stem}-{number}{extension}")
}

/// Si el nombre ya acaba en el sufijo, mirando solo ASCII: el sufijo es ASCII
/// entero, así que comparar por bytes no rompe ninguna tilde del resto.
fn ends_with_the_suffix(stem: &str) -> bool {
    stem.len() >= SIGNED_SUFFIX.len()
        && stem[stem.len() - SIGNED_SUFFIX.len()..].eq_ignore_ascii_case(SIGNED_SUFFIX)
}

/// Parte el nombre en tronco y extensión **con el punto incluido**.
///
/// Un nombre que empieza por punto y no tiene otro —`.oculto`— es tronco
/// entero: `.oculto` no es la extensión de un fichero sin nombre.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 => name.split_at(dot),
        _ => (name, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: cadenas, sin disco.
    #[test]
    fn the_signed_document_takes_the_suffix_before_the_extension() {
        assert_eq!(signed_name("contrato.pdf"), "contrato-firmado.pdf");
        assert_eq!(signed_name("acta 2024.PDF"), "acta 2024-firmado.PDF");
    }

    #[test]
    fn a_name_that_already_ends_in_the_suffix_does_not_take_a_second_one() {
        assert_eq!(signed_name("contrato-firmado.pdf"), "contrato-firmado.pdf");
        assert_eq!(signed_name("contrato-FIRMADO.pdf"), "contrato-FIRMADO.pdf");
    }

    #[test]
    fn a_name_without_extension_keeps_not_having_one() {
        assert_eq!(signed_name("informe"), "informe-firmado");
    }

    #[test]
    fn a_dotfile_is_all_stem_and_not_an_extension_without_a_name() {
        assert_eq!(signed_name(".oculto"), ".oculto-firmado");
    }

    #[test]
    fn a_document_without_a_usable_name_still_gets_one() {
        assert_eq!(signed_name(""), "documento-firmado");
    }

    #[test]
    fn the_number_goes_after_the_suffix_and_before_the_extension() {
        assert_eq!(
            numbered("contrato-firmado.pdf", FIRST_NUMBER),
            "contrato-firmado-2.pdf"
        );
        assert_eq!(numbered("informe-firmado", 3), "informe-firmado-3");
    }

    #[test]
    fn the_first_namesake_is_the_second_file_because_the_first_carries_no_number() {
        assert_eq!(FIRST_NUMBER, 2);
    }
}
