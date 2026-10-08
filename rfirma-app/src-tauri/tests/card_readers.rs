//! La lista en caliente contra el módulo PKCS#11 falso como almacén de tarjeta, con un doble del vigilante de lectores (ADR-0048).

#![cfg(not(windows))]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

use fake_pkcs11::FakeCard;
use rfirma_lib::identity::adapters::pkcs11::RealToken;
use rfirma_lib::identity::application::certificates::{listed_rows, ListedCertificates};
use rfirma_lib::identity::application::readers::{
    follow_the_readers, follow_the_readers_apart, CardListing, LastListing, ReaderNews,
};
use rfirma_lib::identity::domain::certificate::{CertificateRef, ListedCertificate};
use rfirma_lib::identity::domain::readers::{Reader, ReaderStatus, ReadyCard};
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::ports::{CertificateMemory, ReaderWatch};
use rfirma_lib::memory_error::MemoryError;

const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";

/// Lo que pasa en el lector antes de que el vigilante lo cuente.
enum Happening<'a> {
    Nothing,
    PutIn(&'a FakeCard),
    TakeOut(&'a FakeCard),
}

/// Un vigilante que cuenta una sucesión de cambios y luego se para.
struct Scripted<'a> {
    steps: VecDeque<(Happening<'a>, Vec<Reader>)>,
}

impl<'a> Scripted<'a> {
    fn of(steps: Vec<(Happening<'a>, Vec<Reader>)>) -> Self {
        Self {
            steps: steps.into(),
        }
    }
}

impl ReaderWatch for Scripted<'_> {
    fn next_change(&mut self) -> Option<Vec<Reader>> {
        let (happening, readers) = self.steps.pop_front()?;
        match happening {
            Happening::Nothing => {}
            Happening::PutIn(card) => card.put_in().expect("la tarjeta deberia entrar"),
            Happening::TakeOut(card) => card.take_out().expect("la tarjeta deberia salir"),
        }
        Some(readers)
    }
}

struct NoMemory;

impl CertificateMemory for NoMemory {
    fn remembered_certificate(&self) -> Option<CertificateRef> {
        None
    }

    fn remember_the_certificate(&self, _reference: &CertificateRef) -> Result<(), MemoryError> {
        Ok(())
    }

    fn forget_the_certificate(&self) -> Result<(), MemoryError> {
        Ok(())
    }
}

/// Lo que haría falta para listar: el token de verdad sobre los almacenes de tarjeta dados.
struct Listing {
    stores: Vec<Store>,
    installed: tempfile::TempDir,
    listed: ListedCertificates,
    installed_copies: ListedCertificates,
    last: LastListing,
}

impl Listing {
    fn over(cards: &[&FakeCard]) -> Self {
        Self {
            stores: cards
                .iter()
                .map(|card| Store::module(card.module()))
                .collect(),
            installed: tempfile::tempdir().expect("deberia haber directorio temporal"),
            listed: ListedCertificates::new(),
            installed_copies: ListedCertificates::new(),
            last: LastListing::default(),
        }
    }

    fn card_listing(&self) -> CardListing<'_> {
        CardListing {
            token: &RealToken,
            stores: self.stores.clone(),
            installed_dir: self.installed.path(),
            listed: &self.listed,
            installed_copies: &self.installed_copies,
            memory: &NoMemory,
            last: &self.last,
        }
    }

    fn followed(&self, steps: Vec<(Happening<'_>, Vec<Reader>)>) -> Vec<ReaderNews> {
        let heard = Mutex::new(Vec::new());
        follow_the_readers(&mut Scripted::of(steps), &self.card_listing(), &|news| {
            heard.lock().unwrap().push(news);
        });
        heard.into_inner().unwrap()
    }
}

fn reader(has_a_card: bool) -> Vec<Reader> {
    vec![Reader {
        name: "Lector de pruebas".to_owned(),
        has_a_card,
    }]
}

fn statuses(heard: &[ReaderNews]) -> Vec<ReaderStatus> {
    heard.iter().map(|news| news.reader).collect()
}

fn labels(certificates: &[ListedCertificate]) -> Vec<&str> {
    certificates.iter().map(|row| row.label.as_str()).collect()
}

fn a_card_outside_the_reader() -> FakeCard {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    card.take_out().expect("la tarjeta deberia salir");
    card
}

#[test]
fn putting_the_card_in_reads_it_and_then_lists_its_certificates_as_a_ready_dnie() {
    let card = a_card_outside_the_reader();
    let listing = Listing::over(&[&card]);

    let heard = listing.followed(vec![
        (Happening::Nothing, reader(false)),
        (Happening::PutIn(&card), reader(true)),
    ]);

    assert_eq!(
        statuses(&heard),
        vec![
            ReaderStatus::NoCard,
            ReaderStatus::Reading,
            ReaderStatus::Ready(ReadyCard::Dnie),
        ]
    );
    assert_eq!(
        heard[1].certificates, None,
        "mientras se lee, la lista no cambia"
    );
    let listed = heard[2]
        .certificates
        .as_deref()
        .expect("la lista con la tarjeta");
    assert_eq!(labels(listed), vec![SIGNING_CERTIFICATE]);
    assert!(listed[0].from_a_dnie);
}

#[test]
fn taking_the_card_out_removes_its_certificates_and_leaves_the_reader_with_no_card() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let listing = Listing::over(&[&card]);

    let heard = listing.followed(vec![
        (Happening::Nothing, reader(true)),
        (Happening::TakeOut(&card), reader(false)),
    ]);

    assert_eq!(
        heard.last().map(|news| news.reader),
        Some(ReaderStatus::NoCard)
    );
    assert_eq!(
        heard.last().and_then(|news| news.certificates.clone()),
        Some(Vec::new()),
        "sus certificados tenían que irse de la lista"
    );
}

