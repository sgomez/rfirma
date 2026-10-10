//! El sondeo de un módulo PKCS#11 para el informe de depuración: carga, `C_Initialize` y `C_GetInfo` con un límite de tiempo, sin tocar ranuras ni tarjetas.

use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::error::{Error, RvError};

/// Lo que la biblioteca declara de sí misma en `C_GetInfo`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryInfo {
    /// El fabricante, sin el relleno de espacios.
    pub manufacturer: String,
    /// La versión de la biblioteca, `mayor.menor`.
    pub version: String,
}

/// Cómo ha respondido un módulo al sondeo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleProbe {
    /// Carga, se inicializa y dice quién es.
    Loads(LibraryInfo),
    /// No se carga, no se inicializa o no responde a `C_GetInfo`.
    DoesNotLoad,
    /// No ha terminado dentro del límite.
    NotResponding,
}

/// Sondea `module` en un hilo aparte y deja de esperarlo pasado `limit`.
pub fn probe(module: &Path, limit: Duration) -> ModuleProbe {
    let (sender, receiver) = mpsc::channel();
    let module = module.to_path_buf();
    let spawned = thread::Builder::new()
        .name("pkcs11-probe".to_owned())
        .spawn(move || {
            let _ = sender.send(library_info(&module));
        });
    if spawned.is_err() {
        return ModuleProbe::DoesNotLoad;
    }
    match receiver.recv_timeout(limit) {
        Ok(Some(info)) => ModuleProbe::Loads(info),
        Ok(None) | Err(RecvTimeoutError::Disconnected) => ModuleProbe::DoesNotLoad,
        Err(RecvTimeoutError::Timeout) => ModuleProbe::NotResponding,
    }
}

fn library_info(module: &Path) -> Option<LibraryInfo> {
    let context = Pkcs11::new(module).ok()?;
    let finalize = match context.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK)) {
        Ok(()) => true,
        Err(Error::Pkcs11(RvError::CryptokiAlreadyInitialized, _)) => false,
        Err(_) => return None,
    };
    let info = context.get_library_info().ok().map(|info| LibraryInfo {
        manufacturer: info.manufacturer_id().trim().to_owned(),
        version: info.library_version().to_string(),
    });
    if finalize {
        let _ = context.finalize();
    }
    info
}

#[cfg(test)]
mod tests;
