//! La invocación de arranque: `afirma://websocket?ports=…&v=4&jvc=3&idsession=…`
//!
//! Es la **única** URL del esquema que llega al sistema operativo
//! (`autoscript.js:2138`-`2158`); todo lo demás viaja luego por dentro del
//! canal. Trae exactamente cuatro parámetros, y de los cuatro este módulo lee
//! tres: `jvc` se ignora a propósito (ID-245), porque en la 1.9.2 su único
//! efecto es un aviso modal que rFirma no da.
//!
//! Tres reglas se apartan del original, y las tres cierran la puerta que el
//! original deja abierta:
//!
//! - **`v` ausente es la versión 1**, no la 4 (`getVersion`,
//!   `ProtocolInvocationLauncher.java:923`-`939`), así que el camino de versión
//!   no soportada se alcanza **por omisión**, que es el caso realista (ID-246).
//! - **Sólo se habla el 4.** El original admite también el 3, cuyo camino es
//!   puerto fijo `63117` y **sin credencial de canal**; rFirma no abre nunca un
//!   canal sin credencial, así que el 3 no existe aquí (ID-247). No rompe
//!   compatibilidad: el `autoscript.js` de la 1.9.2 manda siempre `v=4`.
//! - **Un `idsession` mal formado se rechaza.** El original lo pone a `null`, y
//!   un `null` desactiva la comprobación del canal entera
//!   (`AfirmaWebSocketServerV4.java:72`): un `idsession` malo abre ahí un canal
//!   sin cerradura. Aquí es `SAF_03` (ID-249).

use super::refusal::{Refusal, SafCode};
use super::url::AfirmaUrl;

/// El verbo de la invocación de arranque, y el único que abre canal.
pub const LAUNCH_VERB: &str = "websocket";

/// La versión de protocolo que se habla, y la única que se acepta (ID-245,
/// ID-247).
pub const PROTOCOL_VERSION: i64 = 4;

/// La versión que se supone cuando la sede no manda `v`. No está soportada, y
/// ese es justo el punto (ID-246).
const VERSION_WHEN_ABSENT: i64 = 1;

/// El `idsession`: la **credencial del canal**, no un identificador de
/// transacción.
///
/// Es lo único que impide que otra página del mismo equipo use el canal ya
/// abierto: viaja en la invocación de arranque y se repite en cada mensaje
/// posterior, eco incluido. Existir como tipo propio, y no como `String`, es lo
/// que hace imposible construirla sin pasar por su validación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelCredential(String);

impl ChannelCredential {
    /// La credencial, si el valor está bien formado.
    ///
    /// La regla de bien formado es **la del original** (`getChannelInfo`,
    /// `ProtocolInvocationLauncher.java:992`-`1008`): no vacía y sólo letras o
    /// dígitos. El suelo de 16 caracteres que pedía el sentido común se
    /// descarta por compatibilidad —el cliente publicado sortea veinte, pero
    /// nada del contrato lo garantiza—. Lo que sí se estrecha es el alfabeto:
    /// `Character.isLetterOrDigit` acepta cualquier letra Unicode, y aquí sólo
    /// pasa `[A-Za-z0-9]`.
    pub fn parse(value: &str) -> Result<Self, Refusal> {
        if value.is_empty() {
            return Err(Refusal::params(
                "la invocacion no trae credencial de canal ('idsession'), y sin ella el canal \
                 quedaria sin cerradura",
            ));
        }
        if !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(Refusal::params(
                "la credencial de canal ('idsession') tiene caracteres que no son letras ni \
                 digitos ASCII",
            ));
        }

        Ok(Self(value.to_owned()))
    }

    /// La credencial tal cual, para compararla con la de cada mensaje.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Lo que pide una invocación de arranque, ya leída.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchRequest {
    ports: Vec<u16>,
    credential: ChannelCredential,
}

impl LaunchRequest {
    /// Lee la invocación de arranque, o dice con qué `SAF_` se rechaza.
    pub fn parse(url: &str) -> Result<Self, Refusal> {
        Self::from_url(&AfirmaUrl::parse(url)?)
    }

    /// Lo mismo, sobre una URL ya partida.
    pub fn from_url(url: &AfirmaUrl) -> Result<Self, Refusal> {
        if url.verb() != LAUNCH_VERB {
            return Err(Refusal::params(format!(
                "la invocacion de arranque es 'afirma://{LAUNCH_VERB}', y esta es \
                 'afirma://{}'",
                url.verb()
            )));
        }

        check_protocol_version(url.parameter("v"))?;

        let ports = parse_ports(url.parameter("ports"))?;
        let credential = ChannelCredential::parse(url.parameter("idsession").unwrap_or_default())?;

        Ok(Self { ports, credential })
    }

