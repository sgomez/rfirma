use std::path::Path;
use std::sync::Mutex;

use super::*;
use crate::identity::application::certificates::listed_rows;
use crate::identity::application::tests::{NoMemory, NoToken, TestAuthority};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;

const CARD: &str = "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so";
const SOFTOKEN: &str = "/usr/lib/libsoftokn3.so";
const INSTALLED: &str = "/casa/ada/.local/share/rfirma/certificates";

type Shows = Box<dyn Fn(&Store) -> Vec<TokenCertificate> + Send + Sync>;

/// Un token que enseña lo que se le diga de cada almacén y apunta qué almacenes le piden listar.
struct Counting {
    asked: Mutex<Vec<Store>>,
    shows: Shows,
}

impl Counting {
    /// Un certificado en cada almacén.
    fn new() -> Self {
        Self::showing(|store| vec![a_certificate_in(store)])
    }

    fn showing(shows: impl Fn(&Store) -> Vec<TokenCertificate> + Send + Sync + 'static) -> Self {
        Self {
            asked: Mutex::new(Vec::new()),
            shows: Box::new(shows),
        }
    }

    fn asked(&self) -> Vec<Store> {
        std::mem::take(&mut *self.asked.lock().unwrap())
    }
}

fn a_certificate_in(store: &Store) -> TokenCertificate {
    let certificate = TestAuthority::root("Firmante de pruebas");
    TokenCertificate::new(
        CertificateRef::new(store.clone(), "token", "FIRMA", vec![0x01]),
        certificate.der(),
    )
}

impl Token for Counting {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.asked.lock().unwrap().push(store.clone());
        Ok((self.shows)(store))
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        NoToken.every_certificate(store)
    }

    fn list_authenticated(
        &self,
        store: &Store,
        pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        NoToken.list_authenticated(store, pin)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        NoToken.secret_of(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        NoToken.offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        NoToken.accepts_the_secret(reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        NoToken.sign_with_secret(reference, secret, algorithm, data)
    }

    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
        pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        NoToken.import_pkcs12(directory, pkcs12, password, pin)
    }

    fn remove_certificate(
        &self,
        directory: &Path,
        reference: &CertificateRef,
        pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        NoToken.remove_certificate(directory, reference, pin)
    }
}

struct Arranged {
    token: Counting,
    stores: Vec<Store>,
    listed: ListedCertificates,
    installed_copies: ListedCertificates,
    last: LastListing,
}

impl Arranged {
    fn new() -> Self {
        Self::over(
            Counting::new(),
            vec![
                Store::module(CARD),
                Store::nss(
                    SOFTOKEN,
                    Path::new("/casa/ada/.mozilla/firefox/ada.default"),
                ),
            ],
        )
    }

    fn over(token: Counting, stores: Vec<Store>) -> Self {
        Self {
            token,
            stores,
            listed: ListedCertificates::new(),
            installed_copies: ListedCertificates::new(),
            last: LastListing::default(),
        }
    }

    fn card_listing(&self) -> CardListing<'_> {
        CardListing {
            token: &self.token,
            stores: self.stores.clone(),
            installed_dir: Path::new(INSTALLED),
            listed: &self.listed,
            installed_copies: &self.installed_copies,
            memory: &NoMemory,
            last: &self.last,
        }
    }

    fn listed_in_full(&self) -> Vec<ListedCertificate> {
        listed_rows(
            &self.token,
            &self.stores,
            Path::new(INSTALLED),
            &self.listed,
            &self.installed_copies,
            &NoMemory,
            &self.last,
        )
        .expect("deberia listar")
    }
}

#[test]
fn after_a_full_listing_only_the_card_stores_are_opened_again_and_the_others_stay() {
    let arranged = Arranged::new();
    arranged.listed_in_full();
    arranged.token.asked();

    let (rows, ready) = arranged.card_listing().relisted();

    assert_eq!(arranged.token.asked(), vec![Store::module(CARD)]);
    assert_eq!(
        rows.as_ref().map(Vec::len),
        Some(2),
        "el de la tarjeta y el de Firefox: {rows:?}"
    );
    assert_eq!(ready, Some(ReadyCard::Other));
}

