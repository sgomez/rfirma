//! **Lo que la sede pide por el canal ya abierto**, leído de la URL y nada más
//! (ID-244, TD-53).
//!
//! [`crate::protocol::message`] dice *qué es* lo que llegó —el eco, una
//! operación o nada del protocolo—; este módulo lee **la operación**: qué verbo
//! pide la sede y qué trae consigo. Aquí no hay token, ni puente, ni socket: se
//! lee una URL y sale una petición o el rechazo con su código.
//!
//! # El verbo viaja dos veces
//!
//! `buildUrl` del cliente publicado pone la operación **en el dominio de la
//! URL** (`afirma://selectcert?…`) y **otra vez en un parámetro** `op=`
//! (`autoscript.js:2077`; lado servidor, `ProtocolInvocationLauncher.java:643`
//! -`645`). El original se queda con el parámetro, y aquí también: si vienen
//! los dos y no coinciden, manda `op`.
//!
//! # Las dos guardias comunes se comprueban aquí
//!
//! `mcv` y el `dat` que pide un fichero local son de **toda** operación
//! ([`super::parameters`], ID-250, ID-267), y el original las comprueba en los
//! cuatro lanzadores, `selectcert` incluido. Se aplican antes de mirar el
//! verbo: una petición que pide leer un fichero del equipo se rechaza aunque
//! el verbo no se atienda.
//!
//! # `selectcert` no tiene nada suyo salvo el filtro
//!
//! De los campos que el cliente publicado manda
//! (`docs/research/contrato-protocolo-afirma.md`, §3.2), lo único que cambia lo
//! que rFirma hace es `properties`, porque dentro viajan los filtros
//! ([`super::filters`]). `ksb64` —el almacén por defecto— no se atiende: rFirma
//! tiene los suyos y no los elige la sede. `sticky` y `resetsticky` tampoco:
//! reutilizar el certificado ya elegido sin preguntar es justo la firma
//! silenciosa que el ID-272 prohíbe.

use base64::Engine as _;

use super::codes::{Parameter, SafCode};
use super::filters::{site_filter, SiteFilter};
use super::parameters::{check_local_access_is_not_requested, check_minimum_client_version};
use super::refusal::Refusal;
use super::url::AfirmaUrl;

/// El verbo de la selección de certificado, tal y como viaja por el cable.
///
/// **No es el nombre que el JS usa por dentro**: allí la constante se llama
/// `OPERATION_SELECT_CERTIFICATE = "certificate"` (`autoscript.js:1761`), que
/// es sólo la etiqueta con la que el cliente recuerda qué respuesta espera. Lo
/// que viaja es esto (`autoscript.js:1943`).
pub const SELECT_CERTIFICATE: &str = "selectcert";

/// Lo que la sede pide, ya leído.
///
/// Hoy sólo la selección de certificado. Las firmas —`sign`, `cosign` y el
/// `countersign` que en PAdES es un `SAF_04` (ID-263)— entran por aquí cuando
/// se atiendan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteOperation {
    /// `selectcert`: la sede pide identidad, no una firma (ID-276).
    SelectCertificate(SelectCertificate),
}

/// La petición de `selectcert`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectCertificate {
    filter: SiteFilter,
}

impl SelectCertificate {
    /// Lo que la sede pide del listado (ID-252).
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }
}

/// Lee la operación que llegó por el canal, o por qué se rechaza.
pub fn read_operation(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    check_minimum_client_version(url.parameter("mcv"))?;
    if let Some(data) = url.parameter("dat") {
        check_local_access_is_not_requested(data)?;
    }

    match verb_of(url).as_str() {
        SELECT_CERTIFICATE => Ok(SiteOperation::SelectCertificate(SelectCertificate {
            filter: site_filter(&declared_properties(url)?)?,
        })),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("la operacion '{other}' no se atiende"),
        )),
    }
}

/// El verbo que pide la sede: el parámetro `op` si viene, y si no, el dominio
/// de la URL.
fn verb_of(url: &AfirmaUrl) -> String {
    url.parameter("op")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| url.verb())
        .trim()
        .to_ascii_lowercase()
}

