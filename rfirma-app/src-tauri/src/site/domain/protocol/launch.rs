//! La invocación de arranque: verbo, versión de protocolo, ubicación de canal y credencial.

use crate::site::domain::channel::ChannelLocation;

use super::cipher::CipherKey;
use super::codes::{Parameter, SafCode};
use super::parameters::{check_servlet_url, reads_as_true};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

/// El verbo de la invocación de arranque, y el único que abre canal.
pub const LAUNCH_VERB: &str = "websocket";

/// El verbo de la invocación de arranque sin WebSocket, sobre TLS crudo.
pub const SERVICE_VERB: &str = "service";

/// La versión de protocolo que sortea puertos, la que manda el cliente publicado.
pub const PROTOCOL_VERSION: i64 = 4;

/// La versión de protocolo sin `ports`, atendida en el puerto fijo.
pub const THIRD_PROTOCOL_VERSION: i64 = 3;

const VERSION_WHEN_ABSENT: i64 = 1;

/// El original lo exige porque el identificador acaba siendo un nombre de fichero.
const LONGEST_IDENTIFIER: usize = 20;

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

/// De dónde salen la operación y el destino de la respuesta en un arranque de servidor intermedio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayRequest {
    /// La URL trae la operación entera, `dat` incluido.
    Inline {
        /// Servlet de almacenamiento (`stservlet`), donde se sube la respuesta.
        store_servlet: String,
        /// Identificador (`id`) con el que se sube la respuesta.
        id: String,
    },
    /// La URL trae la operación, y por `fileid` se recupera el documento que va en `dat`.
    DataByFileId {
        /// Servlet de almacenamiento (`stservlet`), donde se sube la respuesta.
        store_servlet: String,
        /// Identificador (`id`) con el que se sube la respuesta.
        id: String,
        /// La referencia (`fileid`) al documento a recuperar.
        fileid: String,
        /// Servlet de recuperación (`rtservlet`) del que se baja el documento.
        retrieve_servlet: String,
    },
    /// La URL solo trae `fileid`: lo recuperado es el XML de parámetros, de donde salen la
    /// operación entera, `stservlet` e `id`.
    ParametersByFileId {
        /// La referencia (`fileid`) al XML de parámetros a recuperar.
        fileid: String,
        /// Servlet de recuperación (`rtservlet`) del que se baja el XML de parámetros.
        retrieve_servlet: String,
    },
}

impl RelayRequest {
    /// Dónde y con qué identificador se sube la respuesta, cuando la URL ya lo dijo.
    pub fn store_target(&self) -> Option<(&str, &str)> {
        match self {
            Self::Inline { store_servlet, id }
            | Self::DataByFileId {
                store_servlet, id, ..
            } => Some((store_servlet, id)),
            Self::ParametersByFileId { .. } => None,
        }
    }
}

