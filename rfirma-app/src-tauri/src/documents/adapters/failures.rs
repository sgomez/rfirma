//! La única traducción de las situaciones de documentos: a la vista de la ventana y al código de la sede (ADR-0009, ADR-0011, ADR-0012).

use crate::commands::Failure;
use crate::documents::application::recents::RecentsError;
use crate::documents::domain::destination::{DestinationError, Situation as DestinationSituation};
use crate::documents::domain::error::DocumentError;
use crate::documents::domain::rubric::{RubricError, Situation as RubricSituation};
use crate::documents::domain::told::DropRefusal;
use crate::site::domain::protocol::SafCode;

fn destination_told(situation: DestinationSituation) -> (&'static str, SafCode) {
    match situation {
        DestinationSituation::FolderMissing => ("folderMissing", SafCode::CannotSaveData),
        DestinationSituation::NotAFolder => ("notAFolder", SafCode::CannotSaveData),
        DestinationSituation::FolderUnreadable => ("folderUnreadable", SafCode::CannotSaveData),
        DestinationSituation::NoFreeName => ("noFreeName", SafCode::CannotSaveData),
    }
}

/// Código de protocolo de una situación de la carpeta destino (ADR-0011).
pub fn code_of_destination(situation: DestinationSituation) -> SafCode {
    destination_told(situation).1
}

impl From<DestinationError> for Failure {
    fn from(error: DestinationError) -> Self {
        Self::new(destination_told(error.situation()).0, error.detail())
    }
}

fn rubric_told(situation: RubricSituation) -> (&'static str, SafCode) {
    match situation {
        RubricSituation::NotAnAcceptedImage => ("notAnAcceptedImage", SafCode::VisibleSignature),
        RubricSituation::DamagedImage => ("damagedImage", SafCode::VisibleSignature),
        RubricSituation::ImageTooLarge => ("imageTooLarge", SafCode::VisibleSignature),
        RubricSituation::SourceUnreadable => ("sourceUnreadable", SafCode::VisibleSignature),
        RubricSituation::StoreUnwritable => ("storeUnwritable", SafCode::VisibleSignature),
        RubricSituation::StoreUnreadable => ("storeUnreadable", SafCode::VisibleSignature),
    }
}

/// Código de protocolo de una situación del almacén de rúbrica (ADR-0012).
pub fn code_of_rubric(situation: RubricSituation) -> SafCode {
    rubric_told(situation).1
}

impl From<&RubricError> for Failure {
    fn from(error: &RubricError) -> Self {
        Self::new(rubric_told(error.situation()).0, error.detail())
    }
}

impl From<RubricError> for Failure {
    fn from(error: RubricError) -> Self {
        Self::from(&error)
    }
}

fn document_told(error: &DocumentError) -> (Failure, SafCode) {
    match error {
        DocumentError::Unreadable(detail) => (
            Failure::new("documentUnreadable", detail.clone()),
            SafCode::CannotReadData,
        ),
        DocumentError::Destination(error) => (
            Failure::from(error.clone()),
            code_of_destination(error.situation()),
        ),
        DocumentError::FolderUnwritable(detail) => (
            Failure::new("folderUnwritable", detail.clone()),
            SafCode::CannotSaveData,
        ),
    }
}

/// Código de protocolo con el que la sede recibe un documento que no se pudo leer ni entregar.
pub fn code_of_document(error: &DocumentError) -> SafCode {
    document_told(error).1
}

impl From<DocumentError> for Failure {
    fn from(error: DocumentError) -> Self {
        document_told(&error).0
    }
}

impl From<RecentsError> for Failure {
    fn from(error: RecentsError) -> Self {
        match error {
            RecentsError::Document(error) => error.into(),
            RecentsError::Memory(error) => error.into(),
        }
    }
}

impl From<DropRefusal> for Failure {
    fn from(refusal: DropRefusal) -> Self {
        match refusal {
            DropRefusal::NotAPdf => crate::signing::domain::Refusal::NotAPdf.into(),
            DropRefusal::Unreadable(detail) => Self::new("droppedFileUnreadable", detail),
        }
    }
}

#[cfg(test)]
mod tests;
