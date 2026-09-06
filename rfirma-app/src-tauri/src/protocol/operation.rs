//! Lo que la sede pide por el canal ya abierto, leído de la URL.

use base64::Engine as _;

use super::codes::{Parameter, SafCode};
use super::filters::{site_filter, SiteFilter};
use super::parameters::{check_local_access_is_not_requested, check_minimum_client_version};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

/// El verbo de la selección de certificado, tal y como viaja por el cable.
///
/// **No es el nombre que el JS usa por dentro**: allí la constante se llama
/// `OPERATION_SELECT_CERTIFICATE = "certificate"` (`autoscript.js:1761`), que
/// es sólo la etiqueta con la que el cliente recuerda qué respuesta espera. Lo
/// que viaja es esto (`autoscript.js:1943`).
pub const SELECT_CERTIFICATE: &str = "selectcert";

/// El verbo de la firma (`autoscript.js:1828`).
pub const SIGN: &str = "sign";

/// El verbo de la cofirma.
pub const COSIGN: &str = "cosign";

/// El verbo de la contrafirma.
pub const COUNTERSIGN: &str = "countersign";

/// El verbo que guarda un fichero en el equipo.
pub const SAVE: &str = "save";

/// El verbo que firma y además guarda.
pub const SIGN_AND_SAVE: &str = "signandsave";

/// El formato de firma PAdES.
pub const PADES: &str = "pades";

/// El algoritmo que rFirma sabe producir.
pub const ACCEPTED_ALGORITHMS: [&str; 2] = ["sha256", "sha256withrsa"];

/// Lo que la sede pide, ya leído.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteOperation {
    /// `selectcert`: la sede pide identidad.
    SelectCertificate(SelectCertificate),
    /// `sign` o `cosign` sobre un PDF: la sede pide una firma.
    Sign(SignRequest),
}

/// Cuál de las dos firmas pidió la sede.
///
/// En PAdES las dos recorren el mismo camino —cofirmar es volver a firmar—, y
/// la distinción se guarda porque es lo que la sede pidió y lo que la ventana
/// tiene que contarle a la persona antes de que consienta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureRound {
    /// `sign`: se firma lo que llega.
    First,
    /// `cosign`: se vuelve a firmar sobre las firmas que el PDF ya trae.
    Again,
}

/// La petición de `sign` o de `cosign`.
///
/// Lleva **el documento ya descodificado**, y no el Base64: lo que se firma son
/// bytes, y dejar el Base64 vivo hasta el momento de firmar es tener dos
/// copias de lo mismo y una ocasión de descodificarlo dos veces distintas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRequest {
    round: SignatureRound,
    algorithm: String,
    document: Vec<u8>,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
}

impl SignRequest {
    /// `sign` o `cosign`.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// El algoritmo tal y como lo pidió la sede, ya admitido.
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// El documento que la sede manda, en bytes.
    pub fn document(&self) -> &[u8] {
        &self.document
    }

    /// Los `extraParams` tal y como vinieron, sin expandir.
    pub fn declared_params(&self) -> &[(String, String)] {
        &self.declared
    }

    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }
}

/// La petición de `selectcert`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectCertificate {
    filter: SiteFilter,
}

impl SelectCertificate {
    /// Lo que la sede pide del listado.
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
        SIGN => sign_request(url, SignatureRound::First),
        COSIGN => sign_request(url, SignatureRound::Again),
        COUNTERSIGN => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            "'countersign' no existe en PAdES: AOPDFSigner.countersign lanza una \
             UnsupportedOperationException",
        )),
        SAVE | SIGN_AND_SAVE => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            "rFirma no guarda ficheros por orden de una sede",
        )),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("la operacion '{other}' no se atiende"),
        )),
    }
}

