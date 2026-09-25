//! El destino de una sola firma: elegirlo con el diálogo de guardar, contarlo y entregar en él, sin tocar la preferencia de carpeta (ADR-0011).

use std::path::PathBuf;

use crate::documents::application::documents::{checked, landing_for};
use crate::documents::domain::destination::{
    signed_name, CheckedFolder, DestinationError, DestinationFolder, SingleDestination, Situation,
};
use crate::documents::domain::document::Document;
use crate::documents::domain::error::DocumentError;
use crate::documents::domain::handles::Handles;
use crate::documents::domain::told::{ChosenDestination, Destination, SignedDocument};
use crate::documents::ports::{DialogClues, DocumentFiles, PortalDialogs};

/// Los destinos de una sola firma elegidos en esta sesión, cada uno tras su asa.
pub type SingleDestinations = Handles<SingleDestination>;

/// Abre el diálogo de guardar y apunta tras un asa el fichero elegido para esta firma.
pub fn choose(
    portal: &dyn PortalDialogs,
    files: &dyn DocumentFiles,
    singles: &SingleDestinations,
    preferred: &DestinationFolder,
    document: &Document,
) -> Result<Option<ChosenDestination>, String> {
    let chosen = portal.save_file(&clues_for_saving(files, preferred, document))?;
    Ok(chosen.map(|path| {
        let single = SingleDestination::at(path);
        let destination = where_it_lands(files, &single);
        ChosenDestination {
            id: singles.mint(single),
            destination,
        }
    }))
}

fn clues_for_saving(
    files: &dyn DocumentFiles,
    preferred: &DestinationFolder,
    document: &Document,
) -> DialogClues {
    let clues = DialogClues::new().with_filter("PDF", &["pdf"]);
    let Ok(folder) = checked(files, preferred) else {
        return clues.with_filename(signed_name(document.name()));
    };
    let name = landing_for(files, &folder, document)
        .ok()
        .and_then(|landing| landing.file_name()?.to_str().map(str::to_owned))
        .unwrap_or_else(|| signed_name(document.name()));
    clues
        .with_starting_folder(folder.path())
        .with_filename(name)
}

/// El destino elegido tras el asa, o el fallo si ya no está en esta sesión.
pub fn chosen(singles: &SingleDestinations, id: &str) -> Result<SingleDestination, DocumentError> {
    singles.get(id).ok_or_else(|| {
        DestinationError::new(
            Situation::FolderMissing,
            "el destino elegido ya no esta en esta sesion",
        )
        .into()
    })
}

/// Lo que se cuenta del destino elegido antes de firmar, sin escribir en disco.
pub fn where_it_lands(files: &dyn DocumentFiles, single: &SingleDestination) -> Destination {
    Destination {
        folder: single.shown_folder().to_owned(),
        name: Some(single.name().to_owned()),
        writable: checked_folder(files, single).is_ok(),
    }
}

/// Escribe el firmado en el fichero elegido, tal cual lo nombró la persona.
pub fn deliver(
    files: &dyn DocumentFiles,
    single: &SingleDestination,
    signed: &[u8],
) -> Result<(PathBuf, SignedDocument), DocumentError> {
    checked_folder(files, single)?;
    files
        .write(single.path(), signed)
        .map_err(DocumentError::FolderUnwritable)?;
    let told = SignedDocument {
        name: single.name().to_owned(),
        folder: single.shown_folder().to_owned(),
        size_bytes: signed.len() as u64,
    };
    Ok((single.path().to_path_buf(), told))
}

fn checked_folder(
    files: &dyn DocumentFiles,
    single: &SingleDestination,
) -> Result<CheckedFolder, DestinationError> {
    CheckedFolder::confirmed(single.folder(), files.folder_fact(single.folder()))
}

#[cfg(test)]
mod tests;
