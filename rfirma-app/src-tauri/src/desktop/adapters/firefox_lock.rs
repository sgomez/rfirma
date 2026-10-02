//! Detección de Firefox vivo por el cerrojo de su perfil: `.parentlock` en Unix, `parent.lock` en Windows.

#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::Path;

/// Indica si Firefox tiene abierto el perfil dado: mientras vive, tiene `parent.lock` abierto sin compartir.
#[cfg(windows)]
pub fn firefox_is_running(profile: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;

    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_SHARING_VIOLATION: i32 = 32;

    let opened = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(profile.join("parent.lock"));
    matches!(
        opened.map_err(|error| error.raw_os_error()),
        Err(Some(ERROR_SHARING_VIOLATION | ERROR_ACCESS_DENIED))
    )
}

/// Indica si Firefox tiene abierto el perfil dado, por el bloqueo POSIX de `.parentlock`.
#[cfg(unix)]
pub fn firefox_is_running(profile: &Path) -> bool {
    let Ok(file) = std::fs::File::open(profile.join(".parentlock")) else {
        return false;
    };
    someone_else_holds_the_write_lock(file.as_raw_fd())
}

#[cfg(unix)]
fn someone_else_holds_the_write_lock(fd: i32) -> bool {
    let mut lock = libc::flock {
        l_type: libc::F_WRLCK as _,
        l_whence: libc::SEEK_SET as _,
        l_start: 0,
        l_len: 0,
        l_pid: 0,
    };
    let queried = unsafe { libc::fcntl(fd, libc::F_GETLK, &mut lock) };
    queried == 0 && i64::from(lock.l_type) != i64::from(libc::F_UNLCK)
}

#[cfg(all(test, unix))]
mod tests;

#[cfg(all(test, windows))]
#[path = "firefox_lock/windows_tests.rs"]
mod windows_tests;