/// La petición de firma, con las cuatro comprobaciones de
/// `UrlParametersToSign` que rFirma hereda.
///
/// El orden importa poco salvo en una cosa: el **formato** se mira antes que
/// nada de lo demás, porque una sede que pide XAdES no se merece un `SAF_03`
/// sobre el algoritmo cuando lo que pasa es que ese formato no se atiende.
fn sign_request(url: &AfirmaUrl, round: SignatureRound) -> Result<SiteOperation, Refusal> {
    let format = required(url, "format", Parameter::Format)
        .map_err(|refusal| refusal.because(RefusalSituation::MissingFormat))?;
    if !format.trim().eq_ignore_ascii_case(PADES) {
        return Err(Refusal::new(
            SafCode::UnsupportedFormat,
            format!("el formato '{format}' no se atiende: rFirma solo firma PAdES"),
        ));
    }

    let algorithm = required(url, "algorithm", Parameter::Algorithm)?;
    if !ACCEPTED_ALGORITHMS.contains(&algorithm.trim().to_ascii_lowercase().as_str()) {
        return Err(Refusal::about(
            Parameter::Algorithm,
            format!("el algoritmo '{algorithm}' no se atiende: rFirma firma con SHA256withRSA"),
        ));
    }

    let data = required(url, "dat", Parameter::Data)?;
    let document = decode_base64(data, Parameter::Data)?;
    if document.is_empty() {
        return Err(Refusal::new(
            SafCode::SignWithoutData,
            "el parametro 'dat' viene vacio: no hay nada que firmar",
        ));
    }

    let declared = declared_properties(url)?;
    Ok(SiteOperation::Sign(SignRequest {
        round,
        algorithm: algorithm.trim().to_owned(),
        document,
        filter: site_filter(&declared)?,
        declared,
    }))
}

/// Un parámetro que la operación exige, o el `SAF_03` que lo nombra.
fn required<'u>(url: &'u AfirmaUrl, name: &str, blame: Parameter) -> Result<&'u str, Refusal> {
    url.parameter(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| Refusal::about(blame, format!("falta el parametro '{name}'")))
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
/// el descodificador es tolerante a propósito con lo que sí puede llegar: la
/// `/` del alfabeto normal y el relleno ausente. Un cliente que mande `/` no
/// está atacando nada, y rechazarle la llamada entera por eso sería inventarse
/// una incompatibilidad que el original no tiene.
///
/// El `+` del alfabeto normal, en cambio, **nunca llega hasta aquí**:
/// [`AfirmaUrl`] ya lo ha convertido en un espacio, porque el original pasa
/// cada valor por `URLDecoder` (ver el encabezado de [`crate::protocol::url`]).
/// Así que una sede que mande Base64 estándar con `+` se lleva el `SAF_03` —
/// igual que en el original, que decodifica igual—, y aquí no hay ningún brazo
/// que lo intente: sería código muerto que promete una tolerancia que no
/// existe.
fn declared_properties(url: &AfirmaUrl) -> Result<Vec<(String, String)>, Refusal> {
    let Some(encoded) = url.parameter("properties").filter(|it| !it.is_empty()) else {
        return Ok(Vec::new());
    };

    let decoded = decode_base64(encoded, Parameter::Properties)?;

    let text = String::from_utf8(decoded).map_err(|error| {
        Refusal::about(
            Parameter::Properties,
            format!("el parametro 'properties' no es texto: {error}"),
        )
    })?;

    Ok(pairs_of(&text))
}

/// El Base64 **URL-safe** del protocolo, con la misma tolerancia en todos los
/// parámetros que lo llevan.
///
/// Tolerante a propósito con lo que sí puede llegar: la `/` del alfabeto normal
/// y el relleno ausente o de más. El `+` del alfabeto normal, en cambio, nunca
/// llega hasta aquí: [`AfirmaUrl`] ya lo ha convertido en un espacio, porque el
/// original pasa cada valor por `URLDecoder`.
fn decode_base64(encoded: &str, blame: Parameter) -> Result<Vec<u8>, Refusal> {
    let normalized: String = encoded
        .chars()
        .filter(|character| *character != '=')
        .map(|character| if character == '/' { '_' } else { character })
        .collect();

    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(normalized.as_bytes())
        .map_err(|error| {
            Refusal::about(
                blame,
                format!("el parametro '{blame}' no es Base64: {error}"),
            )
        })
}

/// Los pares de un bloque `java.util.Properties`.
pub fn pairs_of(text: &str) -> Vec<(String, String)> {
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
mod tests;
