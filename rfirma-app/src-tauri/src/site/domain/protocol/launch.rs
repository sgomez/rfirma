//! La invocación de arranque: verbo, versión de protocolo, ubicación de canal y credencial.

use crate::site::domain::channel::ChannelLocation;

use super::cipher::CipherKey;
use super::codes::{Parameter, SafCode};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

/// El verbo de la invocación de arranque, y el único que abre canal.
pub const LAUNCH_VERB: &str = "websocket";

/// La versión de protocolo que sortea puertos, la que manda el cliente publicado.
pub const PROTOCOL_VERSION: i64 = 4;

/// La versión de protocolo sin `ports`, atendida en el puerto fijo.
pub const THIRD_PROTOCOL_VERSION: i64 = 3;

const VERSION_WHEN_ABSENT: i64 = 1;

/// Puerto fijo del protocolo 3, nunca atado cuando la sede sorteó puertos (ADR-0005).
pub const THE_PORT_OF_THE_THIRD_PROTOCOL: u16 = 63117;

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

/// La credencial negociada para el canal: exigida, o ausente si la sede no la trajo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NegotiatedCredential {
    /// El canal exige esta credencial en cada mensaje.
    Required(ChannelCredential),
    /// El canal no exige ninguna credencial.
    Absent,
}

/// Información de canal del servidor intermedio, negociada desde la propia invocación de
/// arranque: no hay canal que sostener, así que la operación viaja con sus servlets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayChannelInfo {
    /// El verbo de operación y sus parámetros, tal como llegaron (`dat` puede faltar).
    pub operation: AfirmaUrl,
    /// Servlet de recuperación (`rtservlet`), si la operación llega por `fileid`.
    pub retrieve_servlet: Option<String>,
    /// Servlet de almacenamiento (`stservlet`), donde se sube la respuesta.
    pub store_servlet: String,
    /// Identificador (`id`) con el que se sube la respuesta y, si aplica, se recupera la petición.
    pub id: String,
    /// La referencia (`fileid`) al contenido a recuperar, cuando la operación no lo trae inline.
    pub fileid: Option<String>,
    /// La clave de cifrado (`key`), si la operación la trae.
    pub key: Option<CipherKey>,
    /// Si la sede pide espera activa (`aw`) antes de operar.
    pub active_wait: bool,
}

/// Lo que pide una invocación de arranque, ya leída.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchRequest {
    version: i64,
    location: ChannelLocation,
    credential: NegotiatedCredential,
}

impl LaunchRequest {
    /// Lee la invocación de arranque, o dice con qué `SAF_` se rechaza.
    pub fn parse(url: &str) -> Result<Self, Refusal> {
        Self::from_url(&AfirmaUrl::parse(url)?)
    }

    /// Lo mismo, sobre una URL ya partida.
    pub fn from_url(url: &AfirmaUrl) -> Result<Self, Refusal> {
        if url.verb() == LAUNCH_VERB {
            return Self::from_websocket_url(url);
        }
        if is_a_relay_launch(url) {
            return Self::from_relay_url(url);
        }

        Err(Refusal::params(format!(
            "la invocacion de arranque es 'afirma://{LAUNCH_VERB}' o una operacion con servlet \
             de servidor intermedio, y esta es 'afirma://{}'",
            url.verb()
        )))
    }

    fn from_websocket_url(url: &AfirmaUrl) -> Result<Self, Refusal> {
        let version = check_protocol_version(url.parameter("v"))?;
        let location = location_of(version, url.parameter("ports"))?;
        let credential = credential_of(version, url.parameter("idsession"))?;

        Ok(Self {
            version,
            location,
            credential,
        })
    }

