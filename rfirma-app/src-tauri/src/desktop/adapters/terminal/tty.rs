//! El secreto tecleado y el certificado elegido en la terminal que controla el proceso, sin pasar por stdin ni stdout.

use crate::identity::domain::protected_secret::ProtectedSecret;

#[cfg(unix)]
pub fn typed_without_echo(prompt: &str) -> Result<ProtectedSecret, String> {
    use std::io::Write;
    use std::os::fd::AsRawFd;

    let mut tty = asked_on_tty(prompt)?;
    let echo = WithoutEcho::on(tty.as_raw_fd(), libc::ECHO)?;
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

/// La posición elegida con las flechas en una lista que empieza en `preselected`.
#[cfg(unix)]
pub fn chosen_on_tty(lines: &[String], preselected: usize) -> Result<usize, String> {
    use std::os::fd::AsRawFd;

    let tty = asked_on_tty(CHOOSE)?;
    let keys = WithoutEcho::on(tty.as_raw_fd(), libc::ECHO | libc::ICANON | libc::ISIG)?;
    let chosen = picked(&mut &tty, &mut &tty, lines, preselected);
    drop(keys);
    chosen
}

#[cfg(any(unix, test))]
const CHOOSE: &str = "Elige el certificado con ↑ y ↓ e Intro; q cancela:\n";

#[cfg(any(unix, test))]
fn picked(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    lines: &[String],
    preselected: usize,
) -> Result<usize, String> {
    let last = lines
        .len()
        .checked_sub(1)
        .ok_or_else(|| "no hay nada que elegir".to_owned())?;
    let mut selected = preselected.min(last);
    shown(output, &menu(lines, selected))?;
    loop {
        selected = match key_of(input)? {
            Key::Enter => return Ok(selected),
            Key::Cancel => return Err("se ha cancelado la elección".to_owned()),
            Key::Up => selected.saturating_sub(1),
            Key::Down => (selected + 1).min(last),
            Key::Other => continue,
        };
        shown(
            output,
            &format!("\x1b[{}A{}", lines.len(), menu(lines, selected)),
        )?;
    }
}

#[cfg(any(unix, test))]
fn menu(lines: &[String], selected: usize) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == selected {
                format!("\r\x1b[2K\x1b[7m> {line}\x1b[0m\n")
            } else {
                format!("\r\x1b[2K  {line}\n")
            }
        })
        .collect()
}

#[cfg(any(unix, test))]
fn shown(output: &mut impl std::io::Write, text: &str) -> Result<(), String> {
    output
        .write_all(text.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|error| format!("no se puede escribir en la terminal ({error})"))
}

#[cfg(any(unix, test))]
#[derive(Debug, PartialEq, Eq)]
enum Key {
    Up,
    Down,
    Enter,
    Cancel,
    Other,
}

#[cfg(any(unix, test))]
fn key_of(input: &mut impl std::io::Read) -> Result<Key, String> {
    Ok(match byte_of(input)? {
        b'\r' | b'\n' => Key::Enter,
        0x03 | 0x04 | b'q' => Key::Cancel,
        b'k' => Key::Up,
        b'j' => Key::Down,
        0x1b => escaped(input)?,
        _ => Key::Other,
    })
}

/// La flecha de una secuencia de escape, en su forma CSI (`ESC [ A`) o SS3 (`ESC O A`).
#[cfg(any(unix, test))]
fn escaped(input: &mut impl std::io::Read) -> Result<Key, String> {
    if !matches!(byte_of(input)?, b'[' | b'O') {
        return Ok(Key::Other);
    }
    Ok(match byte_of(input)? {
        b'A' => Key::Up,
        b'B' => Key::Down,
        _ => Key::Other,
    })
}

#[cfg(any(unix, test))]
fn byte_of(input: &mut impl std::io::Read) -> Result<u8, String> {
    let mut byte = [0u8; 1];
    match input.read(&mut byte) {
        Ok(0) => Err("la terminal se ha cerrado sin elegir".to_owned()),
        Ok(_) => Ok(byte[0]),
        Err(error) => Err(format!("no se puede leer de la terminal ({error})")),
    }
}

#[cfg(unix)]
struct WithoutEcho {
    fd: std::os::fd::RawFd,
    before: libc::termios,
}

#[cfg(unix)]
impl WithoutEcho {
    fn on(fd: std::os::fd::RawFd, cleared: libc::tcflag_t) -> Result<Self, String> {
        let mut before = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: `fd` es la terminal abierta y `before` tiene el tamaño que pide tcgetattr.
        if unsafe { libc::tcgetattr(fd, before.as_mut_ptr()) } != 0 {
            return Err("no se puede apagar el eco de la terminal".to_owned());
        }
        // SAFETY: tcgetattr ha devuelto 0, así que ha rellenado la estructura.
        let before = unsafe { before.assume_init() };
        let mut silent = before;
        silent.c_lflag &= !cleared;
        if cleared & libc::ICANON != 0 {
            silent.c_cc[libc::VMIN] = 1;
            silent.c_cc[libc::VTIME] = 0;
        }
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
pub fn chosen_on_tty(_lines: &[String], _preselected: usize) -> Result<usize, String> {
    Err(
        "elegir el certificado en la terminal todavía no está disponible en este sistema"
            .to_owned(),
    )
}

#[cfg(test)]
mod tests;
