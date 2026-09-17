//! Monitor de terminal en tiempo real para la suite de conformidad.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};

use crate::catalogue::Check;
use crate::dossier::Header;
use crate::livelog::{compose_log_region, LiveLogSink, LogFilter, DEFAULT_LOG_WIDTH};
use crate::verdicts::{chapter_tag, format_badge};

pub(crate) struct ProgressMonitor {
    #[allow(dead_code)]
    plain: bool,
    interactive_tty: bool,
    active_item: Arc<Mutex<Option<ActiveItem>>>,
    running: Arc<AtomicBool>,
    ticker: Mutex<Option<JoinHandle<()>>>,
    log_sink: LiveLogSink,
    frame_lines: Arc<Mutex<usize>>,
}

struct ActiveItem {
    kind: String,
    current: usize,
    total: usize,
    name: String,
    start: Instant,
    suspended: bool,
}

pub(crate) fn format_header(subject: &str, header: &Header) -> String {
    format!(
        "Sondeo de compatibilidad\n  Sujeto:   {subject} (v{})\n  Sistema:  {} {}\n  Almacén:  {} ({})\n",
        header.subject_version, header.os, header.os_version, header.store, header.transport
    )
}

/// Lo que se lee antes de invocar al sujeto: qué comprobación es, de qué conjunto y de qué
/// capítulo, y su enunciado resumido — para saber qué se mide sin abrir el catálogo.
pub(crate) fn format_check_announcement(
    chapter: &str,
    suite: &str,
    id: &str,
    statement: &str,
) -> String {
    format!("\n{} {suite} · {id}\n  {statement}", chapter_tag(chapter))
}

pub(crate) fn render_dialog_box(prompt: &str, use_color: bool) -> String {
    let char_count = prompt.chars().count();
    let width = char_count.max(48);
    let border = "─".repeat(width + 4);
    let pad = " ".repeat(width - char_count);
    if use_color {
        format!(
            "\x1b[1;36m┌{border}┐\x1b[0m\n\
             \x1b[1;36m│\x1b[0m  \x1b[1m{prompt}\x1b[0m{pad}  \x1b[1;36m│\x1b[0m\n\
             \x1b[1;36m└{border}┘\x1b[0m"
        )
    } else {
        format!(
            "┌{border}┐\n\
             │  {prompt}{pad}  │\n\
             └{border}┘"
        )
    }
}

pub(crate) fn format_verdict_line(
    badge: &str,
    color: &str,
    name: &str,
    duration: Duration,
    observation: Option<&str>,
    use_color: bool,
) -> String {
    let badge_formatted = format_badge(badge, color, use_color);
    let duration_str = format!("{:.1}s", duration.as_secs_f32());
    if let Some(obs) = observation {
        let trimmed = obs.trim();
        if !trimmed.is_empty() && trimmed != "-" {
            format!("{badge_formatted} {name} ({duration_str}) - {trimmed}")
        } else {
            format!("{badge_formatted} {name} ({duration_str})")
        }
    } else {
        format!("{badge_formatted} {name} ({duration_str})")
    }
}

impl ProgressMonitor {
    #[allow(dead_code)]
    pub(crate) fn is_plain(&self) -> bool {
        self.plain
    }

    pub(crate) fn new(plain: bool, log_lines: usize, log_filter: LogFilter) -> Self {
        let is_tty = !plain && std::io::stdout().is_terminal();
        Self {
            plain,
            interactive_tty: is_tty,
            active_item: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(true)),
            ticker: Mutex::new(None),
            log_sink: LiveLogSink::new(log_filter, log_lines),
            frame_lines: Arc::new(Mutex::new(0)),
        }
    }

    /// El extremo por el que el conductor y el sujeto entregan sus líneas, desde sus propios
    /// hilos.
    pub(crate) fn log_sink(&self) -> LiveLogSink {
        self.log_sink.clone()
    }

    pub(crate) fn display_header(&self, subject: &str, header: &Header) {
        println!("{}", format_header(subject, header));
        let _ = std::io::stdout().flush();
    }

    pub(crate) fn announce_check(&self, check: &Check) {
        println!(
            "{}",
            format_check_announcement(&check.chapter, &check.suite, &check.id, &check.statement)
        );
        let _ = std::io::stdout().flush();
    }

    pub(crate) fn start_progress(&self, kind: &str, current: usize, total: usize, name: &str) {
        let mut lock = self.active_item.lock().unwrap();
        *lock = Some(ActiveItem {
            kind: kind.to_owned(),
            current,
            total,
            name: name.to_owned(),
            start: Instant::now(),
            suspended: false,
        });
        self.log_sink.clear();

        if self.interactive_tty {
            self.ensure_ticker();
        }
    }

    fn ensure_ticker(&self) {
        let mut ticker_lock = self.ticker.lock().unwrap();
        if ticker_lock.is_some() {
            return;
        }
        let running = Arc::clone(&self.running);
        let active = Arc::clone(&self.active_item);
        let log_sink = self.log_sink.clone();
        let frame_lines = Arc::clone(&self.frame_lines);
        let handle = spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            while running.load(Ordering::SeqCst) {
                {
                    if let Ok(item_lock) = active.lock() {
                        if let Some(ref item) = *item_lock {
                            if !item.suspended {
                                let glyph = frames[i % frames.len()];
                                i = i.wrapping_add(1);
                                let elapsed = item.start.elapsed().as_secs_f32();
                                let spinner_line = format!(
                                    "{} [{} {}/{}] {}... ({:.1}s)",
                                    glyph, item.kind, item.current, item.total, item.name, elapsed
                                );
                                let log_region = compose_log_region(
                                    &log_sink.snapshot(),
                                    log_sink.height(),
                                    DEFAULT_LOG_WIDTH,
                                    log_sink.filter(),
                                );
                                let rendered_frame = if log_region.is_empty() {
                                    spinner_line
                                } else {
                                    format!("{spinner_line}\n{log_region}")
                                };
                                repaint_frame(&frame_lines, &rendered_frame);
                            }
                        }
                    }
                }
                sleep(Duration::from_millis(80));
            }
        });
        *ticker_lock = Some(handle);
    }

    pub(crate) fn finish_item(
        &self,
        badge: &str,
        color: &str,
        name: &str,
        duration: Duration,
        observation: Option<&str>,
    ) {
        {
            let mut lock = self.active_item.lock().unwrap();
            *lock = None;
        }
        if self.interactive_tty {
            clear_frame(&self.frame_lines);
        }
        let line = format_verdict_line(
            badge,
            color,
            name,
            duration,
            observation,
            self.interactive_tty,
        );
        println!("{line}");
        let _ = std::io::stdout().flush();
    }

    pub(crate) fn suspend(&self) {
        let mut lock = self.active_item.lock().unwrap();
        if let Some(ref mut item) = *lock {
            item.suspended = true;
        }
        if self.interactive_tty {
            clear_frame(&self.frame_lines);
        }
    }

    pub(crate) fn resume(&self) {
        let mut lock = self.active_item.lock().unwrap();
        if let Some(ref mut item) = *lock {
            item.suspended = false;
        }
    }

    pub(crate) fn ask(&self, prompt: &str) -> String {
        self.suspend();
        let is_tty = self.interactive_tty && std::io::stdin().is_terminal();
        let box_str = render_dialog_box(prompt, is_tty);
        println!("\n{box_str}");
        print!("> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("no pude leer de teclado");
        let result = line.trim().to_owned();
        self.resume();
        result
    }
}

