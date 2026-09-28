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

/// `properties`: la sede se conforma con el único candidato y no quiere preguntas.
const HEADLESS: &str = "headless";

/// `properties`: puesto a `false`, la sede se conforma con el único candidato.
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
    unattended: Unattended,
    actual_name: Option<String>,
}

/// Lo que la sede declara que no hace falta preguntar a la persona.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Unattended {
    headless: bool,
    choice_waived: bool,
}

impl Unattended {
    /// `headless=true`: lo que haga falta preguntar se rechaza.
    pub fn is_headless(self) -> bool {
        self.headless
    }

    /// `headless=true` o `mandatoryCertSelection=false` (`CertFilterManager.isMandatoryCertificate`, 1.9.2).
    pub fn waives_the_choice(self) -> bool {
        self.choice_waived
    }
}

impl DeclaredProperties {
    /// Los pares que sí se le entregan al firmador.
    pub fn crossing(&self) -> &[(String, String)] {
        &self.crossing
    }

    /// Lo que la sede declara que no hace falta preguntar.
    pub fn unattended(&self) -> Unattended {
        self.unattended
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
        unattended: unattended_as_declared(&all),
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

fn unattended_as_declared(declared: &[(String, String)]) -> Unattended {
    let headless = property_value(declared, HEADLESS)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    let not_mandatory = property_value(declared, MANDATORY_CERT_SELECTION)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("false"));
    Unattended {
        headless,
        choice_waived: headless || not_mandatory,
    }
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

/// Los pares de un bloque `java.util.Properties`, leído como `Properties.load`; vacío si un `\u` está mal formado.
pub fn pairs_of(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for line in logical_lines(text) {
        let (raw_key, raw_value) = split_key_and_value(&line);
        let (Some(key), Some(value)) = (unescape(raw_key), unescape(raw_value)) else {
            return Vec::new();
        };
        if !key.is_empty() {
            pairs.push((key, value));
        }
    }

    pairs
}

fn is_properties_blank(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\u{c}')
}

/// Las líneas lógicas: sin comentarios ni vacías, y con las continuaciones ya unidas.
fn logical_lines(text: &str) -> Vec<String> {
    let normalized = text.replace("\r\n", "\n");
    let mut physical = normalized.split(['\n', '\r']);
    let mut logical = Vec::new();

    while let Some(first) = physical.next() {
        let first = first.trim_start_matches(is_properties_blank);
        if first.is_empty() || first.starts_with('#') || first.starts_with('!') {
            continue;
        }
        let mut line = first.to_owned();
        while ends_in_a_continuation(&line) {
            line.pop();
            let Some(next) = physical.next() else { break };
            line.push_str(next.trim_start_matches(is_properties_blank));
        }
        logical.push(line);
    }

    logical
}

fn ends_in_a_continuation(line: &str) -> bool {
    let backslashes = line.chars().rev().take_while(|it| *it == '\\').count();
    backslashes % 2 == 1
}

/// Parte la línea lógica: la clave acaba en el primer `=`, `:` o blanco sin escapar, y entre
/// clave y valor cabe un blanco, un único `=` o `:` y más blancos.
fn split_key_and_value(line: &str) -> (&str, &str) {
    let mut escaped = false;
    let key_end = line
        .char_indices()
        .find(|(_, character)| {
            if escaped {
                escaped = false;
                return false;
            }
            if *character == '\\' {
                escaped = true;
                return false;
            }
            matches!(character, '=' | ':') || is_properties_blank(*character)
        })
        .map_or(line.len(), |(index, _)| index);

    let after_key = line[key_end..].trim_start_matches(is_properties_blank);
    let value = after_key
        .strip_prefix(['=', ':'])
        .unwrap_or(after_key)
        .trim_start_matches(is_properties_blank);
    (&line[..key_end], value)
}

/// Deshace los escapes de `Properties.load`, o nada si un `\u` no lleva cuatro dígitos hexadecimales.
fn unescape(value: &str) -> Option<String> {
    let mut units: Vec<u16> = Vec::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            push_character(&mut units, character);
            continue;
        }
        match characters.next() {
            Some('n') => units.push(u16::from(b'\n')),
            Some('r') => units.push(u16::from(b'\r')),
            Some('t') => units.push(u16::from(b'\t')),
            Some('f') => units.push(0x0c),
            Some('u') => units.push(unicode_unit(&mut characters)?),
            Some(other) => push_character(&mut units, other),
            None => break,
        }
    }

    Some(String::from_utf16_lossy(&units))
}

fn push_character(units: &mut Vec<u16>, character: char) {
    let mut buffer = [0u16; 2];
    units.extend_from_slice(character.encode_utf16(&mut buffer));
}

fn unicode_unit(characters: &mut std::str::Chars) -> Option<u16> {
    let digits: String = characters.take(4).collect();
    if digits.len() != 4 || !digits.chars().all(|it| it.is_ascii_hexdigit()) {
        return None;
    }
    u16::from_str_radix(&digits, 16).ok()
}
