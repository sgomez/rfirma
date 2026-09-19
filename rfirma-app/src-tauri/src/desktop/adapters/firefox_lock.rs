//! Detección de Firefox vivo por el cerrojo POSIX de `.parentlock` en su perfil.

use std::os::unix::io::AsRawFd;
use std::path::Path;

/// Indica si Firefox tiene abierto el perfil dado, por el bloqueo POSIX de `.parentlock`.
pub fn firefox_is_running(profile: &Path) -> bool {
    let Ok(file) = std::fs::File::open(profile.join(".parentlock")) else {
        return false;
    };
    someone_else_holds_the_write_lock(file.as_raw_fd())
}

fn someone_else_holds_the_write_lock(fd: i32) -> bool {
    let mut lock = libc::flock {
        l_type: libc::F_WRLCK as _,
        l_whence: libc::SEEK_SET as _,
        l_start: 0,
        l_len: 0,
        l_pid: 0,
    };
    let queried = unsafe { libc::fcntl(fd, libc::F_GETLK, &mut lock) };
    queried == 0 && lock.l_type as i32 != libc::F_UNLCK
}

#[cfg(test)]
mod tests;
