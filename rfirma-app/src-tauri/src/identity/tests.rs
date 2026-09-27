use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::identity::domain::error::Situation;
use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::ports::Signer;

/// Un token que solo recuerda con qué secreto se le pidió firmar o aceptar, y responde `reply` a `secret_of`.
struct RecordingToken {
    reply: StoreSecret,
    received_secret: Mutex<Option<Vec<u8>>>,
}

impl RecordingToken {
    fn replying(reply: StoreSecret) -> Self {
        Self {
            reply,
            received_secret: Mutex::new(None),
        }
    }

    fn received_secret(&self) -> Option<Vec<u8>> {
        self.received_secret.lock().unwrap().clone()
    }
}

impl Token for RecordingToken {
    fn list(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn every_certificate(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn list_authenticated(
        &self,
        _store: &Store,
        _pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(self.reply)
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        _algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        Ok(())
    }

    fn accepts_the_secret(
        &self,
        _reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        *self.received_secret.lock().unwrap() = Some(secret.as_bytes().to_vec());
        Ok(())
    }

    fn sign_with_secret(
        &self,
        _reference: &CertificateRef,
        secret: &ProtectedSecret,
        _algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        *self.received_secret.lock().unwrap() = Some(secret.as_bytes().to_vec());
        Ok(b"firmado".to_vec())
    }

    fn import_pkcs12(
        &self,
        _directory: &Path,
        _pkcs12: &[u8],
        _password: &str,
        _pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        Err(TokenError::new(
            Situation::Pkcs12Unreadable,
            "no importa nada",
        ))
    }

    fn remove_certificate(
        &self,
        _directory: &Path,
        _reference: &CertificateRef,
        _pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        Err(TokenError::new(
            Situation::CertificateNotFound,
            "no quita nada",
        ))
    }
}

/// Un llavero que cuenta cuántas veces se le pidió el PIN y siempre entrega el mismo.
#[derive(Clone)]
struct CountingKeyring {
    calls: Arc<AtomicUsize>,
    pin: &'static str,
}

impl CountingKeyring {
    fn with_pin(pin: &'static str) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                calls: calls.clone(),
                pin,
            },
            calls,
        )
    }
}

impl ports::Keyring for CountingKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ProtectedSecret::from_str(self.pin))
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

fn factory_of(keyring: CountingKeyring) -> KeyringFactory {
    Arc::new(move || Ok(Box::new(keyring.clone()) as Box<dyn ports::Keyring + Send + Sync>))
}

/// Un llavero sin PIN que cuenta si se le pidió crear uno.
#[derive(Clone)]
struct PinMissingKeyring {
    create_pin_calls: Arc<AtomicUsize>,
}

impl PinMissingKeyring {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let create_pin_calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                create_pin_calls: create_pin_calls.clone(),
            },
            create_pin_calls,
        )
    }
}

impl ports::Keyring for PinMissingKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::PinMissing)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.create_pin_calls.fetch_add(1, Ordering::SeqCst);
        Ok(ProtectedSecret::from_str("pin-nuevo"))
    }
}

/// Una referencia bajo el Almacén de rFirma: hace falta un `cert9.db` de verdad en `installed`.
fn an_installed_reference(installed: &Path) -> CertificateRef {
    std::fs::write(installed.join("cert9.db"), b"").expect("deberia poder escribirse cert9.db");
    CertificateRef::new(
        Store::nss("/usr/lib/libsoftokn3.so", installed),
        "rFirma",
        "un-certificado",
        None,
    )
}

/// Una referencia de una tarjeta cualquiera, fuera del Almacén de rFirma.
fn a_card_reference() -> CertificateRef {
    CertificateRef::new(
        Store::module("/usr/lib/softhsm/libsofthsm2.so"),
        "tarjeta",
        "un-certificado",
        None,
    )
}

