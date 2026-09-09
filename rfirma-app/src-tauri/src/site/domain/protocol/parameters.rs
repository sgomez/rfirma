//! Lo común a toda operación: las guardias de forma y los dos indicadores del certificado pegado.

use super::codes::{Parameter, SafCode};
use super::refusal::Refusal;
use super::url::AfirmaUrl;
use super::version::{Version, IMPLEMENTED_AUTOFIRMA_VERSION};

const LOCAL_FILE_PREFIX: &str = "file:/";
const LOCAL_HOSTS: [&str; 2] = ["localhost", "127.0.0.1"];
const STICKY: &str = "sticky";
const RESET_STICKY: &str = "resetsticky";

/// Lo que la sede pide sobre el certificado pegado del proceso.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StickyCertificate {
    sticky: bool,
    resets: bool,
}

impl StickyCertificate {
    /// Si la sede acepta que se conteste con el certificado recordado sin preguntar.
    pub fn is_sticky(&self) -> bool {
        self.sticky
    }

    /// Si la sede pide olvidar el certificado recordado antes de resolver.
    pub fn resets(&self) -> bool {
        self.resets
    }
}

/// Lee `sticky` y `resetsticky`, que `selectcert`, `sign`, `signandsave` y `batch` comparten.
pub fn sticky_certificate(url: &AfirmaUrl) -> StickyCertificate {
    StickyCertificate {
        sticky: flag_of(url, STICKY),
        resets: flag_of(url, RESET_STICKY),
    }
}

fn flag_of(url: &AfirmaUrl, name: &str) -> bool {
    url.parameter(name).is_some_and(reads_as_true)
}

/// Lo que `Boolean.parseBoolean` acepta: `true` sin distinguir mayúsculas y sin recortar espacios.
pub fn reads_as_true(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

/// Comprueba una URL de servlet como `UrlParameters.validateURL`: `http` o `https`, host no
/// local y sin parámetros propios.
pub fn check_servlet_url(candidate: &str, blame: Parameter) -> Result<(), Refusal> {
    let Some((scheme, rest)) = candidate.split_once("://") else {
        return Err(Refusal::about(
            blame,
            format!("la url '{candidate}' no tiene forma de url absoluta"),
        ));
    };

    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err(Refusal::about(
            blame,
            format!("el esquema '{scheme}' no se admite en una url de servlet"),
        ));
    }

    let host = host_of(rest);
    if host.is_empty() {
        return Err(Refusal::about(
            blame,
            format!("la url '{candidate}' no trae host"),
        ));
    }
    if LOCAL_HOSTS
        .iter()
        .any(|local| host.eq_ignore_ascii_case(local))
    {
        return Err(Refusal::new(
            SafCode::LocalAccessBlocked,
            format!("el parametro '{blame}' pide acceso a una direccion local: {candidate}"),
        ));
    }

    if candidate.contains('?') || candidate.contains('=') {
        return Err(Refusal::about(
            blame,
            format!("la url de servlet no admite parametros propios: {candidate}"),
        ));
    }

    Ok(())
}

fn host_of(rest: &str) -> &str {
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority.rsplit_once('@').map_or(authority, |(_, it)| it);
    authority.split_once(':').map_or(authority, |(it, _)| it)
}

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
