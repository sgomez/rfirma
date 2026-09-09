//! La traza por `stderr` de lo que llega por `afirma://`, y solo en compilación de desarrollo.

use crate::site::domain::protocol::AfirmaUrl;

/// Deja en `stderr` la invocación de arranque, abreviada si se puede leer.
pub fn note_the_launch(url: &str) {
    if !cfg!(debug_assertions) {
        return;
    }
    let shown = AfirmaUrl::parse(url).map_or_else(|_| url.to_owned(), |url| url.abridged());
    eprintln!("rfirma: arranque {shown}");
}

/// Deja en `stderr` la operación que llega por el canal ya abierto.
pub fn note_the_operation(url: &AfirmaUrl) {
    if !cfg!(debug_assertions) {
        return;
    }
    eprintln!("rfirma: operacion {}", url.abridged());
}