#[test]
fn secret_of_needs_nothing_for_the_installed_store_even_if_the_token_would_ask() {
    let installed = tempfile::tempdir().expect("directorio temporal");
    let reference = an_installed_reference(installed.path());
    let token = RecordingToken::replying(StoreSecret::TypedOnScreen);
    let (keyring, calls) = CountingKeyring::with_pin("pin");
    let signer = TokenSigner {
        token: &token,
        installed_certificates: installed.path(),
        keyring: &factory_of(keyring),
    };

    assert_eq!(
        signer.secret_of(&reference).unwrap(),
        StoreSecret::NotNeeded
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn secret_of_asks_the_token_for_a_card() {
    let installed = tempfile::tempdir().expect("directorio temporal");
    let reference = a_card_reference();
    let token = RecordingToken::replying(StoreSecret::TypedOnScreen);
    let (keyring, _) = CountingKeyring::with_pin("pin");
    let signer = TokenSigner {
        token: &token,
        installed_certificates: installed.path(),
        keyring: &factory_of(keyring),
    };

    assert_eq!(
        signer.secret_of(&reference).unwrap(),
        StoreSecret::TypedOnScreen
    );
}

#[test]
fn signing_the_installed_certificate_uses_the_keyring_pin_instead_of_the_empty_secret() {
    let installed = tempfile::tempdir().expect("directorio temporal");
    let reference = an_installed_reference(installed.path());
    let token = RecordingToken::replying(StoreSecret::NotNeeded);
    let (keyring, calls) = CountingKeyring::with_pin("pin-del-llavero");
    let signer = TokenSigner {
        token: &token,
        installed_certificates: installed.path(),
        keyring: &factory_of(keyring),
    };

    signer
        .sign_with_secret(
            &reference,
            &ProtectedSecret::from_str(""),
            SignatureAlgorithm::Sha256Rsa,
            b"datos",
        )
        .expect("deberia firmar");

    assert_eq!(token.received_secret(), Some(b"pin-del-llavero".to_vec()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn signing_a_card_certificate_never_touches_the_keyring() {
    let installed = tempfile::tempdir().expect("directorio temporal");
    let reference = a_card_reference();
    let token = RecordingToken::replying(StoreSecret::TypedOnScreen);
    let (keyring, calls) = CountingKeyring::with_pin("pin-del-llavero");
    let signer = TokenSigner {
        token: &token,
        installed_certificates: installed.path(),
        keyring: &factory_of(keyring),
    };
    let typed = ProtectedSecret::from_str("1234");

    signer
        .sign_with_secret(&reference, &typed, SignatureAlgorithm::Sha256Rsa, b"datos")
        .expect("deberia firmar");

    assert_eq!(token.received_secret(), Some(b"1234".to_vec()));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn signing_with_a_keyring_missing_the_pin_fails_as_keyring_pin_missing_without_creating_one() {
    let installed = tempfile::tempdir().expect("directorio temporal");
    let reference = an_installed_reference(installed.path());
    let token = RecordingToken::replying(StoreSecret::NotNeeded);
    let (keyring, create_pin_calls) = PinMissingKeyring::new();
    let factory: KeyringFactory =
        Arc::new(move || Ok(Box::new(keyring.clone()) as Box<dyn ports::Keyring + Send + Sync>));
    let signer = TokenSigner {
        token: &token,
        installed_certificates: installed.path(),
        keyring: &factory,
    };

    let error = signer
        .sign_with_secret(
            &reference,
            &ProtectedSecret::from_str(""),
            SignatureAlgorithm::Sha256Rsa,
            b"datos",
        )
        .expect_err("el llavero no tiene el PIN todavia");

    assert_eq!(error.situation(), Situation::KeyringPinMissing);
    assert_eq!(create_pin_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn accepting_the_secret_of_an_installed_certificate_also_uses_the_keyring_pin() {
    let installed = tempfile::tempdir().expect("directorio temporal");
    let reference = an_installed_reference(installed.path());
    let token = RecordingToken::replying(StoreSecret::NotNeeded);
    let (keyring, calls) = CountingKeyring::with_pin("pin-del-llavero");
    let signer = TokenSigner {
        token: &token,
        installed_certificates: installed.path(),
        keyring: &factory_of(keyring),
    };

    signer
        .accepts_the_secret(&reference, &ProtectedSecret::from_str(""))
        .expect("deberia aceptar el pin del llavero");

    assert_eq!(token.received_secret(), Some(b"pin-del-llavero".to_vec()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
