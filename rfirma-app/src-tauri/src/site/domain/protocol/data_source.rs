//! De dónde salen los datos cuando `dat` no los trae: el puerto que los baja y qué valor es una descarga.

/// El origen remoto de `dat`: quien baja lo que hay en una URL `http(s)` (`DataDownloader`, 1.9.2).
pub trait DataSource {
    /// Los bytes que hay en esa URL, o por qué no se han podido obtener.
    fn download(&self, url: &str) -> Result<Vec<u8>, String>;
}

const HTTP: &str = "http://";
const HTTPS: &str = "https://";

/// La URL de la que hay que bajar el `dat`, o nada si el valor son los datos en sí.
pub fn download_url(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (trimmed.starts_with(HTTP) || trimmed.starts_with(HTTPS)).then_some(trimmed)
}

#[cfg(test)]
mod tests;
