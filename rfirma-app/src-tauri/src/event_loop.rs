//! El bucle de eventos de Tauri de los dos roles: construye la aplicación, la ejecuta y limpia al salir; no compone raíces.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{desktop, site, startup_dialog, startup_failure};

/// Construye la aplicación con el único contexto de Tauri del crate y la ejecuta.
pub(crate) fn build_and_run(
    builder: tauri::Builder<tauri::Wry>,
    context: tauri::Context<tauri::Wry>,
    serving: &Arc<Serving>,
) {
    let serving = Arc::clone(serving);
    builder
        .build(context)
        .unwrap_or_else(|error| {
            startup_dialog::report_and_exit(&startup_failure::StartupFailure::new(
                startup_failure::Situation::WindowUnavailable,
                error.to_string(),
            ))
        })
        .run(move |app, event| {
            let window_is_up = serving.window_is_up.load(Ordering::SeqCst);
            if !desktop::adapters::process::launch_the_delivered_urls(&event, window_is_up) {
                serving.only_for_links.store(true, Ordering::SeqCst);
            }
            erase_the_scratch_folder_on_exit(app, &event);
        });
}

/// Borra la carpeta de paso de este proceso al salir del bucle de eventos; un `Drop` no es
/// fiable porque Tauri puede salir sin soltarlo.
fn erase_the_scratch_folder_on_exit(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    use tauri::Manager;

    if let tauri::RunEvent::Exit = event {
        let folder = app.state::<site::adapters::scratch::ProcessFolder>();
        let _ = std::fs::remove_dir_all(folder.path());
    }
}

/// Si una ventana de este proceso atiende ya a la persona, o si solo arrancó para entregar
/// enlaces a procesos de sede.
#[derive(Default)]
pub(crate) struct Serving {
    pub(crate) window_is_up: AtomicBool,
    pub(crate) only_for_links: AtomicBool,
}

impl Serving {
    pub(crate) fn from_the_start() -> Self {
        Self {
            window_is_up: AtomicBool::new(true),
            only_for_links: AtomicBool::new(false),
        }
    }
}
