use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::x509::extension::BasicConstraints;
use openssl::x509::{X509Name, X509};
use rdnie::{Session, INTERMEDIATE_CA_LABEL, SIGNATURE_LABEL};
use rdnie_emulator::{EmulatedDnie, HolderCertificate, Step, DNIE_3_ATR, DNIE_4_ATR};
use rsa::traits::PublicKeyParts;

use super::{dnie_store, seen_dnie, CardOpener, DnieToken};
use crate::identity::application::certificates::{certificates_with_their_chains, on_the_desktop};
use crate::identity::application::readers::LastListing;
use crate::identity::application::tests::a_dnie_der;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::seen_dnie::DnieInTheReader;
use crate::identity::domain::store::StoreClass;
use crate::identity::ports::Token;

const AUTHENTICATION_LABEL: &str = "CertAutenticacion";

/// El emulador de rdnie como tarjeta: una nueva en cada apertura, con la sesión que confía en su raíz.
struct Emulated(Box<dyn Fn() -> EmulatedDnie + Send + Sync>);

impl CardOpener for Emulated {
    type Card = EmulatedDnie;

    fn open(&self) -> Result<Session<EmulatedDnie>, rdnie::Error> {
        let card = (self.0)();
        let root = card.component_root();
        Session::open_trusting(card, root)
    }
}

fn the_dnie(card: impl Fn() -> EmulatedDnie + Send + Sync + 'static) -> DnieToken<Emulated> {
    DnieToken::new(Emulated(Box::new(card)))
}

