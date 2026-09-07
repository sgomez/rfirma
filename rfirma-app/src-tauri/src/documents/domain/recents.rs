//! La bandeja de recientes: los diez últimos, por ruta canónica, con lo que quien firma quiera recordar de cada uno (ADR-0010, ADR-0011).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Estado de firma persistido en caché para un documento reciente.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Badge {
    /// Documento con al menos una firma.
    Signed,
    /// Documento sin firmas.
    Unsigned,
}

/// Capacidad máxima del historial de documentos recientes.
pub const CAPACITY: usize = 10;

/// Estado de firma visualizado en la interfaz, incluyendo disponibilidad actual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShownBadge {
    /// Documento con al menos una firma.
    Signed,
    /// Documento sin firmas.
    Unsigned,
    /// El fichero ya no está accesible en la ruta registrada.
    Unavailable,
}

/// Metadatos cacheados de un documento reciente; `S` es lo que se recuerda de su recuadro.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentDocument<S> {
    path: PathBuf,
    name: String,
    badge: Badge,
    modified: Option<u64>,
    last_used: u64,
    #[serde(default = "nothing")]
    placement: Option<S>,
}

fn nothing<S>() -> Option<S> {
    None
}

impl<S> RecentDocument<S> {
    /// Construye una entrada reciente sobre una ruta ya canónica y su instante de modificación.
    pub fn seen(path: PathBuf, modified: Option<u64>, badge: Badge, at: SystemTime) -> Self {
        Self {
            name: file_name(&path),
            path,
            badge,
            modified,
            last_used: seconds_since_epoch(at).unwrap_or_default(),
            placement: None,
        }
    }

    /// Ruta canónica del documento.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nombre de fichero para visualización.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Insignia de firma en caché.
    pub fn badge(&self) -> Badge {
        self.badge
    }

    /// Fecha de última modificación registrada en segundos desde la época UNIX.
    pub fn modified(&self) -> Option<u64> {
        self.modified
    }

    /// Fecha de último acceso registrada en segundos desde la época UNIX.
    pub fn last_used(&self) -> u64 {
        self.last_used
    }

    /// Posición del recuadro visible configurada en este documento.
    pub fn placement(&self) -> Option<&S> {
        self.placement.as_ref()
    }

    /// Asigna o elimina la posición del recuadro visible para este documento.
    pub fn place(&mut self, placement: Option<S>) {
        self.placement = placement;
    }

    /// Comprueba si el fichero existe actualmente en la ruta registrada.
    pub fn is_available(&self) -> bool {
        self.path.exists()
    }

    /// Estado de firma calculado para visualización.
    pub fn shown_badge(&self) -> ShownBadge {
        if !self.is_available() {
            return ShownBadge::Unavailable;
        }
        match self.badge {
            Badge::Signed => ShownBadge::Signed,
            Badge::Unsigned => ShownBadge::Unsigned,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn seconds_since_epoch(instant: SystemTime) -> Option<u64> {
    instant
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

/// Colección acotada de documentos recientes ordenados por fecha de uso.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Recents<S> {
    entries: Vec<RecentDocument<S>>,
}

impl<S> Default for Recents<S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<'de, S: DeserializeOwned> Deserialize<'de> for Recents<S> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = Vec::<serde_json::Value>::deserialize(deserializer)?;
        let mut entries: Vec<RecentDocument<S>> = rows
            .into_iter()
            .filter_map(|row| serde_json::from_value(row).ok())
            .collect();
        entries.truncate(CAPACITY);
        Ok(Self { entries })
    }
}

impl<S: Clone> Recents<S> {
    /// Registra un documento reciente colocándolo en cabeza y conservando posición previa.
    pub fn record(&mut self, mut document: RecentDocument<S>) {
        let remembered = self
            .entries
            .iter()
            .find(|entry| entry.path == document.path)
            .and_then(|entry| entry.placement.clone());
        if document.placement.is_none() {
            document.placement = remembered;
        }
        self.entries.retain(|entry| entry.path != document.path);
        self.entries.insert(0, document);
        self.entries.truncate(CAPACITY);
    }
}

impl<S> Recents<S> {
    /// Obtiene la entrada correspondiente a una ruta si existe.
    pub fn entry(&self, path: &Path) -> Option<&RecentDocument<S>> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Actualiza la posición de recuadro asociada a una ruta registrada.
    pub fn place(&mut self, path: &Path, placement: Option<S>) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) {
            entry.place(placement);
        }
    }

    /// Elimina las posiciones de recuadro configuradas en todos los recientes.
    pub fn forget_placements(&mut self) {
        for entry in &mut self.entries {
            entry.place(None);
        }
    }

    /// Lista ordenada de documentos recientes.
    pub fn entries(&self) -> &[RecentDocument<S>] {
        &self.entries
    }

    /// Elimina una ruta concreta del historial de recientes.
    pub fn forget(&mut self, path: &Path) {
        self.entries.retain(|entry| entry.path != path);
    }

    /// Vacía todas las entradas del historial.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Número de entradas en el historial.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Comprueba si el historial está vacío.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests;
