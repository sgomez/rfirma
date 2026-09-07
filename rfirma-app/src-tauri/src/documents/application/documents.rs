//! Casos de uso para apertura y resolución de destino de documentos (ADR-0011).

use std::path::{Path, PathBuf};

use crate::documents::domain::destination::{CheckedFolder, DestinationFolder};
use crate::documents::domain::document::Document;
use crate::documents::domain::error::DocumentError;
use crate::documents::domain::handles::Handles;
use crate::documents::domain::told::{
    Destination, DropRefusal, DroppedDocument, OpenedDocument, SignedDocument,
};
use crate::documents::ports::{DocumentFiles, DocumentsMemory};

/// Los documentos abiertos en esta sesión, cada uno tras su asa.
pub type OpenedDocuments = Handles<Document>;

/// Registra el documento abierto por el usuario y actualiza la última carpeta usada.
pub fn note_opened(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
    handle: PathBuf,
) -> OpenedDocument {
    let document = Document::opened(handle);
    remember_the_folder(memory, &document);
    told_as_opened(files, document, opened)
}

/// Registra un documento en curso sin guardar rastro en el historial ni recordar carpeta.
pub fn note_opened_unrecorded(
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
    handle: PathBuf,
) -> OpenedDocument {
    told_as_opened(files, Document::passing_through(handle), opened)
}

/// Devuelve el contenido en bytes del documento abierto por su identificador.
pub fn bytes_of(
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
    id: &str,
) -> Result<Vec<u8>, DocumentError> {
    let document = opened_document(opened, id)?;
    files
        .read(document.reading_path())
        .map_err(DocumentError::Unreadable)
}

/// Procesa los ficheros soltados en la ventana y registra el primer PDF válido.
pub fn dropped_document(
    files: &dyn DocumentFiles,
    paths: &[PathBuf],
    opened: &OpenedDocuments,
) -> Option<DroppedDocument> {
    told_as_dropped(files, decide_what_was_dropped(files, paths), opened)
}

/// Expande las carpetas soltadas, elige el primer PDF y pregunta al disco si se deja leer.
pub fn decide_what_was_dropped(
    files: &dyn DocumentFiles,
    paths: &[PathBuf],
) -> crate::documents::domain::dropped::Dropped {
    use crate::documents::domain::dropped::{first_pdf, resolved, Choice};
    let choice = first_pdf(&expanded(files, paths));
    let readable = match &choice {
        Choice::Pdf { path, .. } => files.readable(path),
        _ => Ok(()),
    };
    resolved(choice, readable)
}

fn expanded(files: &dyn DocumentFiles, paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut expanded = Vec::with_capacity(paths.len());
    for path in paths {
        match files.folder_fact(path) {
            crate::documents::domain::destination::FolderFact::Folder => {
                expanded.extend(files.files_within(path));
            }
            _ => expanded.push(path.clone()),
        }
    }
    expanded
}

/// Convierte el resultado de procesamiento de arrastre en lo que se cuenta a la ventana.
pub fn told_as_dropped(
    files: &dyn DocumentFiles,
    decided: crate::documents::domain::dropped::Dropped,
    opened: &OpenedDocuments,
) -> Option<DroppedDocument> {
    match decided {
        crate::documents::domain::dropped::Dropped::Nothing => None,
        crate::documents::domain::dropped::Dropped::Opened {
            path,
            also_entering,
            discarded,
        } => Some(DroppedDocument {
            document: Some(told_as_opened(files, Document::opened(path), opened)),
            also_entering: also_entering
                .into_iter()
                .map(|path| told_as_opened(files, Document::opened(path), opened))
                .collect(),
            refused: None,
            discarded,
        }),
        crate::documents::domain::dropped::Dropped::NotAPdf { discarded } => {
            Some(DroppedDocument {
                document: None,
                also_entering: Vec::new(),
                refused: Some(DropRefusal::NotAPdf),
                discarded,
            })
        }
        crate::documents::domain::dropped::Dropped::Unreadable { detail, discarded } => {
            Some(DroppedDocument {
                document: None,
                also_entering: Vec::new(),
                refused: Some(DropRefusal::Unreadable(detail)),
                discarded,
            })
        }
    }
}

/// Guarda el documento firmado en la carpeta de destino resolviendo homónimos (ADR-0011).
pub fn deliver(
    files: &dyn DocumentFiles,
    chosen: &DestinationFolder,
    document: &Document,
    signed: &[u8],
) -> Result<(PathBuf, SignedDocument), DocumentError> {
    let folder = checked(files, chosen)?;
    let landing = landing_for(files, &folder, document)?;
    files
        .write(&landing, signed)
        .map_err(DocumentError::FolderUnwritable)?;
    let told = told_as(&landing, &folder, signed.len() as u64);
    Ok((landing, told))
}

