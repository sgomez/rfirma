//! **El documento que la aplicación tiene delante**, que no es la fila que se
//! guarda en la bandeja (ID-287).
//!
//! Hasta aquí los dos conceptos eran el mismo tipo: el único camino para tener
//! un documento delante —con su insignia y su recuadro— era escribir su fila
//! con [`crate::app::recents::record`], y firmar terminaba escribiendo otra.
//! Eso vale mientras todo lo que se firma sea algo que el usuario abrió; deja
//! de valer en cuanto quien manda el documento es una sede, porque **de ese no
//! se guarda nada** (ID-286): ni fila, ni colocación del recuadro, ni «último
//! documento».
//!
//! Así que son dos:
//!
//! - [`DocumentInHand`] —lo de aquí— es el documento **en curso**: el
//!   identificador con el que la ventana lo nombra, la concesión del portal
//!   que hay detrás y si de él se guarda rastro. Vive lo que dura el trabajo.
//! - [`crate::memory::RecentDocument`] es la **fila**: lo que se persiste, se
//!   deduplica por ruta canónica y se pinta en la bandeja.
//!
//! Quien decide si la segunda existe es la primera, y por eso el interruptor
//! ([`crate::memory::Remembrance`]) viaja con la concesión y no con la fila.

use std::path::Path;

use crate::app::{documents, recents};
use crate::commands::views::{Failure, PlacementView, RecentDocumentView};
use crate::destination::PortalDocument;
use crate::memory::{Badge, Configuration, Memory, OpenedDocuments, Remembrance};

/// El documento que la aplicación tiene entre manos.
///
/// **No es una fila de la bandeja** y no se persiste: es lo que hace falta para
/// trabajar con el documento en curso —leerlo, firmarlo— más la única cosa que
/// hay que saber para no dejar rastro de él cuando no se debe.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentInHand {
    id: String,
    document: PortalDocument,
    remembrance: Remembrance,
}

impl DocumentInHand {
    /// **Caso de uso.** Toma en la mano el documento abierto con ese
    /// identificador.
    ///
    /// Lo que la ventana manda es el identificador que se acuñó al abrir, y no
    /// una ruta: quien sabe a qué concesión del portal corresponde es el
    /// registro, y solo él (ID-62). De ahí sale también si se recuerda, que se
    /// decidió por dónde entró el documento y no aquí.
    pub fn taken(opened: &OpenedDocuments, id: &str) -> Result<Self, Failure> {
        let document = documents::opened_document(opened, id)?;
        let remembrance = opened.remembrance(id).unwrap_or(Remembrance::Unrecorded);
        Ok(Self {
            id: id.to_owned(),
            document,
            remembrance,
        })
    }

    /// El identificador con el que la ventana lo nombra.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// La concesión del portal que hay detrás.
    pub fn document(&self) -> &PortalDocument {
        &self.document
    }

    /// Por dónde se leen sus bytes. **No cruza a la ventana** (ADR-0011).
    pub fn reading_path(&self) -> &Path {
        self.document.reading_path()
    }

    /// Si de este documento se guarda rastro.
    pub fn is_remembered(&self) -> bool {
        self.remembrance == Remembrance::Remembered
    }
}

/// **Caso de uso.** Pone delante el documento abierto y, **solo si se
/// recuerda**, deja su fila en la bandeja.
///
/// Es el único sitio donde se decide esa diferencia. Lo que devuelve tiene la
/// misma forma en los dos casos —la ventana pinta lo mismo— pero uno ha
/// escrito en el disco y el otro no ha tocado nada: ni fila, ni la colocación
/// del recuadro, ni el tamaño global.
pub fn take(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    id: &str,
    placement: Option<PlacementView>,
) -> Result<RecentDocumentView, Failure> {
    let in_hand = DocumentInHand::taken(opened, id)?;
    if in_hand.is_remembered() {
        return recents::record(memory, configuration, opened, id, placement);
    }
    Ok(told_without_a_row(&in_hand, placement))
}

