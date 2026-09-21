//! El registro de una comprobación: la línea marcada por su procedencia y su reloj, el fichero que
//! la conserva y el extremo que la entrega en vivo; no decide quién la mira.

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Quién habló: la sede que conduce el cliente publicado, el cliente a prueba o la propia suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provenance {
    Site,
    Client,
    Suite,
}

pub(crate) fn provenance_tag(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::Site => "sede",
        Provenance::Client => "cliente",
        Provenance::Suite => "suite",
    }
}

/// El reloj monótono desde el arranque de la comprobación, `MM:SS.cc`, nunca la hora del día.
pub(crate) fn format_elapsed_clock(elapsed: Duration) -> String {
    let total_millis = elapsed.as_millis();
    let minutes = total_millis / 60_000;
    let seconds = (total_millis / 1_000) % 60;
    let centis = (total_millis % 1_000) / 10;
    format!("{minutes:02}:{seconds:02}.{centis:02}")
}

/// La línea tal y como sale, con el mismo formato en vivo y en el fichero.
pub(crate) fn format_log_line(elapsed: Duration, provenance: Provenance, text: &str) -> String {
    format!(
        "{} {:<7} {text}",
        format_elapsed_clock(elapsed),
        provenance_tag(provenance)
    )
}

/// El fichero de una comprobación, junto a su transcripción: lo que se lee cuando ya terminó.
#[derive(Clone)]
pub(crate) struct CheckLog {
    file: Arc<Mutex<std::fs::File>>,
}

impl CheckLog {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{} no se pudo crear: {error}", parent.display()))?;
        }
        let file = std::fs::File::create(path)
            .map_err(|error| format!("{} no se pudo crear: {error}", path.display()))?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Añade `line`, ya formateada, y la deja en disco antes de devolver el control.
    pub(crate) fn write(&self, line: &str) {
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }
}

/// El extremo por el que la sede, el cliente y la suite entregan sus líneas, desde sus hilos.
#[derive(Clone)]
pub(crate) struct LiveLogSink {
    deliver: Arc<dyn Fn(String) + Send + Sync>,
}

impl LiveLogSink {
    pub(crate) fn new(deliver: impl Fn(String) + Send + Sync + 'static) -> Self {
        Self {
            deliver: Arc::new(deliver),
        }
    }

    pub(crate) fn push(&self, provenance: Provenance, elapsed: Duration, text: &str) {
        (self.deliver)(format_log_line(elapsed, provenance, text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_formats_minutes_seconds_and_centiseconds() {
        assert_eq!(format_elapsed_clock(Duration::from_millis(310)), "00:00.31");
        assert_eq!(
            format_elapsed_clock(Duration::from_millis(1190)),
            "00:01.19"
        );
        assert_eq!(
            format_elapsed_clock(Duration::from_millis(61_000)),
            "01:01.00"
        );
    }

    #[test]
    fn provenance_tag_names_site_client_and_suite() {
        assert_eq!(provenance_tag(Provenance::Site), "sede");
        assert_eq!(provenance_tag(Provenance::Client), "cliente");
        assert_eq!(provenance_tag(Provenance::Suite), "suite");
    }

    #[test]
    fn live_log_sink_delivers_the_line_formatted_like_the_file() {
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let into = Arc::clone(&delivered);
        let sink = LiveLogSink::new(move |line| into.lock().unwrap().push(line));

        sink.push(Provenance::Suite, Duration::from_millis(310), "aviso");

        assert_eq!(*delivered.lock().unwrap(), ["00:00.31 suite   aviso"]);
    }

    #[test]
    fn check_log_keeps_the_line_as_the_live_sink_formats_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a_check.log");
        let log = CheckLog::open(&path).unwrap();
        log.write(&format_log_line(
            Duration::from_millis(310),
            Provenance::Site,
            "launch",
        ));
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written, "00:00.31 sede    launch\n");
    }
}
