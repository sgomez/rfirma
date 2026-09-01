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
//! Bajo el arenero la aplicación no conoce la ruta original de un documento, y
//! la del portal es un enlace concedido para esta sesión que además no se puede
//! usar para nada más que leer (ver [`crate::destination::PortalDocument`]).
//! Mandarla a la ventana sería mandar una mentira, y el ADR-0011 lo prohíbe.
//! Lo que cruza es este identificador: **sin estructura**, sin nada del nombre
//! ni de la ruta dentro, y de él no se reconstruye ninguna.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::destination::PortalDocument;

/// Cuántos documentos se han acuñado en este proceso. Solo lo usa el amasado
/// de reserva de [`minted_without_the_system_csprng`], para que dos documentos
/// abiertos en el mismo instante no puedan colisionar.
static MINTED: AtomicU64 = AtomicU64::new(0);

/// Los documentos que se han abierto en esta sesión.
#[derive(Debug, Default)]
pub struct OpenedDocuments {
    documents: Mutex<HashMap<String, PortalDocument>>,
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
        let id = mint();
        lock(&self.documents).insert(id.clone(), document);
        id
    }

    /// El documento que se apuntó con ese identificador, si sigue apuntado.
    pub fn get(&self, id: &str) -> Option<PortalDocument> {
        lock(&self.documents).get(id).cloned()
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

/// Acuña un identificador opaco de 128 bits, en hexadecimal.
///
/// Los 128 bits salen del **CSPRNG del sistema** (`getrandom`), y **no se
/// derivan del documento**. Derivarlo de la ruta —un hash del nombre, por
/// ejemplo— dejaría que la ventana comprobara rutas por fuerza bruta contra el
/// identificador, que es exactamente la fuga que el ADR-0011 cierra.
///
/// No sirve amasarlo con `RandomState`: `std` siembra sus claves una vez por
/// hilo y cada `RandomState::new()` posterior se limita a incrementar una, así
/// que todos los identificadores de la sesión saldrían de la misma semilla más
/// un contador y dos consecutivos no serían independientes.
fn mint() -> String {
    match (getrandom::u64(), getrandom::u64()) {
        (Ok(high), Ok(low)) => format!("{high:016x}{low:016x}"),
        _ => minted_without_the_system_csprng(),
    }
}

/// Cuando el CSPRNG del sistema no responde —no debería pasar en Linux, pero
/// `getrandom` puede fallar— se vuelve al amasado de `RandomState` más un
/// contador de proceso.
///
/// Es peor —misma semilla por hilo, así que la entropía no crece con cada
/// acuñado— pero mantiene lo que de verdad importa aquí: **sigue sin llevar
/// nada del documento dentro** y sigue sin repetirse dentro del proceso, que
/// es lo que la tabla necesita. Un `panic!` en su lugar tumbaría la orden que
/// abre el documento por un fallo del que no se recupera nadie.
fn minted_without_the_system_csprng() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let counter = MINTED.fetch_add(1, Ordering::Relaxed);
    let half = |seed: u64| {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(counter);
        hasher.write_u64(seed);
        hasher.finish()
    };
    format!("{:016x}{:016x}", half(0), half(1))
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