    /// Los puertos sorteados por la sede, en el orden en que los mandó: se
    /// prueban de uno en uno hasta que alguno abra.
    pub fn ports(&self) -> &[u16] {
        &self.ports
    }

    /// La credencial que cerrará el canal.
    pub fn credential(&self) -> &ChannelCredential {
        &self.credential
    }
}

/// **Los puertos que la sede sorteó, se acepte la invocación o no** (ID-248).
///
/// Existe aparte de [`LaunchRequest`] porque un rechazo se contesta *por el
/// socket cuando hay socket*: para atarse a un puerto y decir el `SAF_` hace
/// falta leer `ports` de una URL que, por lo demás, ya se ha rechazado —una
/// versión de protocolo que no se habla, una credencial mal formada—. Lo que no
/// se pueda leer es una lista vacía, y una lista vacía es el único caso en el
/// que el rechazo sale sólo por ventana.
pub fn drawn_ports(url: &AfirmaUrl) -> Vec<u16> {
    parse_ports(url.parameter("ports")).unwrap_or_default()
}

/// `v`, con la regla que hace que la omisión caiga en «no soportada».
///
/// Un `v` que no es un entero **no** es un rechazo aparte: el original lo
/// registra y se queda con el 1 (`getVersion`), o sea con el mismo camino que
/// la omisión. Aquí igual, y por eso el rechazo es siempre `SAF_21`.
fn check_protocol_version(declared: Option<&str>) -> Result<(), Refusal> {
    let version = declared
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(VERSION_WHEN_ABSENT);

    if version == PROTOCOL_VERSION {
        return Ok(());
    }

    Err(Refusal::new(
        SafCode::UnsupportedProcedure,
        format!("la sede declara la version de protocolo {version} y aqui se habla la {PROTOCOL_VERSION}"),
    ))
}