#[test]
fn a_full_listing_after_the_readers_relisted_keeps_the_handles_they_announced() {
    let arranged = Arranged::new();
    let (announced, _) = arranged.card_listing().relisted();

    let searched = arranged.listed_in_full();

    for row in announced.expect("la lista del lector") {
        assert!(
            arranged.listed.get(&row.id).is_some(),
            "la fila {} perdió su asa",
            row.label
        );
        assert!(searched.iter().any(|again| again.id == row.id));
    }
}

#[test]
fn a_window_that_mounts_late_reads_the_last_status_announced() {
    let now = ReaderNow::default();
    assert_eq!(now.status(), ReaderStatus::Unavailable);

    now.note(ReaderStatus::NoCard);

    assert_eq!(now.status(), ReaderStatus::NoCard);
}

#[test]
fn with_no_full_listing_yet_every_store_is_opened() {
    let arranged = Arranged::new();

    let (rows, _) = arranged.card_listing().relisted();

    assert_eq!(arranged.token.asked().len(), 2);
    assert_eq!(rows.map(|rows| rows.len()), Some(2));
}

#[test]
fn the_readers_are_watched_on_linux_and_windows_but_not_on_macos() {
    use crate::desktop::domain::channel::Channel;
    use crate::desktop::domain::platform::Platform;

    assert!(watches_the_readers(Platform::Linux, Channel::Native));
    assert!(watches_the_readers(Platform::Linux, Channel::Flatpak));
    assert!(watches_the_readers(Platform::Windows, Channel::Windows));
    assert!(!watches_the_readers(Platform::MacOs, Channel::Native));
}

const CARD_KSP: &str = "Microsoft Smart Card Key Storage Provider";
const SOFTWARE_KSP: &str = "Microsoft Software Key Storage Provider";
const ON_THE_DNIE: &str = "FIRMA DEL DNIE";
const IN_SOFTWARE: &str = "FIRMA EN EL DISCO";

fn the_windows_store() -> Store {
    Store::module("cng:CurrentUser/MY")
}

fn in_windows_with_its_key_in(provider: &str, label: &str, der: Vec<u8>) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(
            the_windows_store(),
            "CurrentUser\\MY",
            label,
            label.as_bytes().to_vec(),
        )
        .with_key_provider(provider),
        der,
    )
}