/// Los pares del `.properties` que la sede mandó dentro de `properties`.
///
/// Viaja en Base64 **URL-safe** (`Base64.encode(bytes, true)` del original), y
/// el descodificador es tolerante a propósito: acepta también el alfabeto
/// normal y el relleno ausente. Un cliente que mande `+` y `/` no está
/// atacando nada, y rechazarle la llamada entera por eso sería inventarse una
/// incompatibilidad que el original no tiene.
fn declared_properties(url: &AfirmaUrl) -> Result<Vec<(String, String)>, Refusal> {
    let Some(encoded) = url.parameter("properties").filter(|it| !it.is_empty()) else {
        return Ok(Vec::new());
    };

    let normalized: String = encoded
        .chars()
        .filter(|character| *character != '=')
        .map(|character| match character {
            '+' => '-',
            '/' => '_',
            other => other,
        })
        .collect();

    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(normalized.as_bytes())
        .map_err(|error| {
            Refusal::about(
                Parameter::Properties,
                format!("el parametro 'properties' no es Base64: {error}"),
            )
        })?;

    let text = String::from_utf8(decoded).map_err(|error| {
        Refusal::about(
            Parameter::Properties,
            format!("el parametro 'properties' no es texto: {error}"),
        )
    })?;

    Ok(pairs_of(&text))
}

/// Los pares de un bloque `java.util.Properties`, leído como lo escribe
/// `SiteFilter::as_java_properties`: una clave por línea, `=` o `:` de
/// separador, `#` y `!` de comentario, y las secuencias de escape con barra.
///
/// **No es un lector completo de `.properties`**: no hay continuación de línea
/// ni `\uXXXX`. Lo que se lee aquí son expresiones de filtro que el original
/// escribe en una línea cada una, y lo que no se reconozca cruza al motor tal
/// cual, que es quien manda (ID-253).
fn pairs_of(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for line in text.lines() {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some(at) = separator_in(line) else {
            continue;
        };
        let key = unescape(line[..at].trim_end());
        let value = unescape(line[at + 1..].trim_start());
        if !key.is_empty() {
            pairs.push((key, value));
        }
    }

    pairs
}

/// Dónde parte la línea: el primer `=` o `:` que no venga escapado.
fn separator_in(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'=' | b':' => return Some(index),
            _ => {}
        }
    }
    None
}

