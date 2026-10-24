//! El registro de una comprobación: la línea marcada por su procedencia y su reloj, el fichero que
//! la conserva y el extremo que la entrega en vivo; no decide quién la mira.

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

/// Quién habló: la sede que conduce el cliente publicado, el cliente a prueba o la propia suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) enum Provenance {
    #[serde(rename = "sede")]
    #[ts(rename = "sede")]
    Site,
    #[serde(rename = "cliente")]
    #[ts(rename = "cliente")]
    Client,
    #[serde(rename = "suite")]
    #[ts(rename = "suite")]
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

/// Una línea del registro ya descompuesta, para que quien la muestra no analice texto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) struct LogLine {
    pub(crate) clock: String,
    pub(crate) provenance: Provenance,
    pub(crate) text: String,
}

/// Descompone una línea del fichero; la que no tiene el formato de `format_log_line` es de la suite.
pub(crate) fn parse_log_line(raw: &str) -> LogLine {
    let mut parts = raw.splitn(3, ' ');
    let clock = parts.next().unwrap_or_default();
    let tag = parts.next().unwrap_or_default();
    let provenance = match tag {
        "sede" | "conductor" => Some(Provenance::Site),
        "cliente" | "sujeto" => Some(Provenance::Client),
        "suite" | "arnés" => Some(Provenance::Suite),
        _ => None,
    };
    match provenance {
        Some(provenance) if clock.len() == 8 && clock.as_bytes()[2] == b':' => LogLine {
            clock: clock.to_owned(),
            provenance,
            text: parts.next().unwrap_or_default().trim_start().to_owned(),
        },
        _ => LogLine {
            clock: String::new(),
            provenance: Provenance::Suite,
            text: raw.to_owned(),
        },
    }
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
    deliver: Arc<dyn Fn(Provenance, String) + Send + Sync>,
}

impl LiveLogSink {
    pub(crate) fn new(deliver: impl Fn(Provenance, String) + Send + Sync + 'static) -> Self {
        Self {
            deliver: Arc::new(deliver),
        }
    }

    pub(crate) fn push(&self, provenance: Provenance, elapsed: Duration, text: &str) {
        (self.deliver)(provenance, format_log_line(elapsed, provenance, text));
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
    fn a_file_line_is_split_into_clock_provenance_and_text() {
        let line = parse_log_line(&format_log_line(
            Duration::from_millis(310),
            Provenance::Client,
            "abre el canal",
        ));
        assert_eq!(
            line,
            LogLine {
                clock: "00:00.31".into(),
                provenance: Provenance::Client,
                text: "abre el canal".into()
            }
        );
    }

    #[test]
    fn a_line_without_the_format_is_the_suites_whole() {
        let line = parse_log_line("sin formato");
        assert_eq!(line.provenance, Provenance::Suite);
        assert_eq!(line.text, "sin formato");
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
        let sink = LiveLogSink::new(move |_, line| into.lock().unwrap().push(line));

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
