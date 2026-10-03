//! `SiteOrigin`: el `host[:puerto]` del `Origin` `https://` del saludo de la sede, o su ausencia; se atribuye y nunca controla el acceso.

#[cfg(test)]
mod tests;

/// El `host[:puerto]` de un origen `https://` bien formado, o su ausencia si no lo era.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteOrigin(Option<String>);

impl SiteOrigin {
    /// Construye el origen a partir del valor crudo de la cabecera `Origin` del saludo.
    pub fn from_header(raw: Option<&str>) -> Self {
        Self(raw.and_then(host_of_an_https_origin))
    }

    /// El origen ausente, para los transportes que no traen la cabecera `Origin`.
    pub fn absent() -> Self {
        Self(None)
    }

    /// El `host[:puerto]` tal y como llegó, sin decodificar; `None` si el origen está ausente.
    pub fn host(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// El `host[:puerto]` de un `Origin` que es `https://`, sin ruta, consulta ni credenciales.
fn host_of_an_https_origin(raw: &str) -> Option<String> {
    let after_scheme = raw.strip_prefix("https://")?;
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..authority_end];
    if authority.ends_with(':') {
        return None;
    }
    let parsed = url::Url::parse(raw).ok()?;
    let is_a_bare_https_origin = parsed.scheme() == "https"
        && parsed.host_str().is_some_and(|host| !host.is_empty())
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.path() == "/"
        && parsed.query().is_none()
        && parsed.fragment().is_none();
    is_a_bare_https_origin.then(|| authority.to_owned())
}