/// Deshace las barras de escape: las tres que escribe el proyecto —`\\`, `\n`,
/// `\r`— más `\t`, y cualquier otra barra que se queda con lo que lleve detrás.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => break,
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: se lee una cadena y sale una petición. No hay socket, ni
    /// token, ni puente.
    fn an_operation(parameters: &str) -> AfirmaUrl {
        AfirmaUrl::parse(&format!("afirma://selectcert?{parameters}")).expect("es del protocolo")
    }

    fn properties(text: &str) -> String {
        base64::engine::general_purpose::URL_SAFE.encode(text.as_bytes())
    }

    #[test]
    fn the_verb_of_the_published_client_is_the_selection_of_a_certificate() {
        let operation = read_operation(&an_operation(
            "op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL",
        ))
        .expect("es una operacion que se atiende");

        let SiteOperation::SelectCertificate(request) = operation;
        assert!(
            request.filter().declares_nothing(),
            "sin 'properties' no hay filtro declarado"
        );
    }

    /// El verbo va dos veces en la URL, y manda el parámetro.
    #[test]
    fn the_parameter_wins_over_the_domain_of_the_url() {
        let url = AfirmaUrl::parse("afirma://sign?op=selectcert").expect("es del protocolo");

        read_operation(&url).expect("el 'op' es el que manda");
    }

    /// Y sin parámetro, el dominio basta.
    #[test]
    fn without_the_parameter_the_domain_of_the_url_is_the_verb() {
        read_operation(&an_operation("idsession=8jAkPZfRw2mQxN4TbYuL")).expect("el dominio vale");
    }

    /// ID-263: lo que no se atiende es `SAF_04`, que es lo que el original
    /// contesta a una operación desconocida.
    #[test]
    fn an_operation_that_is_not_attended_is_refused_with_the_code_of_the_original() {
        let url = AfirmaUrl::parse("afirma://batch?op=batch").expect("es del protocolo");

        let refusal = read_operation(&url).expect_err("no se atiende");

        assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
    }

    /// Las dos guardias comunes se comprueban **también** en `selectcert`
    /// (ID-250, ID-267), y antes de mirar el verbo.
    #[test]
    fn the_two_common_guards_are_checked_in_the_selection_of_a_certificate_too() {
        let too_new = read_operation(&an_operation("op=selectcert&mcv=99.9.9"))
            .expect_err("la sede exige una version que no se implementa");
        assert_eq!(too_new.code(), SafCode::MinimumVersionNonSatisfied);

        let local = read_operation(&an_operation("op=selectcert&dat=file:///etc/shadow"))
            .expect_err("pide leer un fichero del equipo");
        assert_eq!(local.code(), SafCode::Params);
        assert_eq!(local.blame(), Some(Parameter::Data));
    }

    /// El filtro viaja **dentro** de `properties`, en Base64 URL-safe, y sale
    /// tal y como la sede lo escribió (ID-256).
    #[test]
    fn the_filter_travels_inside_the_properties_and_comes_out_untouched() {
        let url = an_operation(&format!(
            "op=selectcert&properties={}",
            properties("filters=subject.contains:PEREZ\n")
        ));

        let SiteOperation::SelectCertificate(request) =
            read_operation(&url).expect("el criterio esta en la lista blanca");

        assert_eq!(
            request.filter().declared(),
            [("filters".to_owned(), "subject.contains:PEREZ".to_owned())]
        );
    }

    /// ID-256: un criterio fuera de la lista blanca rechaza la invocación
    /// entera, y el rechazo llega hasta aquí.
    #[test]
    fn a_criterion_outside_the_whitelist_refuses_the_whole_call() {
        let url = an_operation(&format!(
            "op=selectcert&properties={}",
            properties("filters=inventado:loquesea\n")
        ));

        let refusal = read_operation(&url).expect_err("el criterio no esta en la lista blanca");

        assert_eq!(refusal.code(), SafCode::Params);
    }

    /// El alfabeto normal se acepta igual que el URL-safe: rechazar la llamada
    /// por eso sería una incompatibilidad que el original no tiene.
    #[test]
    fn the_plain_base64_alphabet_is_accepted_too() {
        let plain = base64::engine::general_purpose::STANDARD
            .encode(b"filters=subject.contains:ANIA\xc3\x91EZ\n");
        let url = an_operation(&format!("op=selectcert&properties={plain}"));

        read_operation(&url).expect("se lee igual");
    }

    #[test]
    fn properties_that_are_not_base64_name_the_parameter_that_came_wrong() {
        let url = an_operation("op=selectcert&properties=!!!!");

        let refusal = read_operation(&url).expect_err("no es Base64");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.blame(), Some(Parameter::Properties));
    }

    /// El bloque se lee como lo escribe el proyecto: comentarios fuera, los dos
    /// separadores y las barras de escape deshechas.
    #[test]
    fn the_properties_block_is_read_the_way_the_original_writes_it() {
        let pairs = pairs_of("# un comentario\n\nfilters=subject.rfc2254:\\(cn=X\\)\nkey:valor\n");

        assert_eq!(
            pairs,
            vec![
                ("filters".to_owned(), "subject.rfc2254:(cn=X)".to_owned()),
                ("key".to_owned(), "valor".to_owned()),
            ]
        );
    }

    /// Y una clave escapada no parte la línea donde no debe.
    #[test]
    fn an_escaped_separator_does_not_split_the_line() {
        let pairs = pairs_of("cla\\=ve=valor\n");

        assert_eq!(pairs, vec![("cla=ve".to_owned(), "valor".to_owned())]);
    }
}
