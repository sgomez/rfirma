//! El registro separado del progreso: la traducción a texto de una línea marcada por su
//! procedencia, la composición de la región viva y el fichero por comprobación que la conserva.

use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// El ancho de la región viva; no se consulta el terminal para no sumar una dependencia nueva.
pub(crate) const DEFAULT_LOG_WIDTH: usize = 74;

/// Quién habló: el conductor de Node que ejecuta el cliente publicado, o el binario declarado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provenance {
    Driver,
    Subject,
}

pub(crate) fn provenance_tag(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::Driver => "conductor",
        Provenance::Subject => "sujeto",
    }
}

/// El filtro de procedencia declarado al arrancar, con `todo` por omisión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LogFilter {
    Driver,
    Subject,
    #[default]
    All,
}

impl LogFilter {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "conductor" => Ok(Self::Driver),
            "sujeto" => Ok(Self::Subject),
            "todo" => Ok(Self::All),
            other => Err(format!(
                "no conozco la procedencia «{other}»; las que hay son: conductor, sujeto, todo"
            )),
        }
    }

    fn allows(self, provenance: Provenance) -> bool {
        match self {
            Self::All => true,
            Self::Driver => provenance == Provenance::Driver,
            Self::Subject => provenance == Provenance::Subject,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "conductor · sujeto",
            Self::Driver => "conductor",
            Self::Subject => "sujeto",
        }
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

/// Una línea ya marcada por su procedencia y su reloj, tal y como viven en la región viva y en el
/// fichero por comprobación.
#[derive(Debug, Clone)]
pub(crate) struct LiveLogLine {
    pub(crate) elapsed: Duration,
    pub(crate) provenance: Provenance,
    pub(crate) text: String,
}

/// La línea tal y como sale, con el mismo formato en la región viva y en el fichero.
fn format_log_line(elapsed: Duration, provenance: Provenance, text: &str) -> String {
    format!(
        "{} {:<9} {text}",
        format_elapsed_clock(elapsed),
        provenance_tag(provenance)
    )
}

fn truncated_to(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut truncated: String = text.chars().take(width.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

/// Dadas unas líneas, un alto y un ancho, el texto de la región viva; vacío si el alto es cero.
pub(crate) fn compose_log_region(
    lines: &[LiveLogLine],
    height: usize,
    width: usize,
    filter: LogFilter,
) -> String {
    if height == 0 || width < 12 {
        return String::new();
    }
    let visible = &lines[lines.len().saturating_sub(height)..];
    let inner_width = width - 4;
    let label = format!("[{}]", filter.label());
    let title = "─ registro ";
    let fixed = 5 + title.chars().count() + label.chars().count();
    let dashes = "─".repeat(width.saturating_sub(fixed).max(1));
    let header = format!("┌{title}{dashes} {label} ─┐");
    let footer = format!("└{}┘", "─".repeat(width.saturating_sub(2)));

    let mut result = vec![header];
    for line in visible {
        let content = format_log_line(line.elapsed, line.provenance, &line.text);
        let truncated = truncated_to(&content, inner_width);
        result.push(format!("│ {truncated:<inner_width$} │"));
    }
    result.push(footer);
    result.join("\n")
}

/// El fichero de una comprobación, junto a su transcripción, con el mismo formato que la región
/// viva: es lo que sirve para investigar una sorpresa cuando la tanda ya terminó.
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

    pub(crate) fn record(&self, elapsed: Duration, provenance: Provenance, text: &str) {
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{}", format_log_line(elapsed, provenance, text));
            let _ = file.flush();
        }
    }
}

/// La región viva compartida entre el hilo que la redibuja y los hilos que reciben del conductor y
/// del sujeto: un búfer acotado a `height`, filtrado por procedencia.
#[derive(Clone)]
pub(crate) struct LiveLogSink {
    buffer: Arc<Mutex<VecDeque<LiveLogLine>>>,
    filter: LogFilter,
    height: usize,
}

impl LiveLogSink {
    pub(crate) fn new(filter: LogFilter, height: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(height.max(1)))),
            filter,
            height,
        }
    }

    pub(crate) fn push(&self, provenance: Provenance, elapsed: Duration, text: String) {
        if self.height == 0 || !self.filter.allows(provenance) {
            return;
        }
        let mut buffer = self.buffer.lock().unwrap();
        if buffer.len() >= self.height {
            buffer.pop_front();
        }
        buffer.push_back(LiveLogLine {
            elapsed,
            provenance,
            text,
        });
    }

    pub(crate) fn snapshot(&self) -> Vec<LiveLogLine> {
        self.buffer.lock().unwrap().iter().cloned().collect()
    }

    pub(crate) fn clear(&self) {
        self.buffer.lock().unwrap().clear();
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn filter(&self) -> LogFilter {
        self.filter
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
    fn provenance_tag_names_driver_and_subject() {
        assert_eq!(provenance_tag(Provenance::Driver), "conductor");
        assert_eq!(provenance_tag(Provenance::Subject), "sujeto");
    }

    #[test]
    fn log_filter_parses_the_three_words() {
        assert_eq!(LogFilter::parse("conductor"), Ok(LogFilter::Driver));
        assert_eq!(LogFilter::parse("sujeto"), Ok(LogFilter::Subject));
        assert_eq!(LogFilter::parse("todo"), Ok(LogFilter::All));
    }

    #[test]
    fn log_filter_rejects_an_unknown_word() {
        assert!(LogFilter::parse("ambos").is_err());
    }

    #[test]
    fn log_filter_all_allows_both_provenances() {
        assert!(LogFilter::All.allows(Provenance::Driver));
        assert!(LogFilter::All.allows(Provenance::Subject));
    }

    #[test]
    fn log_filter_driver_allows_only_the_driver() {
        assert!(LogFilter::Driver.allows(Provenance::Driver));
        assert!(!LogFilter::Driver.allows(Provenance::Subject));
    }

    fn a_line(millis: u64, provenance: Provenance, text: &str) -> LiveLogLine {
        LiveLogLine {
            elapsed: Duration::from_millis(millis),
            provenance,
            text: text.to_owned(),
        }
    }

    #[test]
    fn compose_log_region_is_empty_when_height_is_zero() {
        let lines = vec![a_line(0, Provenance::Driver, "algo")];
        assert_eq!(
            compose_log_region(&lines, 0, DEFAULT_LOG_WIDTH, LogFilter::All),
            ""
        );
    }

    #[test]
    fn compose_log_region_frames_the_lines_with_provenance_and_clock() {
        let lines = vec![
            a_line(310, Provenance::Driver, "launch afirma://websocket?v=1"),
            a_line(420, Provenance::Subject, "INFO ProtocolInvocationLauncher"),
        ];
        let region = compose_log_region(&lines, 8, DEFAULT_LOG_WIDTH, LogFilter::All);
        assert!(region.contains("registro"));
        assert!(region.contains("conductor · sujeto"));
        assert!(region.contains("00:00.31 conductor"));
        assert!(region.contains("00:00.42 sujeto"));
        assert!(region.starts_with('┌'));
        assert!(region.ends_with('┘'));
    }

    #[test]
    fn compose_log_region_keeps_only_the_last_lines_up_to_height() {
        let lines: Vec<LiveLogLine> = (0..5)
            .map(|n| a_line(n * 100, Provenance::Driver, &format!("linea {n}")))
            .collect();
        let region = compose_log_region(&lines, 2, DEFAULT_LOG_WIDTH, LogFilter::All);
        assert!(!region.contains("linea 0"));
        assert!(!region.contains("linea 2"));
        assert!(region.contains("linea 3"));
        assert!(region.contains("linea 4"));
    }

    #[test]
    fn compose_log_region_truncates_a_line_longer_than_the_width() {
        let long_text = "x".repeat(200);
        let lines = vec![a_line(0, Provenance::Driver, &long_text)];
        let region = compose_log_region(&lines, 8, DEFAULT_LOG_WIDTH, LogFilter::All);
        assert!(region.contains('…'));
        for line in region.lines() {
            assert!(line.chars().count() <= DEFAULT_LOG_WIDTH);
        }
    }

    #[test]
    fn compose_log_region_names_the_active_filter() {
        let lines = vec![a_line(0, Provenance::Subject, "algo")];
        let region = compose_log_region(&lines, 8, DEFAULT_LOG_WIDTH, LogFilter::Subject);
        assert!(region.contains("[sujeto]"));
    }

    #[test]
    fn live_log_sink_drops_the_oldest_line_past_its_height() {
        let sink = LiveLogSink::new(LogFilter::All, 2);
        sink.push(Provenance::Driver, Duration::ZERO, "uno".to_owned());
        sink.push(Provenance::Driver, Duration::ZERO, "dos".to_owned());
        sink.push(Provenance::Driver, Duration::ZERO, "tres".to_owned());
        let snapshot = sink.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].text, "dos");
        assert_eq!(snapshot[1].text, "tres");
    }

    #[test]
    fn live_log_sink_ignores_a_provenance_the_filter_excludes() {
        let sink = LiveLogSink::new(LogFilter::Driver, 8);
        sink.push(Provenance::Subject, Duration::ZERO, "descartada".to_owned());
        assert!(sink.snapshot().is_empty());
    }

    #[test]
    fn live_log_sink_stores_nothing_when_height_is_zero() {
        let sink = LiveLogSink::new(LogFilter::All, 0);
        sink.push(Provenance::Driver, Duration::ZERO, "algo".to_owned());
        assert!(sink.snapshot().is_empty());
    }

    #[test]
    fn check_log_writes_the_same_format_as_the_live_region() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a_check.log");
        let log = CheckLog::open(&path).unwrap();
        log.record(Duration::from_millis(310), Provenance::Driver, "launch");
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written, "00:00.31 conductor launch\n");
    }
}
