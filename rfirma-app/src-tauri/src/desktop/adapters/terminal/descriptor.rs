//! El secreto que sale de un descriptor abierto por quien llama, sin pasar por argv, entorno ni disco.

use crate::identity::domain::protected_secret::ProtectedSecret;

#[cfg(unix)]
pub fn read_from(descriptor: u32) -> Result<ProtectedSecret, String> {
    use std::os::fd::{FromRawFd, RawFd};

    let raw = RawFd::try_from(descriptor)
        .map_err(|_| format!("el descriptor {descriptor} no es válido"))?;
    // SAFETY: fcntl solo consulta las banderas; no toca el descriptor.
    if unsafe { libc::fcntl(raw, libc::F_GETFD) } == -1 {
        return Err(format!("el descriptor {descriptor} no está abierto"));
    }
    // SAFETY: está abierto y quien llama lo cedió con `-password-fd`; el descriptor no se cierra al soltar la lectura.
    let mut file = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(raw) });
    first_line(&mut *file).map_err(|reason| format!("el descriptor {descriptor} {reason}"))
}

#[cfg(not(unix))]
pub fn read_from(_descriptor: u32) -> Result<ProtectedSecret, String> {
    Err("--password-fd todavía no está disponible en este sistema".to_owned())
}

#[cfg(any(unix, test))]
const LONGEST_SECRET: u64 = 1024;

/// La primera línea, sin el salto ni el retorno de carro, aunque el final de la entrada la corte.
#[cfg(any(unix, test))]
fn first_line(input: &mut impl std::io::Read) -> Result<ProtectedSecret, String> {
    use zeroize::Zeroize;

    let mut read = Vec::new();
    let result = read_until_newline(input, &mut read);
    let line_end = read.iter().position(|byte| *byte == b'\n');
    let mut line = &read[..line_end.unwrap_or(read.len())];
    if let Some(without_return) = line.strip_suffix(b"\r") {
        line = without_return;
    }
    let secret = ProtectedSecret::new(line);
    let is_empty = line.is_empty();
    read.zeroize();
    if let Err(error) = result {
        return Err(format!("no se puede leer ({error})"));
    }
    if is_empty {
        return Err("está vacío".to_owned());
    }
    Ok(secret)
}

#[cfg(any(unix, test))]
fn read_until_newline(input: &mut impl std::io::Read, read: &mut Vec<u8>) -> std::io::Result<()> {
    let mut byte = [0u8; 1];
    while (read.len() as u64) < LONGEST_SECRET {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                read.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
