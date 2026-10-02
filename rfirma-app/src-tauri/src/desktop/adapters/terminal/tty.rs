//! El secreto tecleado en la terminal que controla el proceso, sin eco y sin pasar por stdin ni stdout.

use crate::identity::domain::protected_secret::ProtectedSecret;

#[cfg(unix)]
pub fn typed_without_echo(prompt: &str) -> Result<ProtectedSecret, String> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use zeroize::Zeroize;

    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|error| format!("no se puede abrir la terminal ({error})"))?;
    tty.write_all(prompt.as_bytes())
        .and_then(|()| tty.flush())
        .map_err(|error| format!("no se puede escribir en la terminal ({error})"))?;
    let echo = WithoutEcho::on(tty.as_raw_fd())?;
    let mut typed = Vec::new();
    let mut byte = [0u8; 1];
    let read = loop {
        match tty.read(&mut byte) {
            Ok(0) => break Err("no se ha tecleado nada".to_owned()),
            Ok(_) if byte[0] == b'\n' => break Ok(()),
            Ok(_) => typed.push(byte[0]),
            Err(error) => break Err(format!("no se puede leer de la terminal ({error})")),
        }
    };
    byte.zeroize();
    drop(echo);
    let _ = tty.write_all(b"\n");
    if typed.last() == Some(&b'\r') {
        typed.pop();
    }
    let secret = ProtectedSecret::new(&typed);
    typed.zeroize();
    read.map(|()| secret)
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
