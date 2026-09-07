//! Quién es el titular de un certificado, leído de su nombre distinguido (RFC 4514).

use crate::identity::domain::certificate::TokenCertificate;

/// Pares atributo=valor de un nombre distinguido respetando comas escapadas (RFC 4514).
fn attribute_pairs(distinguished_name: &str) -> Vec<String> {
    let mut pairs = Vec::new();
    let mut start = 0;
    let bytes = distinguished_name.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b',' && !comma_is_escaped(bytes, index) {
            pairs.push(unescape(&distinguished_name[start..index]));
            start = index + 1;
        }
    }
    pairs.push(unescape(&distinguished_name[start..]));
    pairs
}

/// Comprueba si la coma en `index` está precedida por un número impar de barras invertidas.
fn comma_is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut backslashes = 0;
    while index > backslashes && bytes[index - 1 - backslashes] == b'\\' {
        backslashes += 1;
    }
    backslashes % 2 == 1
}

/// Desescapa caracteres según RFC 4514.
fn unescape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            if let Some(escaped) = chars.next() {
                result.push(escaped);
                continue;
            }
        }
        result.push(character);
    }
    result
}

/// Extrae el valor de un atributo de un nombre distinguido.
pub fn attribute(name: &str, distinguished_name: &str) -> String {
    attribute_pairs(distinguished_name)
        .into_iter()
        .find_map(|part| part.trim().strip_prefix(name).map(str::to_owned))
        .unwrap_or_default()
}

/// Extrae el nombre común (CN) y número de serie del subject del certificado.
pub fn holder_of(subject: Option<&str>) -> (String, String) {
    let subject = subject.unwrap_or_default();
    (
        attribute("CN=", subject),
        attribute("SERIALNUMBER=", subject),
    )
}

/// Datos del titular a estampar en la firma visible.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StampedHolder {
    /// El `CN` del subject, entero.
    pub common_name: String,
    /// La autoridad emisora, la misma que enseña el desplegable.
    pub issuer: String,
    /// Si el certificado es de seudónimo.
    pub pseudonym: bool,
}

/// Lo que el recuadro estampa de un certificado, leído del DER.
pub fn stamped_holder_of(certificate: &TokenCertificate) -> StampedHolder {
    let subject = certificate.subject();
    StampedHolder {
        common_name: attribute("CN=", subject.as_deref().unwrap_or_default()),
        issuer: issuer_of(certificate.issuer().as_deref()),
        pseudonym: is_pseudonym(subject.as_deref()),
    }
}

/// Comprueba si el certificado es de seudónimo según el RDN 2.5.4.65.
pub fn is_pseudonym(subject: Option<&str>) -> bool {
    const PSEUDONYM: [&str; 3] = ["2.5.4.65=", "OID.2.5.4.65=", "PSEUDONYM="];
    attribute_pairs(subject.unwrap_or_default())
        .iter()
        .any(|pair| {
            let pair = pair.trim().to_ascii_uppercase();
            PSEUDONYM.iter().any(|name| pair.starts_with(name))
        })
}

/// Extrae el nombre de la autoridad emisora a partir del emisor del certificado.
pub fn issuer_of(issuer: Option<&str>) -> String {
    let issuer = issuer.unwrap_or_default().trim();
    let common_name = attribute("CN=", issuer);
    if !common_name.is_empty() {
        return common_name;
    }
    let organisation = attribute("O=", issuer);
    if !organisation.is_empty() {
        return organisation;
    }
    issuer.to_owned()
}

#[cfg(test)]
mod tests;