/// El documento en curso contado como la ventana lo entiende, **sin fila
/// detrás**.
///
/// La insignia es `Sin firmar` y no la cacheada porque no hay nada cacheado: de
/// este documento no se guardó nunca nada, y firmarlo tampoco guardará. El
/// recuadro que sale es el que entró, devuelto tal cual: la ventana lo tiene
/// puesto, pero nadie lo ha recordado.
fn told_without_a_row(
    in_hand: &DocumentInHand,
    placement: Option<PlacementView>,
) -> RecentDocumentView {
    RecentDocumentView {
        id: in_hand.id().to_owned(),
        name: in_hand.document().name().to_owned(),
        badge: Badge::Unsigned,
        modified: documents::modified_seconds(in_hand.document()),
        last_used: now_in_seconds(),
        available: in_hand.reading_path().exists(),
        placement,
    }
}

/// Ahora mismo, en segundos desde la época.
fn now_in_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fixtures::a_memory;
    use crate::signing::PageSet;

    /// **Grada A**: ficheros de verdad en un directorio temporal, que es lo que
    /// hace falta para canonicalizar una ruta y para leer un `mtime`.
    fn a_pdf(directory: &Path, name: &str) -> std::path::PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, b"%PDF-1.7\n").expect("deberia escribirse");
        path
    }

    fn a_placement() -> PlacementView {
        PlacementView {
            rect: [10.0, 20.0, 210.0, 70.0],
            pages: PageSet::only_page(3),
        }
    }

    /// El recorrido local no cambia: el documento que se abrió por el diálogo
    /// deja su fila, con su recuadro, exactamente como antes.
    #[test]
    fn a_document_that_is_remembered_still_leaves_its_row() {
        let home = tempfile::tempdir().expect("deberia crearse");
        let memory = a_memory(home.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let path = a_pdf(home.path(), "contrato.pdf");
        let id = opened.remember(PortalDocument::opened(path));

        let row = take(&memory, &configuration, &opened, &id, Some(a_placement()))
            .expect("deberia ponerse delante");

        assert_eq!(row.name, "contrato.pdf");
        assert_eq!(row.placement, Some(a_placement()));
        assert_eq!(recents::listed_rows(&memory, &opened).len(), 1);
    }

    /// **TD-64**: el documento que no se recuerda no aparece en la bandeja y no
    /// deja colocación del recuadro, aunque la ventana lo haya arrastrado.
    #[test]
    fn a_document_that_is_not_remembered_leaves_neither_row_nor_placement() {
        let home = tempfile::tempdir().expect("deberia crearse");
        let memory = a_memory(home.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let path = a_pdf(home.path(), "de-la-sede.pdf");
        let id = opened.remember_unrecorded(PortalDocument::opened(path));

        let row = take(&memory, &configuration, &opened, &id, Some(a_placement()))
            .expect("deberia ponerse delante");

        assert_eq!(row.id, id);
        assert_eq!(row.name, "de-la-sede.pdf");
        assert!(recents::listed_rows(&memory, &opened).is_empty());
        let remembered = memory
            .state()
            .map(crate::memory::Loaded::into_value)
            .ok()
            .and_then(|state| state.visible_signature);
        assert_eq!(
            remembered, None,
            "el tamano del recuadro tampoco se recuerda"
        );
    }

    /// Y el mismo fichero abierto por los dos caminos no comparte destino: la
    /// concesión que se recuerda escribe, la que no, no.
    #[test]
    fn remembrance_belongs_to_the_grant_and_not_to_the_file() {
        let home = tempfile::tempdir().expect("deberia crearse");
        let memory = a_memory(home.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let path = a_pdf(home.path(), "contrato.pdf");
        let unrecorded = opened.remember_unrecorded(PortalDocument::opened(path.clone()));
        let remembered = opened.remember(PortalDocument::opened(path));

        take(&memory, &configuration, &opened, &unrecorded, None).expect("deberia ponerse delante");
        assert!(recents::listed_rows(&memory, &opened).is_empty());

        take(&memory, &configuration, &opened, &remembered, None).expect("deberia ponerse delante");
        assert_eq!(recents::listed_rows(&memory, &opened).len(), 1);
    }

    /// Un identificador que no es de esta sesión no pone nada delante.
    #[test]
    fn an_identifier_of_no_session_puts_nothing_in_hand() {
        let opened = OpenedDocuments::new();

        let taken = DocumentInHand::taken(&opened, "00000000000000000000000000000000");

        assert!(taken.is_err());
    }
}
