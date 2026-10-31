//! La lista en caliente: al cambiar un lector o una tarjeta, se vuelven a listar los almacenes de tarjeta y se anuncia el estado del lector (ADR-0048).

use std::path::Path;
use std::sync::Mutex;
use std::thread::JoinHandle;

use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::platform::Platform;
use crate::identity::application::certificates::{
    listed_rows, on_the_desktop, rows_keeping_handles, usable_certificate, with_their_chains,
    ListedCertificates,
};
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::readers::{
    listing_after, ready_card_among, status_of, Listing, Reader, ReaderStatus,
};
use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::{CertificateMemory, ReaderWatch, Relisting, Token};

/// Los certificados del último listado completo, para no volver a abrir más que los almacenes de tarjeta.
#[derive(Debug, Default)]
pub struct LastListing {
    found: Mutex<Option<Vec<TokenCertificate>>>,
    failed: Mutex<Vec<(Store, TokenError)>>,
}

impl LastListing {
    /// Guarda lo que acaba de encontrar un listado completo.
    pub fn keep(&self, found: &[TokenCertificate]) {
        *lock(&self.found) = Some(found.to_vec());
    }

    /// El último listado completo, si ya hubo alguno.
    pub fn found(&self) -> Option<Vec<TokenCertificate>> {
        lock(&self.found).clone()
    }

    /// El error con el que falló cada almacén la última vez que se listó.
    pub fn failures(&self) -> Vec<(Store, TokenError)> {
        lock(&self.failed).clone()
    }

    fn note_failures(&self, listed: &[Store], failed: &[(Store, TokenError)]) {
        let mut noted = lock(&self.failed);
        noted.retain(|(store, _)| !listed.contains(store));
        noted.extend(failed.iter().cloned());
    }

    /// Saca del último listado el certificado que ya no está, sin abrir ningún almacén.
    pub fn forget(&self, reference: &CertificateRef) {
        if let Some(found) = lock(&self.found).as_mut() {
            found.retain(|certificate| certificate.reference() != reference);
        }
    }

    /// Guarda lo de los almacenes que no son de tarjeta junto con las tarjetas que tenga ahora, y lo devuelve.
    fn replace_other_than_cards(
        &self,
        mut found: Vec<TokenCertificate>,
        installed_dir: &Path,
    ) -> Vec<TokenCertificate> {
        let mut last = lock(&self.found);
        if let Some(before) = last.as_deref() {
            found.extend(only_on_a_card(before, installed_dir));
        }
        *last = Some(found.clone());
        found
    }

    /// Los certificados del último listado de los almacenes que no enseñan tarjetas, si ya hubo alguno.
    fn other_than_cards(&self, installed_dir: &Path) -> Option<Vec<TokenCertificate>> {
        lock(&self.found).as_ref().map(|found| {
            found
                .iter()
                .filter(|certificate| !shows_cards(&certificate.reference().store(), installed_dir))
                .cloned()
                .collect()
        })
    }
}

/// Los certificados de los almacenes, apuntando en `last` el error de cada uno que no se abre; falla solo si no se abre ninguno.
pub fn heard_across(
    token: &dyn Token,
    stores: &[Store],
    last: &LastListing,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let listed = token.list_each(stores);
    last.note_failures(stores, listed.failed());
    listed.certificates()
}

/// El certificado del asa, tomado del último listado; solo si no hay ninguno lista los almacenes y lo guarda (ADR-0011).
pub fn chosen_from_the_last_listing(
    token: &dyn Token,
    stores: &[Store],
    last: &LastListing,
    listed: &ListedCertificates,
    handle: &str,
) -> Result<TokenCertificate, TokenError> {
    let found = match last.found() {
        Some(found) => found,
        None => {
            let found = with_their_chains(token, heard_across(token, stores, last)?);
            last.keep(&found);
            found
        }
    };
    usable_certificate(&found, &last.failures(), handle, listed).cloned()
}

/// Las filas del escritorio de lo encontrado, con las asas conservadas y las instaladas marcadas.
pub fn rows_marking_the_installed(
    token: &dyn Token,
    found: Vec<TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    installed_copies: &ListedCertificates,
    memory: &dyn CertificateMemory,
) -> Vec<ListedCertificate> {
    rows_keeping_handles(
        on_the_desktop(found),
        installed_dir,
        listed,
        installed_copies,
        memory,
    )
    .into_iter()
    .map(|mut row| {
        row.installed = row.installed
            || listed
                .get(&row.id)
                .is_some_and(|reference| token.installed_in_the_system(installed_dir, &reference));
        row
    })
    .collect()
}

/// Si esta plataforma y este canal pueden vigilar lectores (ADR-0048).
pub fn watches_the_readers(platform: Platform, channel: Channel) -> bool {
    match platform {
        Platform::Linux => matches!(channel, Channel::Native | Channel::Flatpak),
        Platform::Windows => channel == Channel::Windows,
        Platform::MacOs => channel == Channel::MacOs,
    }
}

