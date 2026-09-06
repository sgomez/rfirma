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
mod tests;
