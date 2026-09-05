//! Los parámetros que trae **cualquier** operación, y sus dos guardias.
//!
//! Es `UrlParameters` del original
//! (`es/gob/afirma/core/misc/protocol/UrlParameters.java`) reducido a lo que
//! afecta a todas las operaciones por igual: la versión mínima que exige la
//! sede y el origen de los datos. Lo propio de cada operación —`format`,
//! `algorithm`, los filtros— no vive aquí.
//!
//! Las dos guardias se comprueban **en los cuatro lanzadores** del original,
//! `selectcert` incluido y no sólo en la firma (ID-251), así que aquí son
//! funciones sueltas: las llama cada operación, no un constructor que sólo la
//! firma usaría.

use super::refusal::{Refusal, SafCode};
use super::version::{Version, IMPLEMENTED_AUTOFIRMA_VERSION};

/// El prefijo que el original prohíbe en `dat` (`UrlParameters.java:300`-`303`).
const LOCAL_FILE_PREFIX: &str = "file:/";

/// `mcv`: la versión mínima de cliente que exige la sede.
///
/// Se compara contra [`IMPLEMENTED_AUTOFIRMA_VERSION`], que es la versión de
/// AutoFirma que rFirma declara implementar y **no** la versión de rFirma
/// (ID-250). La comparación es la del original, que no es semver (ID-251,
/// [`super::version`]).
///
/// Ausente es «no exige nada»: el `buildUrl` del cliente publicado sólo
/// antepone `mcv` cuando la sede llamó a `setMinimumClientVersion`.
pub fn check_minimum_client_version(requested: Option<&str>) -> Result<(), Refusal> {
    let Some(requested) = requested.filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    let Ok(requested_version) = Version::parse(requested) else {
        return Err(Refusal::params(format!(
            "el parametro 'mcv' no tiene forma de version: {requested}"
        )));
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

/// `dat`: los datos a firmar, y la defensa contra leer ficheros del equipo.
///
/// Un `dat` que empieza por `file:/` es la sede pidiendo que rFirma le abra un
/// fichero local y se lo firme (ID-267). El original ya lo prohíbe con
/// `ParameterLocalAccessRequestedException`; aquí, además, **sin distinguir
/// mayúsculas y sin dejarse engañar por espacios delante**, que al original sí
/// se le cuelan. No cuesta nada: el Base64 de una operación real nunca empieza
/// por espacio.
pub fn check_local_access_is_not_requested(data: &str) -> Result<(), Refusal> {
    let candidate = data.trim_start().to_ascii_lowercase();

    if candidate.starts_with(LOCAL_FILE_PREFIX) {
        return Err(Refusal::params(
            "no se permite la lectura de ficheros locales: el parametro 'dat' pide un 'file:/'",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_that_demands_nothing_is_served() {
        assert!(check_minimum_client_version(None).is_ok());
        assert!(check_minimum_client_version(Some("")).is_ok());
    }

    #[test]
    fn the_versions_the_sites_actually_demand_are_served() {
        for requested in ["1.6", "1.7", "1.8", "1.9", "1.9.2"] {
            assert!(
                check_minimum_client_version(Some(requested)).is_ok(),
                "una sede que exige {requested} tiene que poder firmar"
            );
        }
    }

    #[test]
    fn a_version_newer_than_the_one_implemented_is_refused_with_its_own_code() {
        let refusal =
            check_minimum_client_version(Some("1.9.3")).expect_err("no se implementa la 1.9.3");

        assert_eq!(refusal.code(), SafCode::MinimumVersionNonSatisfied);
    }

    #[test]
    fn the_comparison_is_against_autofirma_and_not_against_the_version_of_rfirma() {
        assert_eq!(IMPLEMENTED_AUTOFIRMA_VERSION, "1.9.2");
        assert!(
            check_minimum_client_version(Some("1.9")).is_ok(),
            "con la version de rFirma —0.x— esta exigencia daria SAF_41 y no se firmaria nunca"
        );
    }

    #[test]
    fn a_minimum_version_that_does_not_parse_is_a_parameter_error() {
        for requested in ["ultima", "1.a", "1..9"] {
            let refusal =
                check_minimum_client_version(Some(requested)).expect_err("no es una version");

            assert_eq!(refusal.code(), SafCode::Params, "con {requested}");
        }
    }

    #[test]
    fn data_that_asks_for_a_local_file_is_refused() {
        for data in [
            "file:/etc/passwd",
            "file:///etc/passwd",
            "FILE:/etc/passwd",
            "  file:/x",
        ] {
            let refusal = check_local_access_is_not_requested(data)
                .expect_err("la sede no elige que ficheros se leen");

            assert_eq!(refusal.code(), SafCode::Params, "con {data}");
        }
    }

    #[test]
    fn base64_data_goes_through() {
        assert!(check_local_access_is_not_requested("JVBERi0xLjcKJeLjz9M").is_ok());
        assert!(check_local_access_is_not_requested("").is_ok());
    }
}
