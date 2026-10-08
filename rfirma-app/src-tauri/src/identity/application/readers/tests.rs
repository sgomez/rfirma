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

/// Un token con un certificado en cada almacén, que apunta qué almacenes le piden listar.
struct Counting {
    asked: Mutex<Vec<Store>>,
}

impl Counting {
    fn new() -> Self {
        Self {
            asked: Mutex::new(Vec::new()),
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
        Ok(vec![a_certificate_in(store)])
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
        Self {
            token: Counting::new(),
            stores: vec![
                Store::module(CARD),
                Store::nss(
                    SOFTOKEN,
                    Path::new("/casa/ada/.mozilla/firefox/ada.default"),
                ),
            ],
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
    assert_eq!(rows.len(), 2, "el de la tarjeta y el de Firefox: {rows:?}");
    assert_eq!(ready, Some(ReadyCard::Other));
}

#[test]
fn with_no_full_listing_yet_every_store_is_opened() {
    let arranged = Arranged::new();

    let (rows, _) = arranged.card_listing().relisted();

    assert_eq!(arranged.token.asked().len(), 2);
    assert_eq!(rows.len(), 2);
}