/// Información de canal del servidor intermedio, negociada desde la propia invocación de
/// arranque: no hay canal que sostener, así que la operación viaja con sus servlets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayChannelInfo {
    /// El verbo de operación y sus parámetros, tal como llegaron (`dat` puede faltar).
    pub operation: AfirmaUrl,
    /// De dónde salen la operación y el destino de la respuesta.
    pub request: RelayRequest,
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
        if url.verb() == SERVICE_VERB {
            return Self::from_service_url(url);
        }
        if is_a_relay_launch(url) {
            return Self::from_relay_url(url);
        }

        Err(Refusal::params(format!(
            "la invocacion de arranque es 'afirma://{LAUNCH_VERB}', 'afirma://{SERVICE_VERB}' o \
             una operacion con servlet de servidor intermedio, y esta es 'afirma://{}'",
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

    fn from_service_url(url: &AfirmaUrl) -> Result<Self, Refusal> {
        let version = check_service_version(url.parameter("v"))?;
        let ports = parse_ports(url.parameter("ports"))?;
        let credential = service_credential_of(url.parameter("idsession"))?;

        Ok(Self {
            version,
            location: ChannelLocation::Service(ports),
            credential,
        })
    }

    fn from_relay_url(url: &AfirmaUrl) -> Result<Self, Refusal> {
        let request = relay_request_of(url)?;

        let key = match url.parameter("key").filter(|value| !value.is_empty()) {
            Some(value) => CipherKey::from_url_parameter(value)
                .map_err(|error| Refusal::about(Parameter::CipherKey, error.detail().to_owned()))?,
            None => None,
        };

        Ok(Self {
            version: PROTOCOL_VERSION,
            location: ChannelLocation::Relay(RelayChannelInfo {
                operation: url.clone(),
                request,
                key,
                active_wait: asks_for_active_wait(url),
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

    if url.verb() == SERVICE_VERB {
        return (!ports.is_empty()).then(|| ChannelLocation::Service(ports));
    }

    if !ports.is_empty() {
        return Some(ChannelLocation::Drawn(ports));
    }

    if declared_version(url.parameter("v")) == THIRD_PROTOCOL_VERSION {
        return Some(ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL));
    }

    None
}

/// Si la invocación tiene la forma de una operación con servidor intermedio: un verbo de
/// operación (no `websocket`) que trae `stservlet`, o que trae `fileid` y `rtservlet` y por tanto
/// recupera su XML de parámetros.
fn is_a_relay_launch(url: &AfirmaUrl) -> bool {
    url.parameter("stservlet").is_some()
        || (url.parameter("fileid").is_some() && url.parameter("rtservlet").is_some())
}

/// Si la sede pide espera activa (`aw`) antes de operar.
pub fn asks_for_active_wait(url: &AfirmaUrl) -> bool {
    url.parameter("aw").is_some_and(reads_as_true)
}

/// El identificador de sesión del servidor intermedio, que el original usa como nombre de fichero.
fn checked_identifier(value: String, blame: Parameter) -> Result<String, Refusal> {
    if value.chars().count() > LONGEST_IDENTIFIER {
        return Err(Refusal::about(
            blame,
            format!("el identificador '{value}' pasa de {LONGEST_IDENTIFIER} caracteres"),
        ));
    }
    if !value.chars().all(|it| it.is_ascii_alphanumeric()) {
        return Err(Refusal::about(
            blame,
            format!("el identificador '{value}' tiene caracteres que no son letras ni digitos"),
        ));
    }

    Ok(value)
}

fn checked_servlet(value: String, blame: Parameter) -> Result<String, Refusal> {
    check_servlet_url(&value, blame)?;
    Ok(value)
}

fn relay_request_of(url: &AfirmaUrl) -> Result<RelayRequest, Refusal> {
    let store_servlet = given(url, "stservlet")
        .map(|it| checked_servlet(it, Parameter::StoreServlet))
        .transpose()?;
    let fileid = given(url, "fileid")
        .map(|it| checked_identifier(it, Parameter::FileId))
        .transpose()?;
    let retrieve_servlet = given(url, "rtservlet")
        .map(|it| checked_servlet(it, Parameter::RetrieveServlet))
        .transpose()?;

    let Some(store_servlet) = store_servlet else {
        return match (fileid, retrieve_servlet) {
            (Some(fileid), Some(retrieve_servlet)) => Ok(RelayRequest::ParametersByFileId {
                fileid,
                retrieve_servlet,
            }),
            _ => Err(Refusal::params(
                "la operacion con servidor intermedio no trae 'stservlet' ni 'fileid' con \
                 'rtservlet', y sin eso no se puede subir la respuesta",
            )),
        };
    };

    let id = given(url, "id")
        .ok_or_else(|| Refusal::params("la operacion con servidor intermedio no trae 'id'"))
        .and_then(|it| checked_identifier(it, Parameter::Identifier))?;

    if url.parameter("dat").is_some() {
        return Ok(RelayRequest::Inline { store_servlet, id });
    }

    let Some(fileid) = fileid else {
        return Err(Refusal::params(
            "la operacion con servidor intermedio no trae ni 'dat' ni 'fileid': no hay datos \
             que operar",
        ));
    };
    let Some(retrieve_servlet) = retrieve_servlet else {
        return Err(Refusal::params(
            "la operacion trae 'fileid' pero no 'rtservlet', y sin el no se puede recuperar \
             el contenido",
        ));
    };

    Ok(RelayRequest::DataByFileId {
        store_servlet,
        id,
        fileid,
        retrieve_servlet,
    })
}

/// El valor de un parámetro que vino y no vino vacío.
fn given(url: &AfirmaUrl, name: &str) -> Option<String> {
    url.parameter(name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
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

/// La credencial de `service`: exigida cuando la sede la manda, ausente si no, para cualquiera
/// de sus tres versiones (mismo estado del dominio que la v3 de `websocket`).
fn service_credential_of(idsession: Option<&str>) -> Result<NegotiatedCredential, Refusal> {
    match idsession.filter(|value| !value.is_empty()) {
        Some(value) => ChannelCredential::parse(value).map(NegotiatedCredential::Required),
        None => Ok(NegotiatedCredential::Absent),
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

/// Las versiones de `service` que aquí se hablan son la 1, la 2 y la `THIRD_PROTOCOL_VERSION`
/// (`ServiceInvocationManager.java:42-45`).
fn check_service_version(declared: Option<&str>) -> Result<i64, Refusal> {
    let version = declared_version(declared);

    if (1..=THIRD_PROTOCOL_VERSION).contains(&version) {
        return Ok(version);
    }

    Err(Refusal::new(
        SafCode::UnsupportedProcedure,
        format!(
            "la sede declara la version de protocolo {version} para '{SERVICE_VERB}' y aqui se \
             hablan la 1, la 2 y la {THIRD_PROTOCOL_VERSION}"
        ),
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