/// `ports`, los tres puertos sorteados.
///
/// El original hace `Math.abs` sobre cada valor y lanza si alguno no es
/// numérico (`getChannelInfo`, `ProtocolInvocationLauncher.java:977`-`990`). Se
/// reproduce el valor absoluto y se añade lo que el original se deja: un puerto
/// fuera de `1..=65535` no se puede atar, así que es un parámetro malo y no un
/// intento de conexión que fracasa después. Sin `ports`, el original se iría al
/// camino del protocolo 3, que aquí no existe (ID-247).
fn parse_ports(declared: Option<&str>) -> Result<Vec<u16>, Refusal> {
    let Some(declared) = declared.filter(|value| !value.is_empty()) else {
        return Err(Refusal::params(
            "la invocacion no trae puertos ('ports'), y el camino sin puertos del original es el \
             del protocolo 3",
        ));
    };

    declared
        .split(',')
        .map(|port| {
            port.parse::<i64>()
                .ok()
                .map(i64::unsigned_abs)
                .and_then(|port| u16::try_from(port).ok())
                .filter(|port| *port != 0)
                .ok_or_else(|| {
                    Refusal::params(format!(
                        "el parametro 'ports' trae un valor que no es un puerto: {port}"
                    ))
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La invocación que manda el `autoscript.js` publicado, tal cual.
    const PUBLISHED: &str =
        "afirma://websocket?ports=49152,50001,60123&v=4&jvc=3&idsession=BQXf7mJ2Kd9pLzR3tYvW";

    #[test]
    fn the_launch_invocation_the_published_client_sends_is_read_whole() {
        let request =
            LaunchRequest::parse(PUBLISHED).expect("la invocacion publicada deberia valer");

        assert_eq!(request.ports(), [49152, 50001, 60123]);
        assert_eq!(request.credential().as_str(), "BQXf7mJ2Kd9pLzR3tYvW");
    }

    #[test]
    fn the_javascript_version_code_is_ignored_on_purpose() {
        for jvc in ["jvc=3", "jvc=0", "jvc=noesunnumero", ""] {
            let url = format!("afirma://websocket?ports=49152&v=4&{jvc}&idsession=abc");
            assert!(LaunchRequest::parse(&url).is_ok(), "con {jvc}");
        }
    }

    #[test]
    fn an_absent_version_is_version_one_and_therefore_unsupported() {
        let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&idsession=abc")
            .expect_err("sin 'v' la version es la 1");

        assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
        assert!(refusal.detail().contains("version de protocolo 1"));
    }

    #[test]
    fn the_third_protocol_does_not_exist_here() {
        let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&v=3&idsession=abc")
            .expect_err("el protocolo 3 abriria un canal sin credencial");

        assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
    }

    #[test]
    fn a_version_that_is_not_a_number_falls_into_the_same_hole_as_an_absent_one() {
        let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&v=4.1&idsession=abc")
            .expect_err("la 4.1 no existe y no hay forma de expresarla");

        assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
        assert!(refusal.detail().contains("version de protocolo 1"));
    }

    #[test]
    fn a_version_written_with_spaces_is_trimmed_like_in_the_original() {
        let request =
            LaunchRequest::parse("afirma://websocket?ports=49152&v=%204%20&idsession=abc")
                .expect("el original hace trim antes de parsear");

        assert_eq!(request.ports(), [49152]);
    }

    #[test]
    fn a_malformed_channel_credential_is_refused_instead_of_nulled() {
        for idsession in [
            "idsession=",
            "idsession=abc-def",
            "idsession=abc def",
            "idsession=ñ",
        ] {
            let url = format!("afirma://websocket?ports=49152&v=4&{idsession}");
            let refusal = LaunchRequest::parse(&url)
                .expect_err("un idsession malo abriria un canal sin cerradura");

            assert_eq!(refusal.code(), SafCode::Params, "con {idsession}");
        }
    }

    #[test]
    fn an_absent_channel_credential_is_refused_too() {
        let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&v=4")
            .expect_err("no hay canal sin credencial");

        assert_eq!(refusal.code(), SafCode::Params);
    }

    #[test]
    fn a_short_credential_is_accepted_because_the_original_has_no_floor() {
        let request = LaunchRequest::parse("afirma://websocket?ports=49152&v=4&idsession=a")
            .expect("un solo caracter esta bien formado");

        assert_eq!(request.credential().as_str(), "a");
    }

    /// Los puertos se leen **aunque la invocación se rechace**: es lo que
    /// permite contestar el `SAF_` por el socket en vez de por la ventana
    /// (ID-248).
    #[test]
    fn the_drawn_ports_are_readable_from_a_launch_that_is_refused() {
        let url =
            AfirmaUrl::parse("afirma://websocket?ports=54001,54002&v=3&idsession=malformado!")
                .expect("es una URL del protocolo");

        assert!(
            LaunchRequest::from_url(&url).is_err(),
            "ni la version ni la credencial valen"
        );
        assert_eq!(drawn_ports(&url), vec![54001, 54002]);
    }

    /// Lo que no se puede leer es una lista vacía, y una lista vacía es el
    /// único caso en el que el rechazo sale sólo por ventana.
    #[test]
    fn a_launch_without_readable_ports_draws_none() {
        let without = AfirmaUrl::parse("afirma://websocket?v=3").expect("es una URL del protocolo");
        let unreadable = AfirmaUrl::parse("afirma://websocket?ports=setenta&v=4")
            .expect("es una URL del protocolo");

        assert!(drawn_ports(&without).is_empty());
        assert!(drawn_ports(&unreadable).is_empty());
    }

    #[test]
    fn the_ports_keep_the_order_the_site_drew_them_in() {
        let request =
            LaunchRequest::parse("afirma://websocket?ports=60123,49152,50001&v=4&idsession=abc")
                .expect("parsea");

        assert_eq!(request.ports(), [60123, 49152, 50001]);
    }

    #[test]
    fn a_negative_port_is_taken_by_its_absolute_value_like_in_the_original() {
        let request = LaunchRequest::parse("afirma://websocket?ports=-49152&v=4&idsession=abc")
            .expect("el original hace Math.abs");

        assert_eq!(request.ports(), [49152]);
    }

    #[test]
    fn ports_that_cannot_be_bound_are_a_parameter_error() {
        for ports in [
            "ports=",
            "ports=abc",
            "ports=49152,abc",
            "ports=0",
            "ports=70000",
            "ports=-9223372036854775808",
        ] {
            let url = format!("afirma://websocket?{ports}&v=4&idsession=abc");
            let refusal = LaunchRequest::parse(&url).expect_err("no es un puerto");

            assert_eq!(refusal.code(), SafCode::Params, "con {ports}");
        }
    }

    #[test]
    fn an_invocation_without_ports_does_not_fall_back_to_the_fixed_port() {
        let refusal = LaunchRequest::parse("afirma://websocket?v=4&idsession=abc")
            .expect_err("el camino sin puertos es el del protocolo 3");

        assert_eq!(refusal.code(), SafCode::Params);
    }

    #[test]
    fn only_the_websocket_verb_opens_a_channel() {
        let refusal = LaunchRequest::parse("afirma://sign?ports=49152&v=4&idsession=abc")
            .expect_err("la invocacion de arranque es 'websocket'");

        assert_eq!(refusal.code(), SafCode::Params);
    }
}
