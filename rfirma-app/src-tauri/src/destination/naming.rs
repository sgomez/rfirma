//! Cómo se llama el documento firmado, y qué pasa cuando ese nombre ya está
//! ocupado (ADR-0011).
//!
//! Aquí no hay disco: son cadenas. Quien mira si el nombre está libre es
//! [`super::CheckedFolder::landing_for`], y por eso el conteo empieza en
//! **2**: el primero no lleva número.
//!
//! El detalle que parece un capricho y no lo es: **no se apila un segundo
//! sufijo**, y tampoco a la tercera vuelta. Cofirmar `contrato-firmado.pdf` da
//! `contrato-firmado-2.pdf`, y cofirmar *ese* da `contrato-firmado-3.pdf`, no
//! `contrato-firmado-2-firmado.pdf`. Para eso [`signed_name`] quita el número
//! de desempate antes de reconocer el sufijo: el nombre que devuelve es
//! siempre el canónico, y el número lo vuelve a poner quien mira la carpeta.
//! Un documento que se cofirma tres veces es el caso normal de rFirma, no el
//! raro, y el nombre no puede crecer con cada firma.

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
/// contrato.pdf            -> contrato-firmado.pdf
/// contrato-firmado.pdf    -> contrato-firmado.pdf   (no se apila el sufijo)
/// contrato-firmado-2.pdf  -> contrato-firmado.pdf   (tampoco a la tercera)
/// informe                 -> informe-firmado
/// ```
///
/// El nombre que sale es el **canónico**, sin número: quien mira si está
/// ocupado y le cuelga el `-2`, `-3`… es
/// [`super::CheckedFolder::landing_for`]. Por eso el número de desempate se
/// quita antes de reconocer el sufijo — si no, la tercera cofirma volvería a
/// apilarlo (`contrato-firmado-2-firmado.pdf`) y el nombre crecería con cada
/// firma, que es justo lo que este módulo promete que no pasa.
pub fn signed_name(original: &str) -> String {
    let (stem, extension) = split_extension(original);
    let stem = if stem.is_empty() { FALLBACK_STEM } else { stem };
    let unnumbered = without_the_number(stem);
    if ends_with_the_suffix(unnumbered) {
        return format!("{unnumbered}{extension}");
    }
    format!("{stem}{SIGNED_SUFFIX}{extension}")
}

/// El mismo nombre con su número de desempate: `contrato-firmado-2.pdf`.
pub fn numbered(name: &str, number: u32) -> String {
    let (stem, extension) = split_extension(name);
    format!("{stem}-{number}{extension}")
}

/// Si el nombre ya acaba en el sufijo, sin distinguir mayúsculas.
///
/// La comparación es **sobre `as_bytes()`, no sobre un corte del `&str`**: el
/// sufijo es ASCII entero, pero el tronco no tiene por qué serlo, y cortar un
/// `&str` por el byte `len - 8` entra en pánico si ese byte cae dentro de un
/// carácter multibyte —`árbitros.pdf`, `1ª parte.pdf`—. Sobre bytes la
/// intención se conserva y el pánico desaparece: un tronco cuyos últimos ocho
/// bytes no sean los del sufijo simplemente no coincide.
fn ends_with_the_suffix(stem: &str) -> bool {
    let (stem, suffix) = (stem.as_bytes(), SIGNED_SUFFIX.as_bytes());
    stem.len() >= suffix.len() && stem[stem.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// El tronco sin su número de desempate final: `contrato-firmado-2` da
/// `contrato-firmado`. Si no lo lleva, o lo que sigue al guion no son solo
/// dígitos, devuelve el tronco tal cual.
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

    /// Un tronco corto que empieza por una vocal acentuada colocaba el corte
    /// del sufijo dentro de la tilde y tumbaba la aplicación entera. En una
    /// aplicación española que firma lo que le entra por el diálogo, esto no
    /// es un caso de laboratorio.
    #[test]
    fn an_accented_name_is_signed_and_does_not_panic() {
        assert_eq!(signed_name("árbitros.pdf"), "árbitros-firmado.pdf");
        assert_eq!(signed_name("1ª parte.pdf"), "1ª parte-firmado.pdf");
        assert_eq!(signed_name("ñ.pdf"), "ñ-firmado.pdf");
        assert_eq!(
            signed_name("Acuerdo Marco-firmado.pdf"),
            "Acuerdo Marco-firmado.pdf"
        );
    }

    /// La tercera cofirma es el caso que el módulo declara normal: el nombre
    /// vuelve al canónico y es la carpeta quien le pone el número.
    #[test]
    fn the_suffix_does_not_stack_on_a_name_that_already_carries_a_number() {
        assert_eq!(
            signed_name("contrato-firmado-2.pdf"),
            "contrato-firmado.pdf"
        );
        assert_eq!(
            signed_name("contrato-FIRMADO-9.pdf"),
            "contrato-FIRMADO.pdf"
        );
    }

    /// Quitar el número solo vale si lo que queda es el sufijo: un `-2` que
    /// forma parte del nombre del usuario se queda donde está.
    #[test]
    fn a_number_that_is_part_of_the_original_name_survives() {
        assert_eq!(signed_name("informe-2.pdf"), "informe-2-firmado.pdf");
        assert_eq!(signed_name("anexo-2b.pdf"), "anexo-2b-firmado.pdf");
        assert_eq!(signed_name("acta-.pdf"), "acta--firmado.pdf");
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
