//! La lista en caliente: al cambiar un lector o una tarjeta, se vuelven a listar los almacenes de tarjeta y se anuncia el estado del lector (ADR-0048).

use std::path::Path;
use std::sync::Mutex;
use std::thread::JoinHandle;

use crate::identity::application::certificates::{
    on_the_desktop, rows_keeping_handles, ListedCertificates,
};
use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::identity::domain::readers::{status_of, Listing, Reader, ReaderStatus, ReadyCard};
use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::{CertificateMemory, ReaderWatch, Token};

/// Los certificados del último listado completo, para no volver a abrir más que los almacenes de tarjeta.
#[derive(Debug, Default)]
pub struct LastListing {
    found: Mutex<Option<Vec<TokenCertificate>>>,
}

impl LastListing {
    /// Guarda lo que acaba de encontrar un listado completo.
    pub fn keep(&self, found: &[TokenCertificate]) {
        *lock(&self.found) = Some(found.to_vec());
    }

    /// Los certificados del último listado que no son de tarjeta, si ya hubo alguno.
    fn other_than_cards(&self, installed_dir: &Path) -> Option<Vec<TokenCertificate>> {
        lock(&self.found).as_ref().map(|found| {
            found
                .iter()
                .filter(|certificate| !is_a_card(&certificate.reference().store(), installed_dir))
                .cloned()
                .collect()
        })
    }
}

/// Lo que la ventana recibe al cambiar los lectores: su estado y, si se ha vuelto a listar, la lista.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderNews {
    /// El estado que resume a todos los lectores.
    pub reader: ReaderStatus,
    /// La lista de siempre, con los certificados de las tarjetas que hay ahora; nada, si no ha cambiado.
    pub certificates: Option<Vec<ListedCertificate>>,
}

/// Lo que hace falta para volver a listar con las tarjetas de ahora.
pub struct CardListing<'a> {
    /// El token por el que se lista.
    pub token: &'a dyn Token,
    /// Todos los almacenes, también los que no son de tarjeta.
    pub stores: Vec<Store>,
    /// El directorio de los `.p12` instalados.
    pub installed_dir: &'a Path,
    /// Las asas del listado vivo.
    pub listed: &'a ListedCertificates,
    /// La copia instalada de cada fila del listado vivo.
    pub installed_copies: &'a ListedCertificates,
    /// El certificado recordado.
    pub memory: &'a dyn CertificateMemory,
    /// El último listado completo.
    pub last: &'a LastListing,
}

impl CardListing<'_> {
    /// La lista de siempre con los almacenes de tarjeta leídos de nuevo, y la tarjeta que ha enseñado certificados.
    pub fn relisted(&self) -> (Vec<ListedCertificate>, Option<ReadyCard>) {
        let (cards, others): (Vec<Store>, Vec<Store>) = self
            .stores
            .iter()
            .cloned()
            .partition(|store| is_a_card(store, self.installed_dir));
        let on_the_cards = listed_in(self.token, &cards);
        let ready = ready_card_among(&on_the_cards);
        let mut found = self
            .last
            .other_than_cards(self.installed_dir)
            .unwrap_or_else(|| listed_in(self.token, &others));
        found.extend(on_the_cards);
        self.last.keep(&found);
        let rows = rows_keeping_handles(
            on_the_desktop(found),
            self.installed_dir,
            self.listed,
            self.installed_copies,
            self.memory,
        );
        (rows, ready)
    }
}

/// Atiende cada cambio de los lectores hasta que el vigilante se para.
pub fn follow_the_readers(
    watch: &mut dyn ReaderWatch,
    listing: &CardListing<'_>,
    announce: &dyn Fn(ReaderNews),
) {
    let mut with_a_card: Option<Vec<String>> = None;
    let mut ready = None;
    while let Some(readers) = watch.next_change() {
        let now = names_with_a_card(&readers);
        let first = with_a_card.is_none();
        if with_a_card.replace(now.clone()).as_ref() == Some(&now) || (first && now.is_empty()) {
            announce(ReaderNews {
                reader: status_of(&readers, Listing::Done(ready)),
                certificates: None,
            });
            continue;
        }
        if !now.is_empty() {
            announce(ReaderNews {
                reader: status_of(&readers, Listing::InProgress),
                certificates: None,
            });
        }
        let (rows, found) = listing.relisted();
        ready = found;
        announce(ReaderNews {
            reader: status_of(&readers, Listing::Done(ready)),
            certificates: Some(rows),
        });
    }
}

/// Como `follow_the_readers`, en un hilo propio: el listado nunca corre en el de quien lo arranca.
pub fn follow_the_readers_apart(
    mut watch: Box<dyn ReaderWatch>,
    lend: impl Fn(&mut dyn FnMut(&CardListing<'_>)) + Send + 'static,
    announce: impl Fn(ReaderNews) + Send + 'static,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        lend(&mut |listing| follow_the_readers(watch.as_mut(), listing, &announce));
    })
}

fn names_with_a_card(readers: &[Reader]) -> Vec<String> {
    let mut names: Vec<String> = readers
        .iter()
        .filter(|reader| reader.has_a_card)
        .map(|reader| reader.name.clone())
        .collect();
    names.sort();
    names
}

/// Lo que enseñan los almacenes; uno que no se abre, o ninguno, no enseña nada.
fn listed_in(token: &dyn Token, stores: &[Store]) -> Vec<TokenCertificate> {
    if stores.is_empty() {
        return Vec::new();
    }
    token.list_across(stores).unwrap_or_default()
}

fn ready_card_among(on_the_cards: &[TokenCertificate]) -> Option<ReadyCard> {
    if on_the_cards.is_empty() {
        return None;
    }
    let a_dnie = on_the_cards.iter().any(TokenCertificate::is_from_a_dnie);
    Some(if a_dnie {
        ReadyCard::Dnie
    } else {
        ReadyCard::Other
    })
}

fn is_a_card(store: &Store, installed_dir: &Path) -> bool {
    store.class_under(installed_dir) == StoreClass::Card
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
