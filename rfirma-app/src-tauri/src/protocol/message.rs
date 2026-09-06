//! Lo que llega por el canal ya abierto, leído como texto y sin efectos.

use crate::protocol::AfirmaUrl;

/// El prefijo que marca un eco.
pub const ECHO_PREFIX: &str = "echo=";

/// El sufijo que cierra el eco.
pub const ECHO_SUFFIX: &str = "@EOF";

const CREDENTIAL_PARAMETER: &str = "idsession=";

/// Un mensaje del canal, ya leído.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelMessage {
    /// El eco del cliente publicado.
    Echo {
        /// La credencial que trae, si trae alguna.
        credential: Option<String>,
    },
    /// Una operación con su verbo y sus parámetros.
    Operation {
        /// La URL ya partida.
        url: AfirmaUrl,
    },
    /// Ni un eco ni una URL del protocolo.
    NotOfTheProtocol,
}

impl ChannelMessage {
    /// Lee el texto que llegó por el canal.
    pub fn read(text: &str) -> Self {
        let text = text.trim();

        if let Some(parameters) = text.strip_prefix(ECHO_PREFIX) {
            return Self::Echo {
                credential: credential_in(parameters),
            };
        }

        match AfirmaUrl::parse(text) {
            Ok(url) => Self::Operation { url },
            Err(_) => Self::NotOfTheProtocol,
        }
    }

    /// La credencial de canal que el mensaje repite, si la repite.
    pub fn credential(&self) -> Option<&str> {
        match self {
            Self::Echo { credential } => credential.as_deref(),
            Self::Operation { url } => url.parameter("idsession").map(strip_echo_suffix),
            Self::NotOfTheProtocol => None,
        }
    }
}

/// La credencial dentro de la cola de un eco: `-idsession=<valor>@EOF`.
fn credential_in(parameters: &str) -> Option<String> {
    let start = parameters.find(CREDENTIAL_PARAMETER)? + CREDENTIAL_PARAMETER.len();
    let value = &parameters[start..];
    let value = value.split(['&', '\n']).next().unwrap_or_default();

    Some(strip_echo_suffix(value).to_owned())
}

/// Quita el `@EOF` final.
fn strip_echo_suffix(value: &str) -> &str {
    value.strip_suffix(ECHO_SUFFIX).unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El eco tal y como lo manda el `autoscript.js` publicado
    /// (`autoscript.js:2286`).
    const PUBLISHED_ECHO: &str = "echo=-idsession=8jAkPZfRw2mQxN4TbYuL@EOF";

    #[test]
    fn the_echo_the_published_client_sends_is_read_whole() {
        let message = ChannelMessage::read(PUBLISHED_ECHO);

        assert_eq!(
            message,
            ChannelMessage::Echo {
                credential: Some("8jAkPZfRw2mQxN4TbYuL".to_owned()),
            }
        );
        assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
    }

    #[test]
    fn an_echo_without_the_end_marker_is_still_an_echo() {
        let message = ChannelMessage::read("echo=-idsession=8jAkPZfRw2mQxN4TbYuL");

        assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
    }

    #[test]
    fn an_echo_that_forgot_the_credential_carries_none() {
        let message = ChannelMessage::read("echo=@EOF");

        assert_eq!(message, ChannelMessage::Echo { credential: None });
        assert_eq!(message.credential(), None);
    }

    #[test]
    fn an_operation_repeats_the_credential_in_its_own_parameter() {
        let message = ChannelMessage::read(
            "afirma://sign?op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES",
        );

        assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
        let ChannelMessage::Operation { url } = &message else {
            panic!("una URL del protocolo es una operacion");
        };
        assert_eq!(url.verb(), "sign");
    }

    #[test]
    fn a_credential_that_ends_in_the_marker_is_trimmed_like_in_the_original() {
        let message = ChannelMessage::read("afirma://sign?idsession=8jAkPZfRw2mQxN4TbYuL@EOF");

        assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
    }

    #[test]
    fn anything_that_is_neither_an_echo_nor_a_protocol_url_is_not_of_the_protocol() {
        assert_eq!(
            ChannelMessage::read("GET / HTTP/1.1"),
            ChannelMessage::NotOfTheProtocol
        );
        assert_eq!(
            ChannelMessage::read("https://sede.example/firmar"),
            ChannelMessage::NotOfTheProtocol
        );
        assert_eq!(ChannelMessage::read("").credential(), None);
    }

    #[test]
    fn surrounding_whitespace_does_not_hide_the_echo() {
        let message = ChannelMessage::read("  echo=-idsession=8jAkPZfRw2mQxN4TbYuL@EOF\n");

        assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
    }
}
