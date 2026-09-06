//! Casos de uso para apertura y resolución de destino de documentos (ADR-0011).

use std::path::{Path, PathBuf};

use crate::commands::Failure;
use crate::documents::adapters::portal::PortalDocument;
use crate::documents::adapters::views::{
    DestinationView, DroppedDocumentView, OpenedDocumentView, SignedDocumentView,
};
use crate::documents::application::opened::OpenedDocuments;
use crate::documents::domain::destination::{CheckedFolder, DestinationFolder};
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::domain::Refusal;
use crate::Memory;

/// Registra el documento abierto por el usuario y actualiza la última carpeta usada.
pub fn note_opened(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    handle: PathBuf,
) -> OpenedDocumentView {
    let document = PortalDocument::opened(handle);
    remember_the_folder(memory, configuration, &document);
    told_as_opened(document, opened)
}

/// Registra un documento en curso sin guardar rastro en el historial ni recordar carpeta.
pub fn note_opened_unrecorded(opened: &OpenedDocuments, handle: PathBuf) -> OpenedDocumentView {
    let document = PortalDocument::opened(handle);
    let name = document.name().to_owned();
    let modified = modified_seconds(&document);
    let path = real_path_of(&document).and_then(|path| path.to_str().map(str::to_owned));
    OpenedDocumentView {
        id: opened.remember_unrecorded(document),
        name,
        modified,
        path,
    }
}

/// Devuelve el contenido en bytes del documento abierto por su identificador.
pub fn bytes_of(opened: &OpenedDocuments, id: &str) -> Result<Vec<u8>, Failure> {
    let document = opened_document(opened, id)?;
    std::fs::read(document.reading_path())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))
}

/// Procesa los ficheros soltados en la ventana y registra el primer PDF válido.
pub fn dropped_document(
    paths: &[PathBuf],
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    told_as_dropped(crate::documents::domain::dropped::first_pdf(paths), opened)
}

/// Convierte el resultado de procesamiento de arrastre en una vista para la ventana.
pub(crate) fn told_as_dropped(
    decided: crate::documents::domain::dropped::Dropped,
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    match decided {
        crate::documents::domain::dropped::Dropped::Nothing => None,
        crate::documents::domain::dropped::Dropped::Opened {
            path,
            also_entering,
            discarded,
        } => Some(DroppedDocumentView {
            document: Some(told_as_opened(PortalDocument::opened(path), opened)),
            also_entering: also_entering
                .into_iter()
                .map(|path| told_as_opened(PortalDocument::opened(path), opened))
                .collect(),
            failure: None,
            discarded,
        }),
        crate::documents::domain::dropped::Dropped::NotAPdf { discarded } => {
            Some(DroppedDocumentView {
                document: None,
                also_entering: Vec::new(),
                failure: Some(Failure::from(Refusal::NotAPdf)),
                discarded,
            })
        }
        crate::documents::domain::dropped::Dropped::Unreadable { detail, discarded } => {
            Some(DroppedDocumentView {
                document: None,
                also_entering: Vec::new(),
                failure: Some(Failure::new("droppedFileUnreadable", detail)),
                discarded,
            })
        }
    }
}

/// Guarda el documento firmado en la carpeta de destino resolviendo homónimos (ADR-0011).
pub fn deliver(
    configuration: &Configuration,
    documents_folder: &Path,
    document: &PortalDocument,
    signed: &[u8],
) -> Result<(PathBuf, SignedDocumentView), Failure> {
    let chosen = crate::chosen_folder(configuration, documents_folder.to_path_buf());
    let folder = CheckedFolder::check(&chosen)?;
    let landing = folder.landing_for(document)?;
    std::fs::write(&landing, signed)
        .map_err(|error| Failure::new("folderUnwritable", error.to_string()))?;
    let told = told_as(&landing, &folder, signed.len() as u64);
    Ok((landing, told))
}

