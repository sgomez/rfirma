use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use rdnie::Session;
use rdnie_emulator::EmulatedDnie;

use super::super::WithTheDnie;
use super::{another_store, Platform};
use crate::identity::adapters::dnie::{dnie_store, CardOpener, DnieToken};
use crate::identity::application::certificates::certificates_by_class;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

/// Un DNIe cuya primera apertura avisa y espera a que la prueba la suelte.
struct HeldOnTheFirstOpening {
    opened: Arc<AtomicUsize>,
    inside: Mutex<Option<mpsc::Sender<()>>>,
    release: Mutex<Option<mpsc::Receiver<()>>>,
}

impl CardOpener for HeldOnTheFirstOpening {
    type Card = EmulatedDnie;

    fn open(&self) -> Result<Session<EmulatedDnie>, rdnie::Error> {
        if self.opened.fetch_add(1, Ordering::SeqCst) == 0 {
            let inside = self.inside.lock().expect("sin envenenar").take();
            inside
                .expect("primera apertura")
                .send(())
                .expect("la prueba escucha");
            let release = self.release.lock().expect("sin envenenar").take();
            release
                .expect("primera apertura")
                .recv()
                .expect("la prueba suelta");
        }
        let card = EmulatedDnie::dnie_4();
        let root = card.component_root();
        Session::open_trusting(card, root)
    }
}

/// Cuántas veces se abre el DNIe si `listing` empieza mientras otra lectura lo tiene y llega a él después.
fn openings_with(listing: fn(&dyn Token, &[Store]) -> usize, expected: usize) -> usize {
    let (inside, entered) = mpsc::channel();
    let (released, release) = mpsc::channel();
    let (platform_released, platform_release) = mpsc::channel();
    let opened = Arc::new(AtomicUsize::new(0));
    let platform = Platform::default();
    *platform.held.lock().expect("sin envenenar") = Some(platform_release);
    let token = Arc::new(WithTheDnie::new(
        platform,
        DnieToken::new(HeldOnTheFirstOpening {
            opened: Arc::clone(&opened),
            inside: Mutex::new(Some(inside)),
            release: Mutex::new(Some(release)),
        }),
    ));

    let reading = {
        let token = Arc::clone(&token);
        thread::spawn(move || token.list(&dnie_store()).expect("el DNIe deberia listarse"))
    };
    entered.recv().expect("la lectura deberia empezar");
    let later = {
        let token = Arc::clone(&token);
        thread::spawn(move || listing(token.as_ref(), &[another_store(), dnie_store()]))
    };
    thread::sleep(Duration::from_millis(100));
    released.send(()).expect("la lectura espera");
    reading.join().expect("sin pánico");
    platform_released.send(()).expect("el listado espera");

    assert_eq!(later.join().expect("sin pánico"), expected);
    opened.load(Ordering::SeqCst)
}

#[test]
fn a_listing_begun_while_the_dnie_is_read_takes_that_reading_though_it_reaches_the_dnie_later() {
    let openings = openings_with(
        |token, stores| token.list_each(stores).certificates().expect("lista").len(),
        3,
    );

    assert_eq!(openings, 1);
}

#[test]
fn the_status_count_begun_while_the_dnie_is_read_takes_that_reading() {
    let openings = openings_with(
        |token, stores| {
            certificates_by_class(token, stores, Path::new("/sin/instalados"))
                .iter()
                .map(|(_, certificates)| certificates)
                .sum()
        },
        3,
    );

    assert_eq!(openings, 1);
}
