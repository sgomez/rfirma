//! **Los documentos abiertos en esta sesión**: del identificador opaco al
//! documento del portal (ID-61).
//!
//! No es una memoria que sobreviva a nada —vive mientras vive el proceso, y no
//! toca ningún fichero— pero está aquí por lo mismo que el resto del módulo:
//! es lo que la aplicación sabe y la ventana no. Mismo patrón que el ciclo de
//! firma a medias de [`crate::commands::SigningSession`]: **lo que la ventana
//! no tiene, no lo puede filtrar**.
//!
//! # Por qué un identificador y no la ruta
//!
//! Bajo el sandbox la aplicación no conoce la ruta original de un documento, y
//! la del portal es un enlace concedido para esta sesión que además no se puede
//! usar para nada más que leer (ver [`crate::destination::PortalDocument`]).
//! Mandarla a la ventana sería mandar una mentira, y el ADR-0011 lo prohíbe.
//! Lo que cruza es este identificador: **sin estructura**, sin nada del nombre
//! ni de la ruta dentro, y de él no se reconstruye ninguna.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::handles::mint;
use crate::destination::PortalDocument;

/// **Si de este documento se guarda rastro** (ID-286, ID-287).
///
/// Es una propiedad de la concesión, no del fichero: el mismo PDF puede
/// entrar por el diálogo —y entonces se recuerda— y llegar mandado por una
/// sede —y entonces no—. Por eso vive junto al documento abierto y no en la
/// bandeja: cuando la bandeja tiene que decidir si escribe, ya es tarde para
/// preguntarle a nadie por dónde entró.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remembrance {
    /// El recorrido local: fila en la bandeja, colocación del recuadro y
    /// carpeta de la que salió.
    Remembered,
    /// **Nada de eso** (ID-286): ni fila, ni colocación, ni «último
    /// documento». Es lo que la sede manda a firmar.
    Unrecorded,
}

/// Los documentos que se han abierto en esta sesión.
#[derive(Debug, Default)]
pub struct OpenedDocuments {
    /// El orden de llegada junto al documento: sin él, dos concesiones de la
    /// misma ruta no se podrían distinguir por antigüedad y
    /// [`OpenedDocuments::last_id_of`] devolvería una cualquiera.
    documents: Mutex<HashMap<String, Grant>>,
    granted: AtomicU64,
}

/// Una concesión apuntada: cuándo llegó, qué documento es y si se recuerda.
#[derive(Debug)]
struct Grant {
    order: u64,
    document: PortalDocument,
    remembrance: Remembrance,
}

impl OpenedDocuments {
    /// Vacío, que es como arranca la aplicación.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apunta un documento recién abierto y devuelve **su identificador**.
    ///
    /// El mismo documento abierto dos veces recibe dos identificadores
    /// distintos, y eso es lo correcto: el identificador nombra una concesión
    /// del portal, no un fichero del disco del usuario.
    pub fn remember(&self, document: PortalDocument) -> String {
        self.grant(document, Remembrance::Remembered)
    }

    /// Apunta un documento **del que no se guarda rastro** (ID-286).
    ///
    /// Devuelve un identificador igual que [`OpenedDocuments::remember`] —se
    /// lee por el mismo sitio y se firma por el mismo recorrido—, pero lo que
    /// se apunte con él no dejará fila en la bandeja.
    pub fn remember_unrecorded(&self, document: PortalDocument) -> String {
        self.grant(document, Remembrance::Unrecorded)
    }

    fn grant(&self, document: PortalDocument, remembrance: Remembrance) -> String {
        let id = mint();
        let order = self.granted.fetch_add(1, Ordering::Relaxed);
        lock(&self.documents).insert(
            id.clone(),
            Grant {
                order,
                document,
                remembrance,
            },
        );
        id
    }

    /// El documento que se apuntó con ese identificador, si sigue apuntado.
    pub fn get(&self, id: &str) -> Option<PortalDocument> {
        lock(&self.documents)
            .get(id)
            .map(|grant| grant.document.clone())
    }

    /// Si de ese documento se guarda rastro, si sigue apuntado.
    pub fn remembrance(&self, id: &str) -> Option<Remembrance> {
        lock(&self.documents).get(id).map(|grant| grant.remembrance)
    }

