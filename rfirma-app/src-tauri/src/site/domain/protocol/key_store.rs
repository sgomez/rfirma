//! El almacén que la sede nombra en `keystore` o en `ksb64`, y cuál de ellos abre rFirma (ADR-0022).

use super::codes::{Parameter, SafCode};
use super::refusal::{Refusal, RefusalSituation};
use super::url::{decode_protocol_base64, AfirmaUrl};

const LEGACY_KEY_STORE: &str = "keystore";

const ENCODED_KEY_STORE: &str = "ksb64";

/// Los dos nombres —el visible y el de la constante— del almacén NSS que rFirma ya abre.
const THE_NSS_STORES: [KeyStoreOfTheOriginal; 2] = [
    ("NSS", "SHARED_NSS"),
    ("Mozilla / Firefox (unificado)", "MOZ_UNI"),
];

/// El nombre visible de un `AOKeyStore` y el de su constante, que son las dos puertas por las
/// que el original resuelve el almacén (`SimpleKeyStoreManager.getKeyStore`, 1.9.2).
type KeyStoreOfTheOriginal = (&'static str, &'static str);

/// Los almacenes de `AOKeyStore` (1.9.2): lo único que el original reconoce como almacén.
const THE_STORES_OF_THE_ORIGINAL: [KeyStoreOfTheOriginal; 19] = [
    ("Windows", "WINDOWS"),
    ("Llavero de Mac", "APPLE"),
    ("NSS", "SHARED_NSS"),
    ("PKCS#12 / PFX", "PKCS12"),
    ("Java KeyStore / JKS", "JAVA"),
    ("PKCS#11", "PKCS11"),
    ("PKCS#7 / X.509", "SINGLE"),
    ("Mozilla / Firefox (unificado)", "MOZ_UNI"),
    ("Java Cryptography Extension KeyStore (JCEKS)", "JCEKS"),
    ("Java KeyStore / JKS (Case Exact)", "JAVACE"),
    ("TEMD (Tarjeta del Ministerio de Defensa)", "TEMD"),
    (
        "Windows / Internet Explorer (otras personas / libreta de direcciones)",
        "WINADDRESSBOOK",
    ),
    ("Windows / Internet Explorer (CA intermedias)", "WINCA"),
    ("Tarjeta FNMT-RCM CERES", "CERES"),
    ("DNIe y tarjetas FNMT-TIF", "DNIEJAVA"),
    (
        "Tarjetas inteligentes conocidas mediante PKCS#11",
        "KNOWN_SMARTCARDS",
    ),
    ("G&D SmartCafe con Applet PKCS#15", "SMARTCAFE"),
    ("Tarjeta FNMT-RCM CERES 4.30 o superior", "CERES_430"),
    ("Tipo desconocido", "OTHER"),
];

/// El almacén que nombra la sede: cómo lo llama y qué biblioteca PKCS#11 le pone detrás.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedKeyStore {
    declared_in: Parameter,
    name: String,
    library: Option<String>,
}

impl NamedKeyStore {
    /// El nombre del almacén, que puede venir vacío cuando la sede solo nombra biblioteca.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// La ruta de la biblioteca PKCS#11 que acompaña al nombre, si la hay.
    pub fn library(&self) -> Option<&str> {
        self.library.as_deref()
    }

    /// El parámetro por el que llegó, que es el que se le nombra a la sede.
    pub fn declared_in(&self) -> Parameter {
        self.declared_in
    }
}

/// El almacén que nombran `keystore` o `ksb64`, con la prioridad y las rarezas del original
/// (`UrlParameters.getKeyStoreName`, 1.9.2).
pub fn key_store_named_by(url: &AfirmaUrl) -> Option<NamedKeyStore> {
    let (declared_in, value) = declared_key_store(url)?;
    let (name, library) = split_on_the_first_colon(&value);
    if name.is_empty() && library.is_none() {
        return None;
    }
    Some(NamedKeyStore {
        declared_in,
        name,
        library,
    })
}

/// El `SAF_07` del almacén que la sede nombra y rFirma no abre (ADR-0022).
pub fn refuse_a_key_store_rfirma_does_not_open(url: &AfirmaUrl) -> Result<(), Refusal> {
    let Some(named) = key_store_named_by(url) else {
        return Ok(());
    };
    if let Some(library) = named.library() {
        return Err(refused(
            named.declared_in(),
            format!("rFirma no carga la biblioteca '{library}' que nombra la sede"),
        ));
    }
    if is_a_store_rfirma_does_not_open(named.name()) {
        return Err(refused(
            named.declared_in(),
            format!(
                "rFirma no abre el almacen '{}' que nombra la sede",
                named.name()
            ),
        ));
    }
    Ok(())
}

fn refused(blame: Parameter, detail: String) -> Refusal {
    Refusal::new(SafCode::CannotFindKeystore, detail)
        .blaming(blame)
        .because(RefusalSituation::UnsupportedKeyStore)
}

/// Un nombre del original que no es el de la familia NSS; uno que el original tampoco reconoce
/// no lo es, porque allí acaba en el almacén por omisión del sistema.
fn is_a_store_rfirma_does_not_open(name: &str) -> bool {
    named_among(name, &THE_STORES_OF_THE_ORIGINAL) && !named_among(name, &THE_NSS_STORES)
}

/// El original mira antes el nombre visible, y ya recortado (ADR-0022).
fn named_among(name: &str, stores: &[KeyStoreOfTheOriginal]) -> bool {
    stores.iter().any(|(visible, constant)| {
        visible.eq_ignore_ascii_case(name.trim()) || constant.eq_ignore_ascii_case(name)
    })
}

/// El `keystore` heredado si vino, y solo si no, el `ksb64`, que se ignora entero cuando no es Base64.
fn declared_key_store(url: &AfirmaUrl) -> Option<(Parameter, String)> {
    if let Some(value) = url.parameter(LEGACY_KEY_STORE) {
        return Some((Parameter::LegacyKeyStore, value.to_owned()));
    }
    let encoded = url.parameter(ENCODED_KEY_STORE)?;
    decoded(encoded).map(|value| (Parameter::KeyStore, value))
}

fn decoded(encoded: &str) -> Option<String> {
    String::from_utf8(decode_protocol_base64(encoded).ok()?).ok()
}

/// El nombre a la izquierda del primer `:` y la biblioteca a su derecha, sin las comillas que
/// el original le quita (`UrlParameters.cleanupPath`, 1.9.2).
fn split_on_the_first_colon(value: &str) -> (String, Option<String>) {
    let Some(colon) = value.find(':') else {
        return (value.to_owned(), None);
    };
    let library = value[colon + 1..].trim().replace(['"', '\''], "");
    (
        value[..colon].trim().to_owned(),
        (!library.is_empty()).then_some(library),
    )
}

#[cfg(test)]
mod tests;
