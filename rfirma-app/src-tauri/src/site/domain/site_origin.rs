//! El origen `https://` del saludo de la sede, o su ausencia: se atribuye y nunca controla el acceso.

#[cfg(test)]
mod tests;

/// El `host[:puerto]` de un origen `https://` sin ruta, consulta ni credenciales; o su ausencia,
/// si la cabecera `Origin` faltaba, valía `null`, no era `https://` o no se pudo analizar.
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
    let host = raw.strip_prefix("https://")?;
    let counts = !host.is_empty() && !host.contains(['/', '?', '#', '@']);
    counts.then(|| host.to_owned())
}
