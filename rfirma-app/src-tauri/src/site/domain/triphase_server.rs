//! El protocolo del servidor trifásico CAdES que la sede nombra en `serverUrl`: qué se le pide y cómo se lee lo que contesta; no es el lote remoto (`AOCAdESTriPhaseSigner`, 1.9.2).

use std::collections::BTreeMap;
use std::fmt;

use base64::alphabet::URL_SAFE as URL_SAFE_ALPHABET;
use base64::engine::general_purpose::URL_SAFE;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine as _;

use super::batch::TriphaseData;
use super::protocol::{CounterTarget, SignatureRound};

/// La clave de `properties` con la dirección del servidor.
pub const SERVER_URL: &str = "serverUrl";
const DOCUMENT_ID: &str = "documentId";
const TARGET: &str = "target";
const FORMAT: &str = "CAdES";
const ERROR_PREFIX: &str = "ERR-";
const CONFIG_NEEDED_PREFIX: &str = "ERR-21:";
const SUCCESS: &str = "OK";
const NEW_ID: &str = "OK NEWID=";

const TOLERANT: GeneralPurpose = GeneralPurpose::new(
    &URL_SAFE_ALPHABET,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Por qué la firma contra el servidor trifásico no ha salido (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// La sede no declaró un `serverUrl` al que se pueda llamar.
    ServerUrlMissing,
    /// El servidor contestó con un error que trae una excepción suya.
    ServerException,
    /// El servidor no respondió.
    ServerUnreachable,
    /// El servidor respondió algo que no es lo que el protocolo espera.
    UnexpectedAnswer,
}

/// Un fallo de la firma trifásica contra el servidor, con su situación y su detalle crudo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriphaseServerError {
    situation: Situation,
    detail: String,
}

impl TriphaseServerError {
    /// Un fallo con su situación y su detalle.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// La situación clasificada.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// El detalle crudo.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for TriphaseServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for TriphaseServerError {}

/// Lo que viaja en las dos llamadas al servidor.
#[derive(Clone, Copy, Debug)]
pub struct ServerCall<'a> {
    /// La operación que pidió la sede.
    pub round: SignatureRound,
    /// El algoritmo ya compuesto con la clave del certificado.
    pub algorithm: &'a str,
    /// El certificado firmante en DER.
    pub certificate: &'a [u8],
    /// Los datos, o la firma previa en cofirma y contrafirma.
    pub document: &'a [u8],
    /// Los parámetros de la sede en formato `.properties`, si queda alguno.
    pub params: Option<&'a str>,
}

/// El `serverUrl` que declaró la sede, si es una URL HTTP a la que se pueda llamar.
pub fn server_url_of(
    from_the_site: &BTreeMap<String, String>,
) -> Result<&str, TriphaseServerError> {
    let declared = from_the_site.get(SERVER_URL).map(|url| url.trim());
    declared
        .filter(|url| is_http_with_a_host(url))
        .ok_or_else(|| {
            TriphaseServerError::new(
                Situation::ServerUrlMissing,
                format!(
                    "no se ha proporcionado una URL valida para el servidor de firma: {}",
                    declared.unwrap_or_default()
                ),
            )
        })
}

fn is_http_with_a_host(url: &str) -> bool {
    let lowered = url.to_ascii_lowercase();
    ["http://", "https://"].iter().any(|scheme| {
        lowered
            .strip_prefix(scheme)
            .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
    })
}

/// Los parámetros que recibe el servidor: los de la sede sin los que ya viajan aparte, y el objetivo de la contrafirma.
pub fn params_for_the_server(
    from_the_site: &BTreeMap<String, String>,
    round: SignatureRound,
) -> BTreeMap<String, String> {
    let mut params = from_the_site.clone();
    params.remove(SERVER_URL);
    params.remove(DOCUMENT_ID);
    if let SignatureRound::Counter { target } = round {
        let named = match target {
            CounterTarget::Tree => "tree",
            CounterTarget::Leafs => "leafs",
        };
        params.insert(TARGET.to_owned(), named.to_owned());
    }
    params
}

/// El formulario de la prefirma (`PreSigner.preSign`, 1.9.2).
pub fn presign_form(call: &ServerCall<'_>) -> Vec<(&'static str, String)> {
    let mut form = common_form("pre", call);
    form.push(("doc", URL_SAFE.encode(call.document)));
    form
}

/// El formulario de la postfirma, con la sesión que ya trae los `PK1` (`PostSigner.postSign`, 1.9.2).
pub fn postsign_form(call: &ServerCall<'_>, signed: &TriphaseData) -> Vec<(&'static str, String)> {
    let mut form = common_form("post", call);
    form.push(("session", URL_SAFE.encode(signed.to_xml().as_bytes())));
    form.push(("doc", URL_SAFE.encode(call.document)));
    form
}

fn common_form(op: &str, call: &ServerCall<'_>) -> Vec<(&'static str, String)> {
    let mut form = vec![
        ("op", op.to_owned()),
        ("cop", crypto_operation_of(call.round).to_owned()),
        ("format", FORMAT.to_owned()),
        ("algo", call.algorithm.to_owned()),
        ("cert", URL_SAFE.encode(call.certificate)),
    ];
    if let Some(params) = call.params {
        form.push(("params", URL_SAFE.encode(params.as_bytes())));
    }
    form
}

fn crypto_operation_of(round: SignatureRound) -> &'static str {
    match round {
        SignatureRound::First => "sign",
        SignatureRound::Again => "cosign",
        SignatureRound::Counter { .. } => "countersign",
    }
}

/// La sesión que devuelve la prefirma, o el error que contestó el servidor.
pub fn presigned(answer: &[u8]) -> Result<TriphaseData, TriphaseServerError> {
    let text = String::from_utf8_lossy(answer);
    if text.starts_with(ERROR_PREFIX) {
        return Err(error_of_the_server(&text));
    }
    let xml = decoded(&text)?;
    TriphaseData::parse_xml(&xml).map_err(|error| unexpected(error.to_string()))
}

/// La firma que devuelve la postfirma, o el error que contestó el servidor.
pub fn postsigned(answer: &[u8]) -> Result<Vec<u8>, TriphaseServerError> {
    let text = String::from_utf8_lossy(answer);
    if text.starts_with(CONFIG_NEEDED_PREFIX) {
        return Err(error_of_the_server(&text));
    }
    let trimmed = text.trim();
    if !trimmed.starts_with(SUCCESS) {
        return Err(unexpected(format!(
            "la firma trifasica no ha finalizado correctamente: {trimmed}"
        )));
    }
    decoded(trimmed.get(NEW_ID.len()..).unwrap_or_default())
}

fn error_of_the_server(message: &str) -> TriphaseServerError {
    let exception = message
        .split_once(':')
        .and_then(|(_, rest)| rest.split_once(':'))
        .map(|(_, exception)| exception.trim())
        .filter(|exception| !exception.is_empty());
    match exception {
        Some(exception) => TriphaseServerError::new(Situation::ServerException, exception),
        None => unexpected(message),
    }
}

fn decoded(text: &str) -> Result<Vec<u8>, TriphaseServerError> {
    let normalized: String = text
        .trim()
        .chars()
        .map(|character| match character {
            '+' => '-',
            '/' => '_',
            other => other,
        })
        .collect();
    TOLERANT
        .decode(normalized)
        .map_err(|error| unexpected(format!("la respuesta no es Base64: {error}")))
}

fn unexpected(detail: impl Into<String>) -> TriphaseServerError {
    TriphaseServerError::new(Situation::UnexpectedAnswer, detail)
}

#[cfg(test)]
mod tests;