impl Drop for ProgressMonitor {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.ticker.lock().unwrap().take() {
            let _ = handle.join();
        }
        if self.interactive_tty {
            clear_frame(&self.frame_lines);
        }
    }
}

/// Redibuja `frame` donde estaba el anterior, sin desplazar lo que ya se dio por resuelto.
fn repaint_frame(frame_lines: &Arc<Mutex<usize>>, frame: &str) {
    let mut previous = frame_lines.lock().unwrap();
    let mut out = std::io::stdout();
    if *previous > 0 {
        let _ = write!(out, "\x1b[{previous}F\x1b[0J");
    }
    let _ = writeln!(out, "{frame}");
    let _ = out.flush();
    *previous = frame.lines().count();
}

/// Borra la última región dibujada por [`repaint_frame`], sin dejar nada en su lugar.
fn clear_frame(frame_lines: &Arc<Mutex<usize>>) {
    let mut previous = frame_lines.lock().unwrap();
    if *previous > 0 {
        print!("\x1b[{previous}F\x1b[0J");
        let _ = std::io::stdout().flush();
    }
    *previous = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_dialog_box_frames_prompt_cleanly_without_color() {
        let prompt = "¿se pidió elegir dónde guardar el fichero? [s/n]";
        let rendered = render_dialog_box(prompt, false);
        assert!(rendered.contains("┌"));
        assert!(rendered.contains("┐"));
        assert!(rendered.contains("└"));
        assert!(rendered.contains("┘"));
        assert!(rendered.contains(prompt));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    fn render_dialog_box_includes_ansi_color_when_enabled() {
        let prompt = "¿se pidió elegir dónde guardar el fichero? [s/n]";
        let rendered = render_dialog_box(prompt, true);
        assert!(rendered.contains("\x1b[1;36m"));
        assert!(rendered.contains("\x1b[0m"));
        assert!(rendered.contains(prompt));
    }

    #[test]
    fn format_check_announcement_names_id_suite_chapter_and_statement() {
        let announcement =
            format_check_announcement("15", "errores", "an_identifier", "Un enunciado resumido.");
        assert!(announcement.contains("[Cap. 15]"));
        assert!(announcement.contains("errores"));
        assert!(announcement.contains("an_identifier"));
        assert!(announcement.contains("Un enunciado resumido."));
    }

    #[test]
    fn format_header_includes_all_coordinates() {
        let header = Header {
            os: "Linux".to_string(),
            os_version: "6.8.0".to_string(),
            subject_version: "1.8.2".to_string(),
            transport: "websocket".to_string(),
            store: "rfirma-test".to_string(),
            date: "2026-09-17".to_string(),
        };
        let formatted = format_header("/usr/bin/autofirma", &header);
        assert!(formatted.contains("Sondeo de compatibilidad"));
        assert!(formatted.contains("/usr/bin/autofirma"));
        assert!(formatted.contains("v1.8.2"));
        assert!(formatted.contains("Linux 6.8.0"));
        assert!(formatted.contains("rfirma-test"));
        assert!(formatted.contains("websocket"));
    }

    #[test]
    fn verdict_line_formats_cleanly_without_color() {
        let line = format_verdict_line(
            "[NO CONFORME]",
            "\x1b[31m",
            "an_identifier",
            Duration::from_millis(1200),
            Some("detalles"),
            false,
        );
        assert_eq!(line, "[NO CONFORME]   an_identifier (1.2s) - detalles");
        assert!(!line.contains("\x1b["));
    }

    #[test]
    fn verdict_line_omits_dash_or_empty_observation() {
        let line = format_verdict_line(
            "[CONFORME]",
            "\x1b[32m",
            "another_identifier",
            Duration::from_millis(300),
            Some("-"),
            false,
        );
        assert_eq!(line, "[CONFORME]      another_identifier (0.3s)");
    }
    #[test]
    fn plain_monitor_reports_is_plain() {
        let monitor = ProgressMonitor::new(true, 8, LogFilter::All);
        assert!(monitor.is_plain());
    }
}
