//! Gestión del historial de documentos recientes con metadatos cacheados (ADR-0010, ADR-0011).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::signing::domain::PageSet;

pub use crate::documents::domain::recents::Badge;

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

/// Posición y páginas configuradas para la firma visible en un documento (ADR-0006).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Placement {
    /// Esquina inferior izquierda, eje X, en espacio de usuario PDF.
    pub lower_left_x: f64,
    /// Esquina inferior izquierda, eje Y, en espacio de usuario PDF.
    pub lower_left_y: f64,
    /// Páginas en las que estampar la firma visible.
    pub pages: PageSet,
}

impl<'de> Deserialize<'de> for Placement {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Stored {
            lower_left_x: f64,
            lower_left_y: f64,
            pages: Option<PageSet>,
            page: Option<u32>,
        }

        let stored = Stored::deserialize(deserializer)?;
        let pages = stored
            .pages
            .or_else(|| stored.page.map(PageSet::only_page))
            .ok_or_else(|| serde::de::Error::missing_field("pages"))?;
        Ok(Self {
            lower_left_x: stored.lower_left_x,
            lower_left_y: stored.lower_left_y,
            pages,
        })
    }
}

/// Metadatos cacheados de un documento reciente.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentDocument {
    path: PathBuf,
    name: String,
    badge: Badge,
    modified: Option<u64>,
    last_used: u64,
    #[serde(default)]
    placement: Option<Placement>,
}

impl RecentDocument {
    /// Construye una entrada reciente a partir de una ruta verificada.
    pub fn seen(path: &Path, badge: Badge, at: SystemTime) -> std::io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(seconds_since_epoch);
        Ok(Self {
            name: file_name(&path),
            path,
            badge,
            modified,
            last_used: seconds_since_epoch(at).unwrap_or_default(),
            placement: None,
        })
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
    pub fn placement(&self) -> Option<&Placement> {
        self.placement.as_ref()
    }

    /// Asigna o elimina la posición del recuadro visible para este documento.
    pub fn place(&mut self, placement: Option<Placement>) {
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
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Recents {
    entries: Vec<RecentDocument>,
}

impl<'de> Deserialize<'de> for Recents {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = Vec::<serde_json::Value>::deserialize(deserializer)?;
        let mut entries: Vec<RecentDocument> = rows
            .into_iter()
            .filter_map(|row| serde_json::from_value(row).ok())
            .collect();
        entries.truncate(CAPACITY);
        Ok(Self { entries })
    }
}

impl Recents {
    /// Registra un documento reciente colocándolo en cabeza y conservando posición previa.
    pub fn record(&mut self, mut document: RecentDocument) {
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

    /// Obtiene la entrada correspondiente a una ruta si existe.
    pub fn entry(&self, path: &Path) -> Option<&RecentDocument> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Actualiza la posición de recuadro asociada a una ruta registrada.
    pub fn place(&mut self, path: &Path, placement: Option<Placement>) {
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
    pub fn entries(&self) -> &[RecentDocument] {
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
