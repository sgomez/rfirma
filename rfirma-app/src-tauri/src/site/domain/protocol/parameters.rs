//! Las dos guardias comunes a toda operación: versión mínima y acceso local.

use super::codes::{Parameter, SafCode};
use super::refusal::Refusal;
use super::version::{Version, IMPLEMENTED_AUTOFIRMA_VERSION};

const LOCAL_FILE_PREFIX: &str = "file:/";

/// Comprueba la versión mínima de cliente que exige la sede.
pub fn check_minimum_client_version(requested: Option<&str>) -> Result<(), Refusal> {
    let Some(requested) = requested.filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    let Ok(requested_version) = Version::parse(requested) else {
        return Err(Refusal::about(
            Parameter::MinimumClientVersion,
            format!("el parametro 'mcv' no tiene forma de version: {requested}"),
        ));
    };
    let implemented = Version::parse(IMPLEMENTED_AUTOFIRMA_VERSION)
        .expect("la version implementada es una constante y tiene que parsear");

    if requested_version.greater_than(&implemented) {
        return Err(Refusal::new(
            SafCode::MinimumVersionNonSatisfied,
            format!(
                "la sede exige la version {requested} y aqui se implementa la \
                 {IMPLEMENTED_AUTOFIRMA_VERSION}"
            ),
        ));
    }

    Ok(())
}

/// Comprueba que los datos a firmar no pidan un fichero local.
pub fn check_local_access_is_not_requested(data: &str) -> Result<(), Refusal> {
    let candidate = data.trim_start().to_ascii_lowercase();

    if candidate.starts_with(LOCAL_FILE_PREFIX) {
        return Err(Refusal::about(
            Parameter::Data,
            "no se permite la lectura de ficheros locales: el parametro 'dat' pide un 'file:/'",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests;
