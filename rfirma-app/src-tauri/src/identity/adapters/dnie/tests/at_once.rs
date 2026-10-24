use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use rdnie::{Session, Transport};
use rdnie_emulator::EmulatedDnie;

use super::super::{dnie_store, CardOpener, DnieToken};
use super::OfThePolice;
use crate::identity::ports::Token;

/// Las sesiones vivas a la vez y el máximo que se ha llegado a ver.
#[derive(Default)]
struct Held {
    now: AtomicUsize,
    most: AtomicUsize,
}

/// El emulador, contando cuántas sesiones lo tienen a la vez y con cada orden algo lenta, como un lector.
struct Counted {
    card: EmulatedDnie,
    held: Arc<Held>,
}

impl Counted {
    fn new(card: EmulatedDnie, held: Arc<Held>) -> Self {
        let now = held.now.fetch_add(1, Ordering::SeqCst) + 1;
        held.most.fetch_max(now, Ordering::SeqCst);
        Self { card, held }
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.held.now.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Transport for Counted {
    fn transmit_raw(&mut self, apdu: &[u8]) -> Result<Vec<u8>, rdnie::Error> {
        thread::sleep(Duration::from_millis(1));
        self.card.transmit_raw(apdu)
    }

    fn atr(&self) -> Result<Vec<u8>, rdnie::Error> {
        self.card.atr()
    }

    fn reset(&mut self) -> Result<Vec<u8>, rdnie::Error> {
        self.card.reset()
    }

    fn transaction<R>(
        &mut self,
        body: impl FnOnce(&mut dyn Transport) -> Result<R, rdnie::Error>,
    ) -> Result<R, rdnie::Error> {
        body(self)
    }
}

struct CountedOpener(Arc<Held>);

impl CardOpener for CountedOpener {
    type Card = Counted;

    fn open(&self) -> Result<Session<Counted>, rdnie::Error> {
        let card = OfThePolice::new().on(EmulatedDnie::dnie_4());
        let root = card.component_root();
        Session::open_trusting(Counted::new(card, Arc::clone(&self.0)), root)
    }
}

#[test]
fn two_listings_at_once_take_turns_on_the_card() {
    let held = Arc::new(Held::default());
    let dnie = Arc::new(DnieToken::new(CountedOpener(Arc::clone(&held))));

    let listings: Vec<_> = (0..2)
        .map(|_| {
            let dnie = Arc::clone(&dnie);
            thread::spawn(move || dnie.list(&dnie_store()).expect("el DNIe deberia listarse"))
        })
        .collect();
    for listing in listings {
        assert_eq!(listing.join().expect("sin pánico").len(), 1);
    }

    assert_eq!(held.most.load(Ordering::SeqCst), 1);
}

/// Una tarjeta cuya primera apertura espera a que la prueba la suelte.
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
        let card = OfThePolice::new().on(EmulatedDnie::dnie_4());
        let root = card.component_root();
        Session::open_trusting(card, root)
    }
}

#[test]
fn a_listing_that_arrives_while_the_card_is_read_takes_that_reading() {
    let (inside, entered) = mpsc::channel();
    let (released, release) = mpsc::channel();
    let opened = Arc::new(AtomicUsize::new(0));
    let dnie = Arc::new(DnieToken::new(HeldOnTheFirstOpening {
        opened: Arc::clone(&opened),
        inside: Mutex::new(Some(inside)),
        release: Mutex::new(Some(release)),
    }));
    let listing = |dnie: &Arc<DnieToken<HeldOnTheFirstOpening>>| {
        let dnie = Arc::clone(dnie);
        thread::spawn(move || {
            let holders = dnie.list(&dnie_store()).expect("el DNIe deberia listarse");
            let every = dnie
                .every_certificate(&dnie_store())
                .expect("el DNIe deberia listarse");
            (holders.len(), every.len())
        })
    };

    let first = listing(&dnie);
    entered.recv().expect("la primera lectura deberia empezar");
    let second = listing(&dnie);
    thread::sleep(Duration::from_millis(100));
    released.send(()).expect("la primera lectura espera");

    assert_eq!(first.join().expect("sin pánico"), (1, 3));
    assert_eq!(second.join().expect("sin pánico"), (1, 3));
    assert_eq!(opened.load(Ordering::SeqCst), 2);
}

#[test]
fn a_listing_that_arrives_after_the_reading_reads_the_card_again() {
    let opened = Arc::new(AtomicUsize::new(0));
    let (inside, _entered) = mpsc::channel();
    let (released, release) = mpsc::channel();
    released.send(()).expect("suelta de antemano");
    let dnie = DnieToken::new(HeldOnTheFirstOpening {
        opened: Arc::clone(&opened),
        inside: Mutex::new(Some(inside)),
        release: Mutex::new(Some(release)),
    });

    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");
    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");

    assert_eq!(opened.load(Ordering::SeqCst), 2);
}

/// Cuenta las aperturas de una tarjeta emulada.
struct CountingOpener(Arc<AtomicUsize>);

impl CardOpener for CountingOpener {
    type Card = EmulatedDnie;

    fn open(&self) -> Result<Session<EmulatedDnie>, rdnie::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let card = OfThePolice::new().on(EmulatedDnie::dnie_4());
        let root = card.component_root();
        Session::open_trusting(card, root)
    }
}

#[test]
fn the_debug_report_waits_for_the_listing_that_holds_the_card() {
    let (inside, entered) = mpsc::channel();
    let (released, release) = mpsc::channel();
    let dnie = Arc::new(DnieToken::new(HeldOnTheFirstOpening {
        opened: Arc::new(AtomicUsize::new(0)),
        inside: Mutex::new(Some(inside)),
        release: Mutex::new(Some(release)),
    }));
    let listing = {
        let dnie = Arc::clone(&dnie);
        thread::spawn(move || dnie.list(&dnie_store()).expect("el DNIe deberia listarse"))
    };
    entered.recv().expect("la lectura deberia empezar");
    let opened_by_the_report = Arc::new(AtomicUsize::new(0));
    let report = {
        let opener = CountingOpener(Arc::clone(&opened_by_the_report));
        thread::spawn(move || super::super::seen_dnie(&opener))
    };

    thread::sleep(Duration::from_millis(100));
    assert_eq!(opened_by_the_report.load(Ordering::SeqCst), 0);
    released.send(()).expect("la lectura espera");
    listing.join().expect("sin pánico");
    report.join().expect("sin pánico");

    assert_eq!(opened_by_the_report.load(Ordering::SeqCst), 1);
}
