//! Una URL `afirma://` partida en lo único que tiene: un verbo y unos pares.
//!
//! El transporte de la 1.9.2 manda **URLs por el socket** —«*la comunicación
//! por sockets/websockets no debería utilizar URLs*», dice el propio AutoFirma—
//! así que tanto la invocación de arranque como cada operación posterior llegan
//! con esta forma. Partirlas es lo primero que pasa en las dos, y por eso está
//! aquí y no en [`super::launch`].
//!
//! Se reproduce `extractParams` del original
//! (`ProtocolInvocationLauncher.java:942`-`966`) con sus tres rarezas, porque el
//! banco de conformidad manda lo que manda el `autoscript.js` publicado y no lo
//! que sería razonable:
//!
//! - **un par sin `=`, o con el `=` en la posición 0, se descarta** (el
//!   original exige `equalsPos > 0`);
//! - **la clave no se descodifica**, sólo el valor;
//! - **el valor pasa por `URLDecoder`**, que convierte `+` en espacio. Da igual
//!   para lo que mandan las sedes —el Base64 del protocolo es URL-safe y no
//!   lleva `+`— pero cambiarlo sería hablar otro idioma que el original.
//!
//! No se usa ningún crate de URL: el idioma que hay que hablar no es el de la
//! RFC 3986, es el de `extractParams`.

use std::collections::BTreeMap;

use super::refusal::Refusal;

/// El esquema, lo único que el original comprueba de la cadena entera
/// (`ProtocolInvocationLauncher.java:172`-`178`).
const SCHEME: &str = "afirma://";

/// Una URL `afirma://` ya partida.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfirmaUrl {
    verb: String,
    parameters: BTreeMap<String, String>,
}

impl AfirmaUrl {
    /// Parte la cadena, o dice por qué no es una URL del protocolo.
    ///
    /// El esquema se compara **sin distinguir mayúsculas**: quien entrega la
    /// cadena es el sistema operativo a través del manejador registrado, y un
    /// esquema es insensible a las mayúsculas por definición. El verbo, en
    /// cambio, se guarda tal cual: los que reconoce el original —`websocket`,
    /// `selectcert`, `sign`, `cosign`— van siempre en minúsculas.
    ///
    /// Una clave repetida se queda con **el último** valor, que es lo que hace
    /// el `HashMap` del original.
    pub fn parse(url: &str) -> Result<Self, Refusal> {
        let Some(rest) = strip_scheme(url) else {
            return Err(Refusal::params(format!(
                "la invocacion no empieza por {SCHEME}: {url}"
            )));
        };

        let (verb, query) = match rest.split_once('?') {
            Some((verb, query)) => (verb, query),
            None => (rest, ""),
        };

        if verb.is_empty() {
            return Err(Refusal::params(format!(
                "la invocacion no trae verbo: {url}"
            )));
        }

        let mut parameters = BTreeMap::new();
        for pair in query.split('&') {
            let Some(position) = pair.find('=') else {
                continue;
            };
            if position == 0 {
                continue;
            }
            let (key, value) = pair.split_at(position);
            parameters.insert(key.to_owned(), url_decode(&value[1..]));
        }

        Ok(Self {
            verb: verb.to_owned(),
            parameters,
        })
    }

    /// Lo que va entre el esquema y el `?`: `websocket`, `sign`, `selectcert`…
    pub fn verb(&self) -> &str {
        &self.verb
    }

    /// El valor de un parámetro, ya descodificado, si vino.
    pub fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters.get(name).map(String::as_str)
    }
}

/// Quita `afirma://` sin distinguir mayúsculas, o dice que no estaba.
fn strip_scheme(url: &str) -> Option<&str> {
    let head = url.get(..SCHEME.len())?;
    head.eq_ignore_ascii_case(SCHEME)
        .then(|| &url[SCHEME.len()..])
}

/// `URLDecoder.decode(valor, "UTF-8")`, con la tolerancia que el original no
/// tiene.
///
/// El original lanza si el escape está mal formado; aquí un `%` suelto o un
/// `%ZZ` se deja **literal**. La diferencia no la puede provocar el cliente
/// publicado —`encodeURIComponent` no produce escapes rotos— y romper por ella
/// convertiría un byte perdido en una invocación entera rechazada.
fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => match hexadecimal_byte(bytes.get(index + 1..index + 3)) {
                Some(byte) => {
                    decoded.push(byte);
                    index += 3;
                }
                None => {
                    decoded.push(b'%');
                    index += 1;
                }
            },
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

