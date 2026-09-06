//! Los tipos de documentos que cruzan a la ventana principal (ADR-0011).

use serde::Serialize;

use crate::documents::application::recents::RecentRow;
use crate::documents::domain::recents::Badge;
use crate::documents::domain::told::{
    Destination, DroppedDocument, OpenedDocument, SignedDocument,
};

use crate::commands::Failure;
use crate::signing::adapters::views::PlacementView;

/// Destino previsto para el documento firmado (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationView {
    /// Nombre de la carpeta de destino.
    pub folder: String,
    /// Nombre del fichero firmado resultante.
    pub name: Option<String>,
    /// Si la carpeta de destino tiene permisos de escritura.
    pub writable: bool,
}

impl From<Destination> for DestinationView {
    fn from(destination: Destination) -> Self {
        Self {
            folder: destination.folder,
            name: destination.name,
            writable: destination.writable,
        }
    }
}

/// Documento firmado resultante (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedDocumentView {
    /// Nombre del fichero resultante.
    pub name: String,
    /// Nombre de la carpeta de destino.
    pub folder: String,
    /// Tamaño en bytes del fichero escrito.
    pub size_bytes: u64,
}

impl From<SignedDocument> for SignedDocumentView {
    fn from(signed: SignedDocument) -> Self {
        Self {
            name: signed.name,
            folder: signed.folder,
            size_bytes: signed.size_bytes,
        }
    }
}

/// Documento abierto para su visualización o firma (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocumentView {
    /// Identificador opaco asignado al documento.
    pub id: String,
    /// Nombre del fichero.
    pub name: String,
    /// Fecha de modificación en segundos Unix.
    pub modified: Option<u64>,
    /// Ruta en el anfitrión si está disponible.
    pub path: Option<String>,
}

impl From<OpenedDocument> for OpenedDocumentView {
    fn from(opened: OpenedDocument) -> Self {
        Self {
            id: opened.id,
            name: opened.name,
            modified: opened.modified,
            path: opened.path,
        }
    }
}

/// Resultado de soltar ficheros sobre la ventana (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedDocumentView {
    /// Documento abierto en el visor.
    pub document: Option<OpenedDocumentView>,
    /// Documentos adicionales incorporados a recientes.
    pub also_entering: Vec<OpenedDocumentView>,
    /// Motivo del fallo si no se pudo abrir ningún documento.
    pub failure: Option<Failure>,
    /// Número de ficheros descartados que no se incorporaron.
    pub discarded: usize,
}

impl From<DroppedDocument> for DroppedDocumentView {
    fn from(dropped: DroppedDocument) -> Self {
        Self {
            document: dropped.document.map(OpenedDocumentView::from),
            also_entering: dropped
                .also_entering
                .into_iter()
                .map(OpenedDocumentView::from)
                .collect(),
            failure: dropped.refused.map(Failure::from),
            discarded: dropped.discarded,
        }
    }
}

/// Entrada de la lista de documentos recientes (ADR-0011).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDocumentView {
    /// Identificador opaco del documento.
    pub id: String,
    /// Nombre del fichero.
    pub name: String,
    /// Insignia o estado del documento.
    pub badge: Badge,
    /// Fecha de modificación en segundos Unix.
    pub modified: Option<u64>,
    /// Fecha de último uso en segundos Unix.
    pub last_used: u64,
    /// Si el fichero sigue existiendo en disco.
    pub available: bool,
    /// Posición del recuadro guardada para este documento.
    pub placement: Option<PlacementView>,
}

impl From<RecentRow> for RecentDocumentView {
    fn from(row: RecentRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            badge: row.badge,
            modified: row.modified,
            last_used: row.last_used,
            available: row.available,
            placement: row.placement.map(PlacementView::from),
        }
    }
}

#[cfg(test)]
mod tests;