    fn from_relay_url(url: &AfirmaUrl) -> Result<Self, Refusal> {
        let store_servlet = url
            .parameter("stservlet")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                Refusal::params(
                "la operacion con servidor intermedio no trae 'stservlet', y sin el no se puede \
                 subir la respuesta",
            )
            })?
            .to_owned();
        let id = url
            .parameter("id")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Refusal::params("la operacion con servidor intermedio no trae 'id'"))?
            .to_owned();
        let fileid = url
            .parameter("fileid")
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let retrieve_servlet = url
            .parameter("rtservlet")
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        if url.parameter("dat").is_none() && fileid.is_none() {
            return Err(Refusal::params(
                "la operacion con servidor intermedio no trae ni 'dat' ni 'fileid': no hay datos \
                 que operar",
            ));
        }
        if fileid.is_some() && url.parameter("dat").is_none() && retrieve_servlet.is_none() {
            return Err(Refusal::params(
                "la operacion trae 'fileid' pero no 'rtservlet', y sin el no se puede recuperar \
                 el contenido",
            ));
        }

        let key = match url.parameter("key").filter(|value| !value.is_empty()) {
            Some(value) => CipherKey::from_url_parameter(value)
                .map_err(|error| Refusal::params(error.detail().to_owned()))?,
            None => None,
        };

        let active_wait = url
            .parameter("aw")
            .is_some_and(|value| !value.is_empty() && value != "false");

        Ok(Self {
            version: PROTOCOL_VERSION,
            location: ChannelLocation::Relay(RelayChannelInfo {
                operation: url.clone(),
                retrieve_servlet,
                store_servlet,
                id,
                fileid,
                key,
                active_wait,
            }),
            credential: NegotiatedCredential::Absent,
        })
    }

    /// La versión de protocolo que declaró la sede, ya validada.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Dónde escuchará el canal: los puertos sorteados por la sede, o el puerto fijo del
    /// protocolo 3.
    pub fn location(&self) -> &ChannelLocation {
        &self.location
    }

    /// La credencial que cerrará el canal, si la sede la exige.
    pub fn credential(&self) -> &NegotiatedCredential {
        &self.credential
    }
}

/// Los puertos que la sede sorteó, se acepte la invocación o no.
pub fn drawn_ports(url: &AfirmaUrl) -> Vec<u16> {
    parse_ports(url.parameter("ports")).unwrap_or_default()
}

/// Dónde contestaría un rechazo a esta URL, si se puede determinar sin conocer si la invocación
/// entera vale: por los puertos que trajo, o por el puerto fijo si declaró la versión 3.
pub fn location_for_a_refusal(url: &AfirmaUrl) -> Option<ChannelLocation> {
    let ports = drawn_ports(url);
    if !ports.is_empty() {
        return Some(ChannelLocation::Drawn(ports));
    }

    if declared_version(url.parameter("v")) == THIRD_PROTOCOL_VERSION {
        return Some(ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL));
    }

    None
}

/// Si la invocación tiene la forma de una operación con servidor intermedio: un verbo de
/// operación (no `websocket`) que trae al menos uno de los dos servlets.
fn is_a_relay_launch(url: &AfirmaUrl) -> bool {
    url.parameter("rtservlet").is_some() || url.parameter("stservlet").is_some()
}

fn location_of(version: i64, ports: Option<&str>) -> Result<ChannelLocation, Refusal> {
    if version == THIRD_PROTOCOL_VERSION {
        return Ok(ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL));
    }

    Ok(ChannelLocation::Drawn(parse_ports(ports)?))
}

fn credential_of(version: i64, idsession: Option<&str>) -> Result<NegotiatedCredential, Refusal> {
    match idsession.filter(|value| !value.is_empty()) {
        Some(value) => ChannelCredential::parse(value).map(NegotiatedCredential::Required),
        None if version == THIRD_PROTOCOL_VERSION => Ok(NegotiatedCredential::Absent),
        None => ChannelCredential::parse("").map(NegotiatedCredential::Required),
    }
}

/// La versión que la sede declaró en `v`, o la que se asume cuando no la trae.
fn declared_version(declared: Option<&str>) -> i64 {
    declared
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(VERSION_WHEN_ABSENT)
}

fn check_protocol_version(declared: Option<&str>) -> Result<i64, Refusal> {
    let version = declared_version(declared);

    if version == PROTOCOL_VERSION || version == THIRD_PROTOCOL_VERSION {
        return Ok(version);
    }

    Err(Refusal::new(
        SafCode::UnsupportedProcedure,
        format!("la sede declara la version de protocolo {version} y aqui se hablan la {THIRD_PROTOCOL_VERSION} y la {PROTOCOL_VERSION}"),
    )
    .because(RefusalSituation::UnsupportedProtocolVersion))
}

fn parse_ports(declared: Option<&str>) -> Result<Vec<u16>, Refusal> {
    let Some(declared) = declared.filter(|value| !value.is_empty()) else {
        return Err(Refusal::about(
            Parameter::Ports,
            "la invocacion no trae puertos ('ports'), y el camino sin puertos es el del \
             protocolo 3",
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
