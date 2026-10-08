//! Lo que la tarjeta guarda entre procesos, en el directorio de su copia del módulo: contador, claves y registro.

use std::ffi::{c_void, CStr};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub(crate) const PIN: &str = "12345678";
pub(crate) const MAX_TRIES: u8 = 3;
pub(crate) const TRIES_LEFT: &str = "tries-left";
pub(crate) const CALL_LOG: &str = "calls.log";
pub(crate) const PROFILE: &str = "profile";
pub(crate) const INTERFERENCE: &str = "interference";
pub(crate) const REMOVED_FROM: &str = "removed-from";

/// Qué tarjeta imita el módulo: el DNIe medido, o una que da las tres señales del PIN y exige el login por firma.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Profile {
    Dnie,
    Signals,
}

impl Profile {
    pub(crate) const SIGNALS_NAME: &'static str = "signals";
}

pub(crate) fn read_profile(dir: &Path) -> Profile {
    match fs::read_to_string(dir.join(PROFILE)) {
        Ok(name) if name.trim() == Profile::SIGNALS_NAME => Profile::Signals,
        _ => Profile::Dnie,
    }
}

pub(crate) fn interference_configured(dir: &Path) -> bool {
    dir.join(INTERFERENCE).is_file()
}

/// La llamada, contada desde 1, a partir de la cual la tarjeta ya no está.
fn removed_from_call(dir: &Path) -> Option<usize> {
    fs::read_to_string(dir.join(REMOVED_FROM))
        .ok()
        .and_then(|text| text.trim().parse().ok())
}

/// Si la llamada que llega ahora es la de la retirada o una posterior.
pub(crate) fn card_removed(dir: &Path) -> bool {
    removed_from_call(dir).is_some_and(|from| calls_received(dir) + 1 >= from)
}

fn calls_received(dir: &Path) -> usize {
    fs::read_to_string(dir.join(CALL_LOG)).map_or(0, |log| log.lines().count())
}

pub(crate) fn read_tries_left(dir: &Path) -> u8 {
    fs::read_to_string(dir.join(TRIES_LEFT))
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(MAX_TRIES)
}

pub(crate) fn write_tries_left(dir: &Path, tries: u8) {
    let _ = fs::write(dir.join(TRIES_LEFT), tries.to_string());
}

pub(crate) fn record_call(dir: &Path, line: &str) {
    if let Ok(mut log) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(CALL_LOG))
    {
        let _ = writeln!(log, "{line}");
    }
}

/// El directorio de la copia del módulo que se ha cargado: cada copia es una tarjeta.
pub(crate) fn card_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        loaded_from()
            .and_then(|module| module.parent().map(Path::to_path_buf))
            .unwrap_or_else(std::env::temp_dir)
    })
}

fn loaded_from() -> Option<PathBuf> {
    let mut info = libc::Dl_info {
        dli_fname: std::ptr::null(),
        dli_fbase: std::ptr::null_mut(),
        dli_sname: std::ptr::null(),
        dli_saddr: std::ptr::null_mut(),
    };
    let anchor = card_dir as fn() -> &'static Path as *const c_void;
    // SAFETY: `dladdr` solo escribe en `info`, y `dli_fname` apunta a una cadena del cargador.
    let found = unsafe { libc::dladdr(anchor, &mut info) };
    if found == 0 || info.dli_fname.is_null() {
        return None;
    }
    // SAFETY: comprobado arriba que no es nulo; el cargador la termina en cero.
    let name = unsafe { CStr::from_ptr(info.dli_fname) };
    Some(PathBuf::from(name.to_string_lossy().into_owned()))
}
