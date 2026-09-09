//! El almacén que la sede nombra en `keystore` o en `ksb64`, y cuál de ellos abre rFirma (ADR-0022).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use super::codes::{Parameter, SafCode};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

const LEGACY_KEY_STORE: &str = "keystore";

const ENCODED_KEY_STORE: &str = "ksb64";

/// Los nombres de `AOKeyStore` (1.9.2) que nombran el almacén NSS que rFirma ya abre.
const THE_NSS_STORES: [&str; 2] = ["SHARED_NSS", "MOZ_UNI"];

/// Los nombres de `AOKeyStore` (1.9.2): lo único que el original reconoce como almacén.
const THE_STORES_OF_THE_ORIGINAL: [&str; 18] = [
    "WINDOWS",
    "APPLE",
    "SHARED_NSS",
    "PKCS12",
    "JAVA",
    "PKCS11",
    "SINGLE",
    "MOZ_UNI",
    "JCEKS",
    "JAVACE",
    "TEMD",
    "WINADDRESSBOOK",
    "WINCA",
    "CERES",
    "DNIEJAVA",
    "KNOWN_SMARTCARDS",
    "SMARTCAFE",
    "CERES_430",
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

fn named_among(name: &str, stores: &[&str]) -> bool {
    stores.iter().any(|store| store.eq_ignore_ascii_case(name))
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
    let normalized: String = encoded
        .chars()
        .filter(|character| *character != '=')
        .map(|character| if character == '/' { '_' } else { character })
        .collect();

    let bytes = URL_SAFE_NO_PAD.decode(normalized.as_bytes()).ok()?;
    String::from_utf8(bytes).ok()
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
