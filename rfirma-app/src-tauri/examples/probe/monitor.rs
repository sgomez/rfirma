//! Monitor de terminal en tiempo real para el sondeo de compatibilidad.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};

use crate::dossier::Header;
use crate::verdicts::format_badge;

pub(crate) struct ProgressMonitor {
    #[allow(dead_code)]
    plain: bool,
    interactive_tty: bool,
    active_item: Arc<Mutex<Option<ActiveItem>>>,
    running: Arc<AtomicBool>,
    ticker: Mutex<Option<JoinHandle<()>>>,
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

    pub(crate) fn new(plain: bool) -> Self {
        let is_tty = !plain && std::io::stdout().is_terminal();
        Self {
            plain,
            interactive_tty: is_tty,
            active_item: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(true)),
            ticker: Mutex::new(None),
        }
    }

    pub(crate) fn display_header(&self, subject: &str, header: &Header) {
        println!("{}", format_header(subject, header));
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
        let handle = spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            while running.load(Ordering::SeqCst) {
                {
                    if let Ok(item_lock) = active.lock() {
                        if let Some(ref item) = *item_lock {
                            if !item.suspended {
                                let frame = frames[i % frames.len()];
                                i = i.wrapping_add(1);
                                let elapsed = item.start.elapsed().as_secs_f32();
                                print!(
                                    "\r\x1b[2K{} [{} {}/{}] {}... ({:.1}s)",
                                    frame, item.kind, item.current, item.total, item.name, elapsed
                                );
                                let _ = std::io::stdout().flush();
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
            print!("\r\x1b[2K");
            let _ = std::io::stdout().flush();
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
            print!("\r\x1b[2K");
            let _ = std::io::stdout().flush();
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
            print!("\r\x1b[2K");
            let _ = std::io::stdout().flush();
        }
    }
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
            "[CONFIRMADO]",
            "\x1b[32m",
            "test_case",
            Duration::from_millis(1200),
            Some("detalles"),
            false,
        );
        assert_eq!(line, "[CONFIRMADO]    test_case (1.2s) - detalles");
        assert!(!line.contains("\x1b["));
    }

    #[test]
    fn verdict_line_omits_dash_or_empty_observation() {
        let line = format_verdict_line(
            "[CONFORME]",
            "\x1b[32m",
            "condition_1",
            Duration::from_millis(300),
            Some("-"),
            false,
        );
        assert_eq!(line, "[CONFORME]      condition_1 (0.3s)");
    }
    #[test]
    fn plain_monitor_reports_is_plain() {
        let monitor = ProgressMonitor::new(true);
        assert!(monitor.is_plain());
    }
}