#[test]
fn a_card_that_no_card_store_shows_as_a_token_is_unreadable() {
    let card = a_card_outside_the_reader();
    let listing = Listing::over(&[&card]);

    let heard = listing.followed(vec![(Happening::Nothing, reader(true))]);

    assert_eq!(
        statuses(&heard),
        vec![ReaderStatus::Reading, ReaderStatus::Unreadable]
    );
}

#[test]
fn a_reader_arriving_empty_announces_its_status_without_listing_again() {
    let card = a_card_outside_the_reader();
    let listing = Listing::over(&[&card]);

    let heard = listing.followed(vec![
        (Happening::Nothing, Vec::new()),
        (Happening::Nothing, reader(false)),
    ]);

    assert_eq!(
        heard,
        vec![
            ReaderNews {
                reader: ReaderStatus::NoReader,
                certificates: None
            },
            ReaderNews {
                reader: ReaderStatus::NoCard,
                certificates: None
            },
        ]
    );
}

#[test]
fn opensc_is_not_initialised_again_when_the_card_comes_and_goes() {
    let card = a_card_outside_the_reader();
    let listing = Listing::over(&[&card]);

    listing.followed(vec![
        (Happening::Nothing, reader(false)),
        (Happening::PutIn(&card), reader(true)),
        (Happening::TakeOut(&card), reader(false)),
        (Happening::PutIn(&card), reader(true)),
    ]);

    assert_eq!(card.calls_to("C_Initialize").len(), 1, "{:?}", card.calls());
    assert!(card.calls_to("C_Finalize").is_empty(), "{:?}", card.calls());
}

#[test]
fn a_certificate_already_listed_keeps_its_handle_when_another_card_arrives() {
    let present = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let arriving = a_card_outside_the_reader();
    let listing = Listing::over(&[&present, &arriving]);
    let two_readers = |second: bool| {
        vec![
            Reader {
                name: "Uno".to_owned(),
                has_a_card: true,
            },
            Reader {
                name: "Dos".to_owned(),
                has_a_card: second,
            },
        ]
    };

    let heard = listing.followed(vec![
        (Happening::Nothing, two_readers(false)),
        (Happening::PutIn(&arriving), two_readers(true)),
    ]);

    let before = heard[1].certificates.as_deref().expect("la lista inicial");
    let after = heard[3]
        .certificates
        .as_deref()
        .expect("la lista con la segunda");
    assert!(after.len() > before.len(), "las dos tarjetas en la lista");
    for row in before {
        assert!(
            after.iter().any(|again| again.id == row.id),
            "la fila {} perdió su asa",
            row.label
        );
    }
    assert_eq!(heard[2].reader, ReaderStatus::Reading);
}

#[test]
fn the_search_of_a_window_that_mounts_after_the_reader_keeps_the_handles_it_announced() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let listing = Listing::over(&[&card]);
    let heard = listing.followed(vec![(Happening::Nothing, reader(true))]);
    let announced = heard
        .last()
        .and_then(|news| news.certificates.clone())
        .expect("la lista con la tarjeta");

    let searched = listed_rows(
        &RealToken,
        &listing.stores,
        listing.installed.path(),
        &listing.listed,
        &listing.installed_copies,
        &NoMemory,
        &listing.last,
    )
    .expect("la tarjeta falsa deberia listarse");

    assert!(!announced.is_empty());
    for row in &announced {
        assert!(
            listing.listed.get(&row.id).is_some(),
            "la fila {} perdió su asa",
            row.label
        );
        assert!(searched.iter().any(|again| again.id == row.id));
    }
}

#[test]
fn the_listing_runs_apart_from_the_thread_that_starts_it() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let listing = Arc::new(Listing::over(&[&card]));
    let heard_on: Arc<Mutex<Vec<ThreadId>>> = Arc::default();
    let started_on = std::thread::current().id();

    struct Once(Option<Vec<Reader>>);
    impl ReaderWatch for Once {
        fn next_change(&mut self) -> Option<Vec<Reader>> {
            self.0.take()
        }
    }

    follow_the_readers_apart(
        Box::new(Once(Some(reader(true)))),
        {
            let listing = Arc::clone(&listing);
            move |use_it| use_it(&listing.card_listing())
        },
        {
            let heard_on = Arc::clone(&heard_on);
            move |_news| heard_on.lock().unwrap().push(std::thread::current().id())
        },
    )
    .join()
    .expect("el hilo del vigilante deberia terminar");

    let heard_on = heard_on.lock().unwrap();
    assert_eq!(heard_on.len(), 2, "leyendo y lista");
    assert!(heard_on.iter().all(|thread| *thread != started_on));
}
