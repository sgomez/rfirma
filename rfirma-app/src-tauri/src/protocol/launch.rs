//! La invocación de arranque: puertos, versión de protocolo y credencial de canal.

use super::codes::{Parameter, SafCode};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

/// El verbo de la invocación de arranque, y el único que abre canal.
pub const LAUNCH_VERB: &str = "websocket";

/// La versión de protocolo que se habla, y la única que se acepta.
pub const PROTOCOL_VERSION: i64 = 4;

const VERSION_WHEN_ABSENT: i64 = 1;

/// La credencial del canal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelCredential(String);

impl ChannelCredential {
    /// La credencial, si el valor está bien formado.
    pub fn parse(value: &str) -> Result<Self, Refusal> {
        if value.is_empty() {
            return Err(Refusal::about(
                Parameter::IdSession,
                "la invocacion no trae credencial de canal ('idsession'), y sin ella el canal \
                 quedaria sin cerradura",
            ));
        }
        if !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(Refusal::about(
                Parameter::IdSession,
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

/// Los puertos que la sede sorteó, se acepte la invocación o no.
pub fn drawn_ports(url: &AfirmaUrl) -> Vec<u16> {
    parse_ports(url.parameter("ports")).unwrap_or_default()
}

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
    )
    .because(RefusalSituation::UnsupportedProtocolVersion))
}

fn parse_ports(declared: Option<&str>) -> Result<Vec<u16>, Refusal> {
    let Some(declared) = declared.filter(|value| !value.is_empty()) else {
        return Err(Refusal::about(
            Parameter::Ports,
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
                    Refusal::about(
                        Parameter::Ports,
                        format!("el parametro 'ports' trae un valor que no es un puerto: {port}"),
                    )
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
