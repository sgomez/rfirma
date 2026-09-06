//! El marcador de que el hilo del isolate murió, sin el hilo.

/// Error cuando el hilo del isolate ha terminado inesperadamente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsolateGone;

impl std::fmt::Display for IsolateGone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("el hilo de la librería nativa se ha caído")
    }
}

impl std::error::Error for IsolateGone {}