/// Los dos dígitos hexadecimales de un `%XX`, si los dos lo son.
fn hexadecimal_byte(digits: Option<&[u8]>) -> Option<u8> {
    let digits = digits?;
    if digits.len() != 2 {
        return None;
    }
    let high = (digits[0] as char).to_digit(16)?;
    let low = (digits[1] as char).to_digit(16)?;
    Some((high * 16 + low) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codes::SafCode;

    #[test]
    fn the_launch_invocation_is_split_into_verb_and_parameters() {
        let url = AfirmaUrl::parse(
            "afirma://websocket?ports=49152,50001,60123&v=4&jvc=3&idsession=abc123",
        )
        .expect("la invocacion de arranque publicada deberia parsearse");

        assert_eq!(url.verb(), "websocket");
        assert_eq!(url.parameter("ports"), Some("49152,50001,60123"));
        assert_eq!(url.parameter("v"), Some("4"));
        assert_eq!(url.parameter("jvc"), Some("3"));
        assert_eq!(url.parameter("idsession"), Some("abc123"));
        assert_eq!(url.parameter("mcv"), None);
    }

    #[test]
    fn a_url_without_query_is_still_a_verb() {
        let url = AfirmaUrl::parse("afirma://websocket").expect("un verbo suelto es valido");

        assert_eq!(url.verb(), "websocket");
        assert_eq!(url.parameter("ports"), None);
    }

    #[test]
    fn the_scheme_is_compared_ignoring_case() {
        let url =
            AfirmaUrl::parse("AFIRMA://sign?op=sign").expect("el esquema no lleva mayusculas");

        assert_eq!(url.verb(), "sign");
    }

    #[test]
    fn anything_that_is_not_the_scheme_is_refused_as_a_parameter_error() {
        for url in [
            "https://sede.example/sign",
            "afirma:/websocket",
            "",
            "afirma://",
        ] {
            let refusal = AfirmaUrl::parse(url).expect_err("no es una invocacion del protocolo");
            assert_eq!(refusal.code(), SafCode::Params, "con {url}");
        }
    }

    #[test]
    fn a_pair_without_a_key_is_dropped_like_in_the_original() {
        let url = AfirmaUrl::parse("afirma://sign?=huerfano&op=sign&suelto").expect("parsea");

        assert_eq!(url.parameter("op"), Some("sign"));
        assert_eq!(url.parameter(""), None);
        assert_eq!(url.parameter("suelto"), None);
    }

    #[test]
    fn a_repeated_key_keeps_the_last_value() {
        let url = AfirmaUrl::parse("afirma://websocket?v=3&v=4").expect("parsea");

        assert_eq!(url.parameter("v"), Some("4"));
    }

    #[test]
    fn the_value_is_decoded_and_the_key_is_not() {
        let url = AfirmaUrl::parse("afirma://sign?a%20b=uno%20dos&c=m%C3%A1s").expect("parsea");

        assert_eq!(url.parameter("a%20b"), Some("uno dos"));
        assert_eq!(url.parameter("a b"), None);
        assert_eq!(url.parameter("c"), Some("más"));
    }

    #[test]
    fn a_plus_becomes_a_space_because_url_decoder_says_so() {
        let url = AfirmaUrl::parse("afirma://sign?dat=a+b").expect("parsea");

        assert_eq!(url.parameter("dat"), Some("a b"));
    }

    #[test]
    fn a_broken_escape_stays_literal_instead_of_sinking_the_invocation() {
        let url = AfirmaUrl::parse("afirma://sign?dat=100%&op=%ZZ&x=%4").expect("parsea");

        assert_eq!(url.parameter("dat"), Some("100%"));
        assert_eq!(url.parameter("op"), Some("%ZZ"));
        assert_eq!(url.parameter("x"), Some("%4"));
    }

    #[test]
    fn an_empty_value_is_not_an_absent_parameter() {
        let url = AfirmaUrl::parse("afirma://websocket?idsession=").expect("parsea");

        assert_eq!(url.parameter("idsession"), Some(""));
    }
}