/// El Almacén de Windows: un certificado en software siempre y, mientras la tarjeta está dentro, los suyos.
struct WindowsWithACard {
    card_in: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl WindowsWithACard {
    fn of(on_the_card: Vec<TokenCertificate>) -> (Self, Counting) {
        let card_in = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shown = std::sync::Arc::clone(&card_in);
        let in_software = in_windows_with_its_key_in(
            SOFTWARE_KSP,
            IN_SOFTWARE,
            TestAuthority::root("Firmante en el disco").der(),
        );
        let token = Counting::showing(move |_store| {
            let mut found = vec![in_software.clone()];
            if shown.load(std::sync::atomic::Ordering::SeqCst) {
                found.extend(on_the_card.iter().cloned());
            }
            found
        });
        (Self { card_in }, token)
    }

    fn with_the_dnie() -> (Self, Counting) {
        Self::of(vec![in_windows_with_its_key_in(
            CARD_KSP,
            ON_THE_DNIE,
            crate::identity::application::tests::a_dnie_der("ADA LOVELACE", true),
        )])
    }

    fn put_in(&self, card_in: bool) {
        self.card_in
            .store(card_in, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Un vigilante que cuenta una sucesión de lectores, con la tarjeta dentro o fuera, y luego se para.
struct Scripted<'a> {
    card: &'a WindowsWithACard,
    steps: std::collections::VecDeque<bool>,
}

impl ReaderWatch for Scripted<'_> {
    fn next_change(&mut self) -> Option<Vec<Reader>> {
        let card_in = self.steps.pop_front()?;
        self.card.put_in(card_in);
        Some(vec![Reader {
            name: "Lector de pruebas".to_owned(),
            has_a_card: card_in,
        }])
    }
}

fn followed(arranged: &Arranged, card: &WindowsWithACard, steps: &[bool]) -> Vec<ReaderNews> {
    let heard = Mutex::new(Vec::new());
    let mut watch = Scripted {
        card,
        steps: steps.iter().copied().collect(),
    };
    follow_the_readers(&mut watch, &arranged.card_listing(), &|news| {
        heard.lock().unwrap().push(news);
    });
    heard.into_inner().unwrap()
}

fn labels_in(news: &ReaderNews) -> Vec<String> {
    let mut labels: Vec<String> = news
        .certificates
        .as_deref()
        .expect("la lista vuelta a listar")
        .iter()
        .map(|row| row.label.clone())
        .collect();
    labels.sort();
    labels
}

#[test]
fn putting_the_dnie_in_lists_the_windows_store_again_and_adds_its_card_certificates() {
    let (card, token) = WindowsWithACard::with_the_dnie();
    let arranged = Arranged::over(token, vec![the_windows_store()]);
    arranged.listed_in_full();
    arranged.token.asked();

    let heard = followed(&arranged, &card, &[false, true]);

    assert_eq!(
        heard.iter().map(|news| news.reader).collect::<Vec<_>>(),
        vec![
            ReaderStatus::NoCard,
            ReaderStatus::Reading,
            ReaderStatus::Ready(ReadyCard::Dnie),
        ]
    );
    assert_eq!(arranged.token.asked(), vec![the_windows_store()]);
    assert_eq!(labels_in(&heard[2]), vec![ON_THE_DNIE, IN_SOFTWARE]);
    let dnie = heard[2]
        .certificates
        .iter()
        .flatten()
        .find(|row| row.label == ON_THE_DNIE)
        .expect("la fila del DNIe");
    assert!(dnie.from_a_dnie);
}

#[test]
fn taking_the_dnie_out_removes_its_certificates_and_keeps_the_one_in_software() {
    let (card, token) = WindowsWithACard::with_the_dnie();
    let arranged = Arranged::over(token, vec![the_windows_store()]);

    let heard = followed(&arranged, &card, &[true, false]);

    let last = heard.last().expect("lo que se anuncia al sacarla");
    assert_eq!(last.reader, ReaderStatus::NoCard);
    assert_eq!(labels_in(last), vec![IN_SOFTWARE]);
}

#[test]
fn a_card_in_the_reader_with_no_card_class_certificate_in_windows_is_unreadable() {
    let (card, token) = WindowsWithACard::of(Vec::new());
    let arranged = Arranged::over(token, vec![the_windows_store()]);

    let heard = followed(&arranged, &card, &[true]);

    assert_eq!(
        heard.iter().map(|news| news.reader).collect::<Vec<_>>(),
        vec![ReaderStatus::Reading, ReaderStatus::Unreadable]
    );
    assert_eq!(labels_in(&heard[1]), vec![IN_SOFTWARE]);
}

#[test]
fn without_the_smart_card_service_there_is_no_reader_and_the_listing_goes_on() {
    struct NoService(bool);
    impl ReaderWatch for NoService {
        fn next_change(&mut self) -> Option<Vec<Reader>> {
            std::mem::take(&mut self.0).then(Vec::new)
        }
    }
    let (_card, token) = WindowsWithACard::with_the_dnie();
    let arranged = Arranged::over(token, vec![the_windows_store()]);
    let heard = Mutex::new(Vec::new());

    follow_the_readers(&mut NoService(true), &arranged.card_listing(), &|news| {
        heard.lock().unwrap().push(news);
    });

    assert_eq!(
        heard.into_inner().unwrap(),
        vec![ReaderNews {
            reader: ReaderStatus::NoReader,
            certificates: None,
        }]
    );
    let listed = arranged.listed_in_full();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].label, IN_SOFTWARE);
}
