use std::ffi::CString;
use std::time::Duration;

use super::*;

#[test]
fn a_profile_with_no_lock_file_is_not_running() {
    let profile = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    assert!(!firefox_is_running(profile.path()));
}

#[test]
fn a_profile_whose_lock_file_no_process_holds_is_not_running() {
    let profile = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    std::fs::write(profile.path().join(".parentlock"), b"")
        .expect("deberia poder crearse el fichero de cerrojo");

    assert!(!firefox_is_running(profile.path()));
}

/// Bloquea `.parentlock` desde un proceso real, como hace Firefox, para que
/// `F_GETLK` encuentre un cerrojo de otro proceso y no del propio.
#[test]
fn a_profile_whose_lock_file_another_process_holds_is_running() {
    let profile = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let lock_path = profile.path().join(".parentlock");
    std::fs::write(&lock_path, b"").expect("deberia poder crearse el fichero de cerrojo");
    let lock_path = CString::new(lock_path.to_str().expect("ruta UTF-8").as_bytes())
        .expect("ruta sin ceros intermedios");

    let child = unsafe { libc::fork() };
    assert!(child >= 0, "el fork deberia poder crearse");

    if child == 0 {
        let fd = unsafe { libc::open(lock_path.as_ptr(), libc::O_RDWR) };
        if fd >= 0 {
            let mut lock = libc::flock {
                l_type: libc::F_WRLCK as _,
                l_whence: libc::SEEK_SET as _,
                l_start: 0,
                l_len: 0,
                l_pid: 0,
            };
            unsafe { libc::fcntl(fd, libc::F_SETLK, &mut lock) };
        }
        loop {
            unsafe { libc::pause() };
        }
    }

    std::thread::sleep(Duration::from_millis(200));
    let running_while_held = firefox_is_running(profile.path());

    unsafe { libc::kill(child, libc::SIGKILL) };
    let mut status = 0;
    unsafe { libc::waitpid(child, &mut status, 0) };

    let running_after_the_process_died = firefox_is_running(profile.path());

    assert!(
        running_while_held,
        "el cerrojo de otro proceso debe detectarse"
    );
    assert!(
        !running_after_the_process_died,
        "el cerrojo debe liberarse con el proceso"
    );
}
