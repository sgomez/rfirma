use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rdnie::{Session, SIGNATURE_LABEL};
use rdnie_emulator::{EmulatedDnie, DEFAULT_PIN};
use rsa::Pkcs1v15Sign;
use sha2::{Digest, Sha256};

use super::{and_the_dnie_store, WithTheDnie};
use crate::identity::adapters::dnie::{dnie_store, CardOpener, DnieToken};
use crate::identity::application::tests::{NoToken, TestAuthority};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

const AUTHENTICATION_LABEL: &str = "CertAutenticacion";
const PLATFORM_LABEL: &str = "DE LA PLATAFORMA";
const PLATFORM_SIGNATURE: [u8; 2] = [0xAB, 0xCD];
const SIGNED: &[u8] = b"los atributos firmados de un documento";

/// Un DNIe emulado nuevo en cada apertura, y cuántas veces se ha abierto.
struct Emulated {
    card: fn() -> EmulatedDnie,
    opened: Arc<AtomicUsize>,
}

impl CardOpener for Emulated {
    type Card = EmulatedDnie;

    fn open(&self) -> Result<Session<EmulatedDnie>, rdnie::Error> {
        self.opened.fetch_add(1, Ordering::SeqCst);
        let card = (self.card)();
        let root = card.component_root();
        Session::open_trusting(card, root)
    }
}

/// El token de una plataforma que apunta cada almacén que se le pide.
#[derive(Clone, Default)]
struct Platform {
    asked: Arc<Mutex<Vec<Store>>>,
}

impl Platform {
    fn asked(&self) -> Vec<Store> {
        self.asked.lock().expect("sin envenenar").clone()
    }

    fn heard(&self, store: &Store) {
        self.asked
            .lock()
            .expect("sin envenenar")
            .push(store.clone());
    }
}

impl Token for Platform {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.heard(store);
        Ok(vec![TokenCertificate::new(
            CertificateRef::new(store.clone(), "plataforma", PLATFORM_LABEL, vec![0x01]),
            TestAuthority::root("Firmante de la plataforma").der(),
        )])
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.list(store)
    }

    fn list_authenticated(
        &self,
        store: &Store,
        _pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        self.list(store)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        self.heard(&reference.store());
        NoToken.secret_of(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        self.heard(&reference.store());
        NoToken.offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        self.heard(&reference.store());
        NoToken.accepts_the_secret(reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        _secret: &ProtectedSecret,
        _algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        self.heard(&reference.store());
        Ok(PLATFORM_SIGNATURE.to_vec())
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
    token: WithTheDnie<Platform>,
    platform: Platform,
    opened: Arc<AtomicUsize>,
}

fn with_the_dnie(card: fn() -> EmulatedDnie) -> Arranged {
    let platform = Platform::default();
    let opened = Arc::new(AtomicUsize::new(0));
    let dnie = DnieToken::new(Emulated {
        card,
        opened: Arc::clone(&opened),
    });
    Arranged {
        token: WithTheDnie::new(platform.clone(), dnie),
        platform,
        opened,
    }
}

fn another_store() -> Store {
    Store::module("/usr/lib/another.so")
}

fn labels_of(found: &[TokenCertificate]) -> Vec<String> {
    let mut labels: Vec<String> = found
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    labels.sort();
    labels
}

fn the_signing_certificate() -> CertificateRef {
    CertificateRef::new(dnie_store(), "DNIe", SIGNATURE_LABEL, None)
}

#[test]
fn the_dnie_store_goes_to_rdnie() {
    let arranged = with_the_dnie(EmulatedDnie::dnie_4);

    let listed = arranged
        .token
        .list(&dnie_store())
        .expect("el DNIe deberia listarse");

    assert_eq!(
        labels_of(&listed),
        vec![AUTHENTICATION_LABEL, SIGNATURE_LABEL]
    );
    assert_eq!(arranged.platform.asked(), Vec::<Store>::new());
}

#[test]
fn any_other_store_goes_to_the_wrapped_token() {
    let arranged = with_the_dnie(EmulatedDnie::dnie_4);
    let another = another_store();
    let reference = CertificateRef::new(another.clone(), "plataforma", PLATFORM_LABEL, vec![0x01]);

    let listed = arranged.token.list(&another).expect("deberia listarse");
    let every = arranged
        .token
        .every_certificate(&another)
        .expect("deberia listarse");
    let signature = arranged
        .token
        .sign_with_secret(
            &reference,
            &ProtectedSecret::from_str("1234"),
            SignatureAlgorithm::Sha256Rsa,
            SIGNED,
        )
        .expect("la plataforma deberia firmar");

    assert_eq!(labels_of(&listed), vec![PLATFORM_LABEL]);
    assert_eq!(labels_of(&every), vec![PLATFORM_LABEL]);
    assert_eq!(signature, PLATFORM_SIGNATURE.to_vec());
    assert_eq!(
        arranged.platform.asked(),
        vec![another.clone(), another.clone(), another]
    );
    assert_eq!(arranged.opened.load(Ordering::SeqCst), 0);
}

fn a_dnie_with_two_attempts_left() -> EmulatedDnie {
    EmulatedDnie::dnie_4().with_pin_retries(2)
}

#[test]
fn the_dnie_copy_signs_through_rdnie_with_its_attempts_left() {
    let arranged = with_the_dnie(a_dnie_with_two_attempts_left);
    let key = a_dnie_with_two_attempts_left().signing_public_key();

    let warning = arranged.token.pin_warning(&the_signing_certificate());
    let signature = arranged
        .token
        .sign_with_secret(
            &the_signing_certificate(),
            &ProtectedSecret::from_str(DEFAULT_PIN),
            SignatureAlgorithm::Sha256Rsa,
            SIGNED,
        )
        .expect("el DNIe deberia firmar");

    assert_eq!(warning, Ok(PinWarning::AttemptsLeft(2)));
    assert!(key
        .verify(
            Pkcs1v15Sign::new::<Sha256>(),
            &Sha256::digest(SIGNED),
            &signature
        )
        .is_ok());
    assert_eq!(arranged.platform.asked(), Vec::<Store>::new());
}

#[test]
fn the_dnie_store_comes_after_the_platform_stores() {
    let first = Store::module("/usr/lib/first.so");
    let second = another_store();

    assert_eq!(
        and_the_dnie_store(vec![first.clone(), second.clone()]),
        vec![first, second, dnie_store()]
    );
}
