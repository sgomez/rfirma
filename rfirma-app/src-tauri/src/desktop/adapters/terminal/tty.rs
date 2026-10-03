//! El secreto tecleado y el certificado elegido en la terminal que controla el proceso, sin pasar por stdin ni stdout.

use crate::desktop::ports::OfferedCertificate;
use crate::identity::domain::protected_secret::ProtectedSecret;
#[cfg(unix)]
use picker::{height_for, Picker, Step};

#[cfg(any(unix, test))]
mod picker;

#[cfg(unix)]
pub fn typed_without_echo(prompt: &str) -> Result<ProtectedSecret, String> {
    use std::io::Write;
    use std::os::fd::AsRawFd;

    let mut tty = asked_on_tty(prompt)?;
    let echo = WithoutEcho::on(tty.as_raw_fd())?;
    let typed = typed_line(&mut tty);
    drop(echo);
    let _ = tty.write_all(b"\n");
    typed
}

#[cfg(unix)]
fn asked_on_tty(prompt: &str) -> Result<std::fs::File, String> {
    use std::io::Write;

    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|error| format!("no se puede abrir la terminal ({error})"))?;
    tty.write_all(prompt.as_bytes())
        .and_then(|()| tty.flush())
        .map_err(|error| format!("no se puede escribir en la terminal ({error})"))?;
    Ok(tty)
}

/// La línea leída hasta el salto, sin el salto ni el retorno de carro que lo preceda.
#[cfg(any(unix, test))]
fn typed_line(input: &mut impl std::io::Read) -> Result<ProtectedSecret, String> {
    use zeroize::Zeroize;

    let mut typed = Vec::new();
    let mut byte = [0u8; 1];
    let read = loop {
        match input.read(&mut byte) {
            Ok(0) => break Err("no se ha tecleado nada".to_owned()),
            Ok(_) if byte[0] == b'\n' => break Ok(()),
            Ok(_) => typed.push(byte[0]),
            Err(error) => break Err(format!("no se puede leer de la terminal ({error})")),
        }
    };
    byte.zeroize();
    if typed.last() == Some(&b'\r') {
        typed.pop();
    }
    let secret = ProtectedSecret::new(&typed);
    typed.zeroize();
    read.map(|()| secret)
}

/// La posición elegida en la lista, que empieza en `preselected`, dibujada bajo el prompt.
#[cfg(unix)]
pub fn chosen_on_tty(offered: &[OfferedCertificate], preselected: usize) -> Result<usize, String> {
    use ratatui::backend::CrosstermBackend;
    use ratatui::crossterm::event::{self, Event, KeyEventKind};
    use ratatui::{Terminal, TerminalOptions, Viewport};
    use std::io::Write;

    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|error| format!("no se puede abrir la terminal ({error})"))?;
    let mut summary = tty.try_clone().map_err(unusable)?;
    let raw = RawMode::on()?;
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(tty),
        TerminalOptions {
            viewport: Viewport::Inline(height_for(offered.len())),
        },
    )
    .map_err(unusable)?;
    let mut picker = Picker::new(offered, preselected);
    let step = loop {
        terminal
            .draw(|frame| picker.draw(frame))
            .map_err(unusable)?;
        if let Event::Key(key) = event::read().map_err(unusable)? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match picker.on(key) {
                Step::Pending => {}
                step => break step,
            }
        }
    };
    let origin = terminal.get_frame().area().as_position();
    terminal.clear().map_err(unusable)?;
    terminal.set_cursor_position(origin).map_err(unusable)?;
    terminal.show_cursor().map_err(unusable)?;
    drop(terminal);
    drop(raw);
    match step {
        Step::Chosen(index) => {
            let _ = writeln!(summary, "Certificado: {}", offered[index].headline);
            Ok(index)
        }
        _ => Err("se ha cancelado la elección".to_owned()),
    }
}

#[cfg(unix)]
fn unusable(error: impl std::fmt::Display) -> String {
    format!("no se puede usar la terminal ({error})")
}

/// El modo crudo de la terminal que controla el proceso, devuelto a su estado al soltarlo.
#[cfg(unix)]
struct RawMode;

#[cfg(unix)]
impl RawMode {
    fn on() -> Result<Self, String> {
        ratatui::crossterm::terminal::enable_raw_mode().map_err(unusable)?;
        Ok(Self)
    }
}

#[cfg(unix)]
impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = ratatui::crossterm::terminal::disable_raw_mode();
    }
}

#[cfg(unix)]
struct WithoutEcho {
    fd: std::os::fd::RawFd,
    before: libc::termios,
}

#[cfg(unix)]
impl WithoutEcho {
    fn on(fd: std::os::fd::RawFd) -> Result<Self, String> {
        let mut before = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: `fd` es la terminal abierta y `before` tiene el tamaño que pide tcgetattr.
        if unsafe { libc::tcgetattr(fd, before.as_mut_ptr()) } != 0 {
            return Err("no se puede apagar el eco de la terminal".to_owned());
        }
        // SAFETY: tcgetattr ha devuelto 0, así que ha rellenado la estructura.
        let before = unsafe { before.assume_init() };
        let mut silent = before;
        silent.c_lflag &= !libc::ECHO;
        // SAFETY: `silent` es una copia válida de la configuración que acaba de leerse.
        if unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &silent) } != 0 {
            return Err("no se puede apagar el eco de la terminal".to_owned());
        }
        Ok(Self { fd, before })
    }
}

#[cfg(unix)]
impl Drop for WithoutEcho {
    fn drop(&mut self) {
        // SAFETY: devuelve a la terminal, aún abierta, la configuración que tenía.
        unsafe { libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.before) };
    }
}

#[cfg(not(unix))]
pub fn typed_without_echo(_prompt: &str) -> Result<ProtectedSecret, String> {
    Err("pedir el PIN en la terminal todavía no está disponible en este sistema".to_owned())
}

#[cfg(not(unix))]
pub fn chosen_on_tty(
    _offered: &[OfferedCertificate],
    _preselected: usize,
) -> Result<usize, String> {
    Err(
        "elegir el certificado en la terminal todavía no está disponible en este sistema"
            .to_owned(),
    )
}

#[cfg(test)]
mod tests;
