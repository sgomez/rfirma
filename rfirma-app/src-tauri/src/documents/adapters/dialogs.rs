//! El diálogo de verdad tras el puerto `PortalDialogs`: `tauri_plugin_dialog` y nada más.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use tauri_plugin_dialog::DialogExt;

use crate::documents::ports::{DialogClues, PortalDialogs};

/// Adaptador de producción para los diálogos del sistema a través de Tauri.
#[derive(Clone, Default)]
pub struct RealPortalDialogs {
    app: Arc<OnceLock<tauri::AppHandle>>,
}

impl RealPortalDialogs {
    /// Crea un adaptador vacío pendiente de vincular al manejador de Tauri.
    pub fn new() -> Self {
        Self::default()
    }

    /// Crea un adaptador vinculado al manejador de Tauri dado.
    pub fn of(app: tauri::AppHandle) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(app);
        Self {
            app: Arc::new(cell),
        }
    }

    /// Vincula el manejador de la aplicación al adaptador.
    pub fn attach(&self, app: tauri::AppHandle) {
        let _ = self.app.set(app);
    }

    fn dialog_builder<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        clues: &DialogClues,
    ) -> tauri_plugin_dialog::FileDialogBuilder<R> {
        let mut dialog = app.dialog().file();
        if let Some(title) = &clues.title {
            dialog = dialog.set_title(title);
        }
        if let Some(filename) = &clues.filename {
            dialog = dialog.set_file_name(filename);
        }
        if let Some(folder) = &clues.starting_folder {
            dialog = dialog.set_directory(folder);
        }
        if !clues.extensions.is_empty() {
            let exts: Vec<&str> = clues.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(clues.description.as_deref().unwrap_or(""), &exts);
        }
        dialog
    }
}

impl PortalDialogs for RealPortalDialogs {
    fn pick_file(&self, clues: &DialogClues) -> Result<Option<PathBuf>, String> {
        let Some(app) = self.app.get() else {
            return Err("no hay manejador de aplicación disponible para mostrar el diálogo".into());
        };
        let dialog = Self::dialog_builder(app, clues);
        match dialog.blocking_pick_file() {
            Some(file_path) => file_path
                .into_path()
                .map(Some)
                .map_err(|error| error.to_string()),
            None => Ok(None),
        }
    }

    fn pick_files(&self, clues: &DialogClues) -> Result<Vec<PathBuf>, String> {
        let Some(app) = self.app.get() else {
            return Err("no hay manejador de aplicación disponible para mostrar el diálogo".into());
        };
        let dialog = Self::dialog_builder(app, clues);
        match dialog.blocking_pick_files() {
            Some(file_paths) => file_paths
                .into_iter()
                .map(|file_path| file_path.into_path().map_err(|error| error.to_string()))
                .collect(),
            None => Ok(Vec::new()),
        }
    }

    fn save_file(&self, clues: &DialogClues) -> Result<Option<PathBuf>, String> {
        let Some(app) = self.app.get() else {
            return Err("no hay manejador de aplicación disponible para mostrar el diálogo".into());
        };
        let dialog = Self::dialog_builder(app, clues);
        match dialog.blocking_save_file() {
            Some(file_path) => file_path
                .into_path()
                .map(Some)
                .map_err(|error| error.to_string()),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests;