    /// El identificador **más reciente** que se apuntó para esa ruta de
    /// lectura, si hay alguno.
    ///
    /// Lo necesita la bandeja en disco: sus filas se guardan por ruta y lo que
    /// cruza a la ventana es un identificador, así que listarlas acuñaría uno
    /// nuevo para el documento que la ventana ya tiene delante y la fila
    /// activa dejaría de reconocerse. El más reciente y no uno cualquiera
    /// porque el orden de un `HashMap` no es orden.
    ///
    /// Solo mira las concesiones que **se recuerdan**: una fila de la bandeja
    /// nunca es un documento de sede, así que prestarle el identificador de
    /// uno sería darle a la ventana el asa de algo que no está en la lista.
    pub fn last_id_of(&self, reading_path: &Path) -> Option<String> {
        lock(&self.documents)
            .iter()
            .filter(|(_, grant)| grant.remembrance == Remembrance::Remembered)
            .filter(|(_, grant)| grant.document.reading_path() == reading_path)
            .max_by_key(|(_, grant)| grant.order)
            .map(|(id, _)| id.clone())
    }

    /// Cuántos hay apuntados.
    pub fn len(&self) -> usize {
        lock(&self.documents).len()
    }

    /// Si no hay ninguno.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: el registro es puro —una tabla en memoria y un acuñado— y
    /// se prueba en el carril rápido (TD-18). Abrir un diálogo de verdad no se
    /// prueba aquí: en el CI no hay portal.
    const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

    #[test]
    fn an_opened_document_comes_back_by_its_identifier() {
        let opened = OpenedDocuments::new();

        let id = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(
            opened.get(&id),
            Some(PortalDocument::opened(A_PORTAL_HANDLE))
        );
    }

    #[test]
    fn an_identifier_nobody_minted_is_simply_not_there() {
        let opened = OpenedDocuments::new();

        assert_eq!(opened.get("00000000000000000000000000000000"), None);
        assert!(opened.is_empty());
    }

    #[test]
    fn documents_opened_one_after_another_all_stay_open() {
        let opened = OpenedDocuments::new();

        let first = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        let second = opened.remember(PortalDocument::opened(
            "/run/user/1000/doc/2f9c94ca/factura.pdf",
        ));

        assert_ne!(first, second);
        assert_eq!(opened.len(), 2);
        assert_eq!(
            opened
                .get(&first)
                .map(|document| document.name().to_owned()),
            Some("contrato.pdf".to_owned())
        );
        assert_eq!(
            opened
                .get(&second)
                .map(|document| document.name().to_owned()),
            Some("factura.pdf".to_owned())
        );
    }

    /// **ID-286**: de qué documentos se guarda rastro se decide al apuntarlos,
    /// y es una propiedad de la concesión y no del fichero.
    #[test]
    fn a_grant_says_whether_the_document_it_stands_for_is_remembered() {
        let opened = OpenedDocuments::new();

        let remembered = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        let unrecorded = opened.remember_unrecorded(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(
            opened.remembrance(&remembered),
            Some(Remembrance::Remembered)
        );
        assert_eq!(
            opened.remembrance(&unrecorded),
            Some(Remembrance::Unrecorded)
        );
        assert_eq!(opened.remembrance("00000000000000000000000000000000"), None);
    }

    /// Y una fila de la bandeja nunca toma prestado el identificador de una
    /// concesión que no se recuerda: en la lista no hay documentos de sede.
    #[test]
    fn the_tray_never_borrows_the_identifier_of_a_document_that_is_not_remembered() {
        let opened = OpenedDocuments::new();
        let remembered = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        opened.remember_unrecorded(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(
            opened.last_id_of(Path::new(A_PORTAL_HANDLE)),
            Some(remembered)
        );
    }

    /// La invariante del ADR-0011 en el sitio donde se acuña: del
    /// identificador no sale ni un trozo de la ruta, ni del nombre, ni un
    /// separador por el que empezar a adivinarla.
    #[test]
    fn the_identifier_carries_nothing_of_the_path_it_stands_for() {
        let opened = OpenedDocuments::new();

        let id = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
        for leak in ["/", "run", "doc", "1e8b83b9", "contrato", "pdf"] {
            assert!(
                !id.contains(leak),
                "el identificador «{id}» lleva «{leak}» dentro"
            );
        }
    }

    /// Y el mismo documento abierto dos veces no acuña el mismo
    /// identificador: nombra la concesión del portal, no el fichero.
    #[test]
    fn the_same_document_opened_twice_is_minted_twice() {
        let opened = OpenedDocuments::new();

        let first = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        let second = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_ne!(first, second);
    }
}