type Version = (&'static str, fn() -> EmulatedDnie);

fn the_two_versions() -> [Version; 2] {
    [
        ("DNIe 3.0", EmulatedDnie::dnie_3),
        ("DNIe 4.0", EmulatedDnie::dnie_4),
    ]
}

fn labels_of(found: &[TokenCertificate]) -> Vec<String> {
    let mut labels: Vec<String> = found
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    labels.sort();
    labels
}

fn modulus_of(der: &[u8]) -> Vec<u8> {
    X509::from_der(der)
        .and_then(|certificate| certificate.public_key())
        .and_then(|key| key.rsa())
        .expect("el certificado deberia traer una clave RSA")
        .n()
        .to_vec()
}

fn listed_by(token: &dyn Token) -> Vec<TokenCertificate> {
    token.list(&dnie_store()).expect("el DNIe deberia listarse")
}

#[test]
fn the_dnie_store_is_a_card() {
    assert_eq!(dnie_store().class(), StoreClass::Card);
}

#[test]
fn lists_the_signing_and_the_authentication_certificates_without_a_pin() {
    for (version, card) in the_two_versions() {
        let emulated = card();
        let signing_key = emulated.signing_public_key().n().to_bytes_be();
        let authentication_key = emulated.authentication_public_key().n().to_bytes_be();

        let found = listed_by(&the_dnie(card));

        assert_eq!(
            labels_of(&found),
            vec![AUTHENTICATION_LABEL, SIGNATURE_LABEL],
            "{version}"
        );
        let key_of = |label: &str| {
            found
                .iter()
                .find(|certificate| certificate.reference().label() == label)
                .map(|certificate| modulus_of(certificate.der()))
        };
        assert_eq!(key_of(SIGNATURE_LABEL), Some(signing_key), "{version}");
        assert_eq!(
            key_of(AUTHENTICATION_LABEL),
            Some(authentication_key),
            "{version}"
        );
        assert!(
            found
                .iter()
                .all(|certificate| certificate.reference().store() == dnie_store()),
            "{version}"
        );
    }
}

#[test]
fn another_card_in_the_reader_lists_nothing() {
    const ANOTHER_CARD: [u8; 11] = [
        0x3B, 0x8F, 0x80, 0x01, 0x80, 0x4F, 0x0C, 0xA0, 0x00, 0x00, 0x03,
    ];

    let found = listed_by(&the_dnie(|| EmulatedDnie::new(&ANOTHER_CARD)));

    assert!(found.is_empty());
}

#[test]
fn a_card_taken_out_while_listing_lists_nothing_and_says_the_card_is_gone() {
    use rdnie_emulator::Step;

    let failure = the_dnie(|| EmulatedDnie::dnie_3().removing_card_at(Step::Certificates))
        .list(&dnie_store())
        .expect_err("la tarjeta se ha ido");

    assert_eq!(
        failure.situation(),
        crate::identity::domain::error::Situation::TokenAbsent
    );
}

/// Los certificados de un DNIe de la Dirección General de la Policía, con la CA intermedia que los emite.
#[derive(Clone)]
struct OfThePolice {
    authentication: Vec<u8>,
    signing: Vec<u8>,
    intermediate: Vec<u8>,
}

impl OfThePolice {
    fn new() -> Self {
        Self {
            authentication: a_dnie_der("ADA LOVELACE", false),
            signing: a_dnie_der("ADA LOVELACE", true),
            intermediate: the_dnie_intermediate_der(),
        }
    }

    fn on(&self, card: EmulatedDnie) -> EmulatedDnie {
        let mut authentication = HolderCertificate::holder_authentication(1);
        authentication.der = self.authentication.clone();
        let mut signing = HolderCertificate::holder_signing(2);
        signing.der = self.signing.clone();
        let mut intermediate = HolderCertificate::new(3, 0)
            .labelled(INTERMEDIATE_CA_LABEL)
            .without_key();
        intermediate.der = self.intermediate.clone();
        card.with_certificates(&[authentication, signing, intermediate])
    }

    fn in_the(self, card: fn() -> EmulatedDnie) -> DnieToken<Emulated> {
        the_dnie(move || self.on(card()))
    }
}

/// La CA que `a_dnie_der` nombra como emisora, emitida a su vez por una raíz.
fn the_dnie_intermediate_der() -> Vec<u8> {
    let key = PKey::from_rsa(Rsa::generate(2048).expect("deberia generarse la clave"))
        .expect("deberia envolverse la clave");
    let mut subject = X509Name::builder().expect("deberia construirse un nombre");
    for (field, value) in [
        ("C", "ES"),
        ("O", "DIRECCION GENERAL DE LA POLICIA"),
        ("OU", "DNIE"),
        ("CN", "AC DNIE 004"),
    ] {
        subject
            .append_entry_by_text(field, value)
            .expect("el atributo deberia entrar");
    }
    let mut issuer = X509Name::builder().expect("deberia construirse un nombre");
    issuer
        .append_entry_by_text("CN", "AC RAIZ DNIE 2")
        .expect("el atributo deberia entrar");
    let mut builder = X509::builder().expect("deberia construirse un certificado");
    builder.set_version(2).expect("la version deberia ponerse");
    builder
        .set_serial_number(
            &BigNum::from_u32(4)
                .and_then(|serial| serial.to_asn1_integer())
                .expect("el serie deberia construirse"),
        )
        .expect("el serie deberia ponerse");
    builder
        .set_subject_name(&subject.build())
        .expect("el titular deberia ponerse");
    builder
        .set_issuer_name(&issuer.build())
        .expect("el emisor deberia ponerse");
    builder.set_pubkey(&key).expect("la clave deberia ponerse");
    builder
        .set_not_before(&Asn1Time::days_from_now(0).expect("deberia haber fecha"))
        .expect("el inicio deberia ponerse");
    builder
        .set_not_after(&Asn1Time::days_from_now(30).expect("deberia haber fecha"))
        .expect("el fin deberia ponerse");
    builder
        .append_extension(
            BasicConstraints::new()
                .critical()
                .ca()
                .build()
                .expect("deberia construirse la extension"),
        )
        .expect("la extension deberia ponerse");
    builder
        .sign(&key, MessageDigest::sha256())
        .expect("el certificado deberia firmarse");
    builder
        .build()
        .to_der()
        .expect("el certificado deberia salir en DER")
}

#[test]
fn the_intermediate_ca_completes_the_chain_of_both_certificates_for_a_site() {
    for (version, card) in the_two_versions() {
        let police = OfThePolice::new();
        let dnie = police.clone().in_the(card);

        let offered =
            certificates_with_their_chains(&dnie, &[dnie_store()], &LastListing::default())
                .expect("el DNIe deberia listarse");

        assert_eq!(
            labels_of(&offered),
            vec![AUTHENTICATION_LABEL, SIGNATURE_LABEL],
            "{version}"
        );
        for certificate in &offered {
            assert_eq!(
                certificate.chain(),
                vec![certificate.der().to_vec(), police.intermediate.clone()],
                "{version}"
            );
        }
    }
}

#[test]
fn a_full_listing_with_holders_and_chain_opens_the_card_once() {
    for (version, card) in the_two_versions() {
        let police = OfThePolice::new();
        let opened = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&opened);
        let dnie = the_dnie(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            police.on(card())
        });

        let offered =
            certificates_with_their_chains(&dnie, &[dnie_store()], &LastListing::default())
                .expect("el DNIe deberia listarse");

        assert_eq!(offered.len(), 2, "{version}");
        assert_eq!(opened.load(Ordering::SeqCst), 1, "{version}");
    }
}

#[test]
fn the_chain_of_a_later_listing_is_read_from_the_card_again() {
    let police = OfThePolice::new();
    let opened = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&opened);
    let dnie = the_dnie(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        police.on(EmulatedDnie::dnie_3())
    });

    certificates_with_their_chains(&dnie, &[dnie_store()], &LastListing::default())
        .expect("el DNIe deberia listarse");
    certificates_with_their_chains(&dnie, &[dnie_store()], &LastListing::default())
        .expect("el DNIe deberia listarse");

    assert_eq!(opened.load(Ordering::SeqCst), 2);
}

