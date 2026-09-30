//! El instalador de la versión anunciada detrás de `UpdateInstaller`: el plugin updater en Windows, «no disponible» en el resto (ADR-0035).

use crate::desktop::domain::installation::InstallFailure;
use crate::desktop::ports::UpdateInstaller;

/// La clave del plugin en `plugins` de la configuración de Tauri.
const UPDATER: &str = "updater";

/// Registra el plugin updater, solo en Windows y solo si la configuración de Windows lo trae.
pub fn with_the_updater(
    builder: tauri::Builder<tauri::Wry>,
    config: &tauri::Config,
) -> tauri::Builder<tauri::Wry> {
    if !configured(config) {
        return builder;
    }
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
}

/// El instalador de esta máquina: el plugin en Windows con su configuración, y si no, ninguno.
pub fn this_desktop(app: tauri::AppHandle) -> Box<dyn UpdateInstaller> {
    #[cfg(windows)]
    if configured(app.config()) {
        return Box::new(plugin::PluginInstaller::new(app));
    }
    let _ = app;
    Box::new(NotAvailable)
}

/// Sin la configuración de Windows que pasa `--config`, `just dev` y `cargo test` no la traen (ADR-0035).
fn configured(config: &tauri::Config) -> bool {
    config.plugins.0.contains_key(UPDATER)
}

/// El canal que no instala desde la aplicación.
pub struct NotAvailable;

impl UpdateInstaller for NotAvailable {
    fn announced(&self) -> Result<Option<String>, InstallFailure> {
        Err(InstallFailure::NotAvailable)
    }

    fn install(&self) -> Result<(), InstallFailure> {
        Err(InstallFailure::NotAvailable)
    }
}

#[cfg(windows)]
mod plugin {
    use std::sync::Mutex;

    use tauri_plugin_updater::{Error, Update, UpdaterExt};

    use crate::desktop::domain::installation::InstallFailure;
    use crate::desktop::ports::UpdateInstaller;

    /// El plugin updater: `latest.json`, la firma minisign y el NSIS en modo `passive`.
    pub struct PluginInstaller {
        app: tauri::AppHandle,
        found: Mutex<Option<Update>>,
    }

    impl PluginInstaller {
        /// El instalador sobre la aplicación en marcha.
        pub fn new(app: tauri::AppHandle) -> Self {
            Self {
                app,
                found: Mutex::new(None),
            }
        }
    }

    impl UpdateInstaller for PluginInstaller {
        fn announced(&self) -> Result<Option<String>, InstallFailure> {
            let app = self.app.clone();
            let update = off_the_event_loop(move || {
                tauri::async_runtime::block_on(async move { app.updater()?.check().await })
            })?;
            let announced = update.as_ref().map(|update| update.version.clone());
            *self.found.lock().map_err(|_| InstallFailure::Network)? = update;
            Ok(announced)
        }

        fn install(&self) -> Result<(), InstallFailure> {
            let update = self
                .found
                .lock()
                .map_err(|_| InstallFailure::Network)?
                .take()
                .ok_or(InstallFailure::Network)?;
            off_the_event_loop(move || {
                tauri::async_runtime::block_on(async move {
                    update.download_and_install(|_, _| {}, || {}).await
                })
            })
        }
    }

    /// Corre el plugin en un hilo propio: la orden ya está dentro del runtime asíncrono.
    fn off_the_event_loop<T: Send + 'static>(
        work: impl FnOnce() -> Result<T, Error> + Send + 'static,
    ) -> Result<T, InstallFailure> {
        std::thread::spawn(work)
            .join()
            .map_err(|_| InstallFailure::Network)?
            .map_err(failure_of)
    }

    /// Lo que no es la firma se cuenta como fallo de red: no se pudo traer lo anunciado.
    fn failure_of(error: Error) -> InstallFailure {
        match error {
            Error::Minisign(_)
            | Error::Base64(_)
            | Error::SignatureUtf8(_)
            | Error::SignedVersionMismatch { .. }
            | Error::MissingSignedVersion => InstallFailure::InvalidSignature,
            _ => InstallFailure::Network,
        }
    }
}
