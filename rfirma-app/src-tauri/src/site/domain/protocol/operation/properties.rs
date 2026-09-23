//! Lectura de parámetros y del `properties` que manda la sede.

use super::super::codes::Parameter;
use super::super::refusal::Refusal;
use super::super::url::{decode_protocol_base64, AfirmaUrl};

/// `properties`: extensiones admitidas por el diálogo de guardado de `signandsave`.
pub(super) const FILENAME_SAVE_EXTS: &str = "filenameSaveExts";

/// `properties`: descripción del filtro de extensiones del diálogo de guardado.
pub(super) const FILENAME_SAVE_DESCRIPTION: &str = "filenameSaveDescription";

/// `properties`: carpeta inicial sugerida al diálogo de guardado.
pub(super) const FILENAME_SAVE_CURRENT_DIR: &str = "filenameSaveCurrentDir";

/// `properties`: extensiones admitidas por el selector que elige el documento a firmar
/// (`AfirmaExtraParams.LOAD_FILE_EXTS`).
pub(super) const FILENAME_EXTS: &str = "filenameExts";

/// `properties`: descripción del filtro de extensiones del selector (`LOAD_FILE_DESCRIPTION`).
pub(super) const FILENAME_DESCRIPTION: &str = "filenameDescription";

/// `properties`: carpeta inicial sugerida al selector (`LOAD_FILE_CURRENT_DIR`).
pub(super) const FILENAME_CURRENT_DIR: &str = "filenameCurrentDir";

/// `properties`: el nombre que la sede propone para el fichero que se va a elegir.
const FILENAME_ACTUAL_NAME: &str = "filenameActualName";

/// `properties`: la sede se conforma con el único certificado que pase el filtro.
const HEADLESS: &str = "headless";

/// `properties`: puesto a `false` dice lo mismo que `headless=true`
/// (`CertFilterManager.isMandatoryCertificate`, 1.9.2).
const MANDATORY_CERT_SELECTION: &str = "mandatoryCertSelection";

/// `properties`: el perfil *baseline*, que el original borra antes de firmar.
const PROFILE: &str = "profile";

/// Las claves de `properties` que el lanzador interpreta él mismo y nunca entrega al firmador
/// (`ProtocolInvocationLauncherSign.java:153`, `CertFilterManager.java:145`, 1.9.2).
const INTERPRETED_BY_THE_LAUNCHER: [&str; 4] = [
    HEADLESS,
    MANDATORY_CERT_SELECTION,
    PROFILE,
    FILENAME_ACTUAL_NAME,
];

/// Un parámetro opcional, o nada si no vino o vino vacío.
pub(super) fn optional(url: &AfirmaUrl, name: &str) -> Option<String> {
    url.parameter(name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Una lista separada por comas, o vacía si el parámetro no vino.
pub(super) fn comma_list(url: &AfirmaUrl, name: &str) -> Vec<String> {
    comma_list_value(url.parameter(name).map(str::to_owned))
}

/// Una lista separada por comas a partir de un valor ya leído, o vacía si no vino.
pub(super) fn comma_list_value(value: Option<String>) -> Vec<String> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|piece| !piece.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// El valor de una clave del `properties` ya decodificado, o nada si no vino o vino vacío.
pub(super) fn property_value(declared: &[(String, String)], key: &str) -> Option<String> {
    declared
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
        .filter(|value| !value.is_empty())
}

/// Un parámetro que la operación exige, o el `SAF_03` que lo nombra.
pub(super) fn required<'u>(
    url: &'u AfirmaUrl,
    name: &str,
    blame: Parameter,
) -> Result<&'u str, Refusal> {
    url.parameter(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| Refusal::about(blame, format!("falta el parametro '{name}'")))
}

/// El verbo que pide la sede: el parámetro `op` si viene, y si no, el dominio
/// de la URL.
pub(super) fn verb_of(url: &AfirmaUrl) -> String {
    url.parameter("op")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| url.verb())
        .to_owned()
}

/// El `properties` que mandó la sede, partido en lo que cruza al firmador y lo que el
/// lanzador interpreta él mismo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DeclaredProperties {
    crossing: Vec<(String, String)>,
    headless: bool,
    actual_name: Option<String>,
}

impl DeclaredProperties {
    /// Los pares que sí se le entregan al firmador.
    pub fn crossing(&self) -> &[(String, String)] {
        &self.crossing
    }

    /// Si la sede se conforma con el único certificado que pase el filtro.
    pub fn is_headless(&self) -> bool {
        self.headless
    }

    /// El nombre que la sede propone al selector de documento.
    pub fn actual_name(&self) -> Option<&str> {
        self.actual_name.as_deref()
    }
}

/// Los pares del `.properties` que la sede mandó dentro de `properties`.
///
/// Viaja en Base64 **URL-safe** (`Base64.encode(bytes, true)` del original), y
/// el descodificador es tolerante a propósito con lo que sí puede llegar: la
/// `/` del alfabeto normal y el relleno ausente.
///
/// Un valor que aun así no se pueda leer **no tumba la operación**: se descarta
/// con traza y el trámite sigue sin parámetros adicionales, que es lo que hace
/// `UrlParametersToSign.setSignParameters` (`UrlParametersToSign.java:207`, 1.9.2).
/// El precio de rechazarlo sería una firma que habría salido.
pub(super) fn declared_properties(url: &AfirmaUrl) -> DeclaredProperties {
    let all = readable_properties(url);
    DeclaredProperties {
        headless: asks_to_skip_the_dialog(&all),
        actual_name: property_value(&all, FILENAME_ACTUAL_NAME),
        crossing: without_the_launcher_keys(all),
    }
}

/// Los pares que se hayan podido leer de `properties`, vacío si no se pudo leer ninguno.
fn readable_properties(url: &AfirmaUrl) -> Vec<(String, String)> {
    let Some(encoded) = url.parameter("properties").filter(|it| !it.is_empty()) else {
        return Vec::new();
    };

    let Ok(decoded) = decode_base64(encoded, Parameter::Properties) else {
        return discarded(encoded.len(), "no es Base64");
    };

    match String::from_utf8(decoded) {
        Ok(text) => pairs_of(&text),
        Err(_) => discarded(encoded.len(), "no es texto UTF-8"),
    }
}

fn discarded(length: usize, reason: &str) -> Vec<(String, String)> {
    eprintln!("rfirma: se descarta 'properties' ({length} caracteres): {reason}");
    Vec::new()
}

/// `headless=true`, o su sinónimo `mandatoryCertSelection=false`
/// (`CertFilterManager.isMandatoryCertificate`, 1.9.2).
fn asks_to_skip_the_dialog(declared: &[(String, String)]) -> bool {
    let headless = property_value(declared, HEADLESS)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    let mandatory = property_value(declared, MANDATORY_CERT_SELECTION)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("false"));
    headless || mandatory
}

/// Los pares sin las cuatro claves que el lanzador interpreta él mismo, compartido con los
/// `extraparams` de un elemento de lote.
pub fn without_the_launcher_keys(declared: Vec<(String, String)>) -> Vec<(String, String)> {
    declared
        .into_iter()
        .filter(|(key, _)| {
            !INTERPRETED_BY_THE_LAUNCHER
                .iter()
                .any(|interpreted| key.eq_ignore_ascii_case(interpreted))
        })
        .collect()
}

/// El Base64 del protocolo, con el `SAF_15` que nombra al parámetro que no lo era.
fn decode_base64(encoded: &str, blame: Parameter) -> Result<Vec<u8>, Refusal> {
    decode_protocol_base64(encoded).map_err(|error| {
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