/// El último estado anunciado de los lectores, para la ventana que se monta después del anuncio.
#[derive(Debug)]
pub struct ReaderNow {
    status: Mutex<ReaderStatus>,
}

impl Default for ReaderNow {
    fn default() -> Self {
        Self {
            status: Mutex::new(ReaderStatus::Unavailable),
        }
    }
}

impl ReaderNow {
    /// Apunta el estado que se acaba de anunciar.
    pub fn note(&self, status: ReaderStatus) {
        *lock(&self.status) = status;
    }

    /// El último estado anunciado; sin vigilante, no soportado.
    pub fn status(&self) -> ReaderStatus {
        *lock(&self.status)
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
    /// Al recuperar el foco: abre solo los almacenes que no son de tarjeta y toma el resto del último listado.
    pub fn relisted_other_than_cards(&self) -> Result<Vec<ListedCertificate>, TokenError> {
        if self.last.found().is_none() {
            return listed_rows(
                self.token,
                &self.stores,
                self.installed_dir,
                self.listed,
                self.installed_copies,
                self.memory,
                self.last,
            );
        }
        let others: Vec<Store> = self
            .stores
            .iter()
            .filter(|store| store.class_under(self.installed_dir) != StoreClass::Card)
            .cloned()
            .collect();
        let found = if others.is_empty() {
            Vec::new()
        } else {
            with_their_chains(self.token, heard_across(self.token, &others, self.last)?)
        };
        let found = self
            .last
            .replace_other_than_cards(found, self.installed_dir);
        Ok(rows_marking_the_installed(
            self.token,
            found,
            self.installed_dir,
            self.listed,
            self.installed_copies,
            self.memory,
        ))
    }
}

impl Relisting for CardListing<'_> {
    fn relisted(&self) -> (Option<Vec<ListedCertificate>>, Listing) {
        let (cards, others): (Vec<Store>, Vec<Store>) = self
            .stores
            .iter()
            .cloned()
            .partition(|store| shows_cards(store, self.installed_dir));
        let on_the_cards = listed_in(self.token, &cards, self.last);
        let ready = ready_card_among(&only_on_a_card(&on_the_cards, self.installed_dir));
        let listing = listing_after(ready, || self.token.unsupported_dnie());
        let mut found = self
            .last
            .other_than_cards(self.installed_dir)
            .unwrap_or_else(|| listed_in(self.token, &others, self.last));
        found.extend(on_the_cards);
        self.last.keep(&found);
        let rows = rows_keeping_handles(
            on_the_desktop(found),
            self.installed_dir,
            self.listed,
            self.installed_copies,
            self.memory,
        );
        (Some(rows), listing)
    }
}

/// Atiende cada cambio de los lectores hasta que el vigilante se para.
pub fn follow_the_readers(
    watch: &mut dyn ReaderWatch,
    listing: &dyn Relisting,
    announce: &dyn Fn(ReaderNews),
) {
    let mut with_a_card: Option<Vec<String>> = None;
    let mut finished = Listing::Done(None);
    while let Some(readers) = watch.next_change() {
        let now = names_with_a_card(&readers);
        let first = with_a_card.is_none();
        if with_a_card.replace(now.clone()).as_ref() == Some(&now) || (first && now.is_empty()) {
            announce(ReaderNews {
                reader: status_of(&readers, finished),
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
        finished = found;
        announce(ReaderNews {
            reader: status_of(&readers, finished),
            certificates: rows,
        });
    }
}

/// Como `follow_the_readers`, en un hilo propio: el listado nunca corre en el de quien lo arranca.
pub fn follow_the_readers_apart(
    mut watch: Box<dyn ReaderWatch>,
    lend: impl Fn(&mut dyn FnMut(&dyn Relisting)) + Send + 'static,
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
fn listed_in(token: &dyn Token, stores: &[Store], last: &LastListing) -> Vec<TokenCertificate> {
    with_their_chains(token, heard_across(token, stores, last).unwrap_or_default())
}

/// Si el almacén se vuelve a listar al cambiar los lectores: uno de tarjeta, o el de Windows o macOS, que copian las suyas.
fn shows_cards(store: &Store, installed_dir: &Path) -> bool {
    matches!(
        store.class_under(installed_dir),
        StoreClass::Card | StoreClass::Windows | StoreClass::Macos
    )
}

/// Si el certificado es de una tarjeta; los `.p12` instalados también son módulos, y no lo son.
fn is_on_a_card(reference: &CertificateRef, installed_dir: &Path) -> bool {
    reference.class_under(installed_dir) == StoreClass::Card
}

/// Los certificados que son de una tarjeta, de entre los que enseñan los almacenes.
pub fn only_on_a_card(found: &[TokenCertificate], installed_dir: &Path) -> Vec<TokenCertificate> {
    found
        .iter()
        .filter(|certificate| is_on_a_card(certificate.reference(), installed_dir))
        .cloned()
        .collect()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