/// Calcula la ruta prevista de destino antes de firmar sin escribir en disco (ADR-0011).
pub fn where_it_lands(
    configuration: &Configuration,
    documents_folder: &Path,
    document: &PortalDocument,
) -> DestinationView {
    let chosen = crate::chosen_folder(configuration, documents_folder.to_path_buf());
    let Ok(folder) = CheckedFolder::check(&chosen) else {
        return DestinationView {
            folder: chosen.name().to_owned(),
            name: None,
            writable: false,
        };
    };
    let name = folder
        .landing_for(document)
        .ok()
        .and_then(|landing| file_name_of(&landing));
    DestinationView {
        folder: folder.name().to_owned(),
        name,
        writable: true,
    }
}

/// Construye la vista de un documento firmado para la ventana (ADR-0011).
pub fn told_as(landing: &Path, folder: &CheckedFolder, size_bytes: u64) -> SignedDocumentView {
    SignedDocumentView {
        name: file_name_of(landing).unwrap_or_default(),
        folder: folder.name().to_owned(),
        size_bytes,
    }
}

fn file_name_of(landing: &Path) -> Option<String> {
    landing
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

fn told_as_opened(document: PortalDocument, opened: &OpenedDocuments) -> OpenedDocumentView {
    let name = document.name().to_owned();
    let modified = modified_seconds(&document);
    let path = real_path_of(&document).and_then(|path| path.to_str().map(str::to_owned));
    OpenedDocumentView {
        id: opened.remember(document),
        name,
        modified,
        path,
    }
}

/// Determina la carpeta inicial para el diálogo de apertura de documentos.
pub fn starting_folder(
    memory: &Memory,
    configuration: &Configuration,
    documents_folder: &Path,
) -> Option<PathBuf> {
    if let Some(remembered) = remembered_folder(memory) {
        return Some(remembered);
    }
    let folder = crate::chosen_folder(configuration, documents_folder.to_path_buf());
    CheckedFolder::check(&folder)
        .ok()
        .map(|checked| checked.path().to_path_buf())
}

/// Devuelve la última carpeta de apertura recordada si continúa existiendo.
pub fn remembered_folder(memory: &Memory) -> Option<PathBuf> {
    memory
        .state()
        .ok()?
        .into_value()
        .last_open_folder
        .filter(|folder| folder.is_dir())
}

/// Registra la carpeta de procedencia de un documento si es conocida.
pub fn remember_the_folder(
    memory: &Memory,
    configuration: &Configuration,
    document: &PortalDocument,
) {
    let Some(folder) = folder_it_came_from(document) else {
        return;
    };
    let Ok(loaded) = memory.state() else {
        return;
    };
    let mut state = loaded.into_value();
    if state.last_open_folder.as_deref() == Some(folder) {
        return;
    }
    state.last_open_folder = Some(folder.to_path_buf());
    let _ = memory.remember_state(configuration, &state);
}

/// Devuelve la carpeta de procedencia del documento o `None` si proviene del portal (ADR-0011).
pub fn folder_it_came_from(document: &PortalDocument) -> Option<&Path> {
    if document.came_through_the_portal() {
        return None;
    }
    document.reading_path().parent()
}

/// Obtiene la carpeta junto al original si el documento no entró por el portal.
pub fn next_to_the_original(document: &PortalDocument) -> Option<DestinationFolder> {
    folder_it_came_from(document).map(DestinationFolder::at)
}

/// Devuelve la ruta real del documento si no procede del portal.
pub fn real_path_of(document: &PortalDocument) -> Option<&Path> {
    if document.came_through_the_portal() {
        return None;
    }
    Some(document.reading_path())
}

/// Obtiene el documento abierto correspondiente al identificador opaco.
pub fn opened_document(opened: &OpenedDocuments, id: &str) -> Result<PortalDocument, Failure> {
    opened.get(id).ok_or_else(|| {
        Failure::new(
            "documentUnreadable",
            "el documento ya no esta abierto en esta sesion",
        )
    })
}

pub(crate) fn modified_seconds(document: &PortalDocument) -> Option<u64> {
    std::fs::metadata(document.reading_path())
        .and_then(|metadata| metadata.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

#[cfg(test)]
mod tests;