/// Comprueba la carpeta de destino preguntando al disco qué hay en su ruta (ADR-0011).
pub fn checked(
    files: &dyn DocumentFiles,
    chosen: &DestinationFolder,
) -> Result<CheckedFolder, crate::documents::domain::destination::DestinationError> {
    CheckedFolder::confirmed(chosen.path(), files.folder_fact(chosen.path()))
}

/// Recorre los nombres homónimos hasta dar con uno libre (ADR-0011).
pub fn landing_for(
    files: &dyn DocumentFiles,
    folder: &CheckedFolder,
    document: &Document,
) -> Result<PathBuf, crate::documents::domain::destination::DestinationError> {
    folder
        .landing_candidates(document)
        .find(|candidate| !files.exists(candidate))
        .ok_or_else(|| folder.no_free_name(document))
}

/// Calcula la ruta prevista de destino antes de firmar sin escribir en disco (ADR-0011).
pub fn where_it_lands(
    files: &dyn DocumentFiles,
    chosen: &DestinationFolder,
    document: &Document,
) -> Destination {
    let Ok(folder) = checked(files, chosen) else {
        return Destination {
            folder: chosen.name().to_owned(),
            name: None,
            writable: false,
        };
    };
    let name = landing_for(files, &folder, document)
        .ok()
        .and_then(|landing| file_name_of(&landing));
    Destination {
        folder: folder.name().to_owned(),
        name,
        writable: true,
    }
}

/// Lo que se cuenta de un documento firmado ya entregado (ADR-0011).
pub fn told_as(landing: &Path, folder: &CheckedFolder, size_bytes: u64) -> SignedDocument {
    SignedDocument {
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

fn told_as_opened(
    files: &dyn DocumentFiles,
    document: Document,
    opened: &OpenedDocuments,
) -> OpenedDocument {
    let name = document.name().to_owned();
    let modified = modified_seconds(files, &document);
    let path = real_path_of(&document).and_then(|path| path.to_str().map(str::to_owned));
    OpenedDocument {
        id: opened.mint(document),
        name,
        modified,
        path,
    }
}

/// Determina la carpeta inicial para el diálogo de apertura de documentos.
pub fn starting_folder(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    chosen: &DestinationFolder,
) -> Option<PathBuf> {
    if let Some(remembered) = remembered_folder(memory, files) {
        return Some(remembered);
    }
    checked(files, chosen)
        .ok()
        .map(|checked| checked.path().to_path_buf())
}

/// Devuelve la última carpeta de apertura recordada si continúa existiendo.
pub fn remembered_folder(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
) -> Option<PathBuf> {
    memory.last_open_folder().filter(|folder| {
        files.folder_fact(folder) == crate::documents::domain::destination::FolderFact::Folder
    })
}

/// La carpeta de destino elegida, o la de documentos por omisión.
pub fn chosen_folder(
    memory: &dyn DocumentsMemory,
    documents_folder: impl Into<PathBuf>,
) -> DestinationFolder {
    memory
        .chosen_destination()
        .unwrap_or_else(|| DestinationFolder::at(documents_folder))
}

/// Registra la carpeta de procedencia de un documento si es conocida.
pub fn remember_the_folder(memory: &dyn DocumentsMemory, document: &Document) {
    let Some(folder) = folder_it_came_from(document) else {
        return;
    };
    if memory.last_open_folder().as_deref() == Some(folder) {
        return;
    }
    let _ = memory.remember_last_open_folder(folder);
}

/// Devuelve la carpeta de procedencia del documento o `None` si proviene del portal (ADR-0011).
pub fn folder_it_came_from(document: &Document) -> Option<&Path> {
    if document.came_through_the_portal() {
        return None;
    }
    document.reading_path().parent()
}

/// Obtiene la carpeta junto al original si el documento no entró por el portal.
pub fn next_to_the_original(document: &Document) -> Option<DestinationFolder> {
    folder_it_came_from(document).map(DestinationFolder::at)
}

/// Devuelve la ruta real del documento si no procede del portal.
pub fn real_path_of(document: &Document) -> Option<&Path> {
    if document.came_through_the_portal() {
        return None;
    }
    Some(document.reading_path())
}

/// Obtiene el documento abierto correspondiente al identificador opaco.
pub fn opened_document(opened: &OpenedDocuments, id: &str) -> Result<Document, DocumentError> {
    opened.get(id).ok_or_else(DocumentError::no_longer_open)
}

pub(crate) fn modified_seconds(files: &dyn DocumentFiles, document: &Document) -> Option<u64> {
    files.modified_seconds(document.reading_path())
}

#[cfg(test)]
mod tests;