#[test]
fn the_signing_panel_offers_only_the_signing_certificate_of_the_dnie() {
    for (version, card) in the_two_versions() {
        let dnie = OfThePolice::new().in_the(card);

        let offered = on_the_desktop(listed_by(&dnie));

        assert_eq!(labels_of(&offered), vec![SIGNATURE_LABEL], "{version}");
        assert!(offered[0].is_from_a_dnie(), "{version}");
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

type SeenCase = (
    &'static str,
    fn() -> EmulatedDnie,
    &'static [u8],
    &'static str,
);

#[test]
fn describes_the_dnie_it_sees_by_its_atr_version_and_key_set() {
    use rdnie::KeyGeneration;

    let cases: [SeenCase; 2] = [
        ("3.0", EmulatedDnie::dnie_3, &DNIE_3_ATR, "segundo"),
        ("4.0", EmulatedDnie::dnie_4, &DNIE_4_ATR, "segundo"),
    ];
    for (version, card, atr, key_set) in cases {
        let DnieInTheReader::Seen(seen) = seen_dnie(&Emulated(Box::new(card))) else {
            panic!("{version}: hay un DNIe");
        };

        assert_eq!(seen.atr, hex(atr), "{version}");
        assert_eq!(seen.version, version);
        assert_eq!(seen.key_set, key_set, "{version}");
    }
    let DnieInTheReader::Seen(first) = seen_dnie(&Emulated(Box::new(|| {
        EmulatedDnie::dnie_3().with_key_generation(KeyGeneration::First)
    }))) else {
        panic!("hay un DNIe");
    };
    assert_eq!(first.key_set, "primero");
}

#[test]
fn sees_no_dnie_in_another_card() {
    const ANOTHER_CARD: [u8; 11] = [
        0x3B, 0x8F, 0x80, 0x01, 0x80, 0x4F, 0x0C, 0xA0, 0x00, 0x00, 0x03,
    ];

    assert_eq!(
        seen_dnie(&Emulated(Box::new(|| EmulatedDnie::new(&ANOTHER_CARD)))),
        DnieInTheReader::Absent
    );
}

#[test]
fn says_the_rdnie_error_of_a_dnie_that_fails_while_opening() {
    for (version, card) in the_two_versions() {
        let seen = seen_dnie(&Emulated(Box::new(move || {
            card().removing_card_at(Step::Opening)
        })));

        assert_eq!(
            seen,
            DnieInTheReader::Unopened("the card was removed".to_owned()),
            "{version}"
        );
    }
}

#[test]
fn a_card_that_does_not_respond_is_an_absent_token_with_the_message_of_rdnie() {
    let message = "The smart card is not responding to a reset";

    let error = super::token_error(&rdnie::Error::Transport(message.to_owned()));

    assert_eq!(
        error.situation(),
        crate::identity::domain::error::Situation::TokenAbsent
    );
    assert!(error.detail().contains(message), "{error}");
}

#[test]
fn the_rdnie_version_is_the_one_in_the_lockfile() {
    assert!(
        super::RDNIE_VERSION.starts_with("0."),
        "{}",
        super::RDNIE_VERSION
    );
}

const DNIE_2_ATR: [u8; 20] = [
    0x3B, 0x7F, 0x38, 0x00, 0x00, 0x00, 0x6A, 0x44, 0x4E, 0x49, 0x65, 0x10, 0x02, 0x4C, 0x34, 0x01,
    0x13, 0x03, 0x90, 0x00,
];

fn burned(atr: &[u8]) -> Vec<u8> {
    let mut burned = atr.to_vec();
    let end = burned.len();
    burned[end - 2..].copy_from_slice(&[0x65, 0x81]);
    burned
}

#[test]
fn rdnie_opens_a_dnie_3_and_a_dnie_4() {
    assert!(super::rdnie_opens(&DNIE_3_ATR));
    assert!(super::rdnie_opens(&DNIE_4_ATR));
}

#[test]
fn rdnie_leaves_a_dnie_2_to_pkcs11() {
    assert!(rdnie::is_dnie(&DNIE_2_ATR));
    assert!(!super::rdnie_opens(&DNIE_2_ATR));
}

#[test]
fn rdnie_leaves_a_burned_dnie_to_pkcs11() {
    let burned = burned(&DNIE_4_ATR);
    assert!(rdnie::is_dnie(&burned));
    assert!(!super::rdnie_opens(&burned));
}

mod at_once;
mod failures;
mod in_the_reader;
mod prompting;
mod removed_card;
mod signing;
