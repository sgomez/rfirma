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
mod tests;
