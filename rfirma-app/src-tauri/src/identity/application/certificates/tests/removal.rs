use std::path::Path;

use super::super::{remember_the_certificate, remove_installed, InstallError, ListedCertificates};
use crate::identity::application::tests::a_certificate;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::CertificateMemory as _;
use crate::identity::ports::{Keyring, Token};
use crate::signing::application::tests::a_memory;

/// Un token cuyo `remove_certificate` siempre responde lo mismo; el resto de la firma no se usa aquí.
pub(super) struct RemovalOutcome(pub(super) Result<(), TokenError>);

impl Token for RemovalOutcome {
    fn list(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        unimplemented!("esta prueba no lista nada")
    }

    fn every_certificate(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        unimplemented!("esta prueba no lista nada")
    }

    fn list_authenticated(
        &self,
        _store: &Store,
        _pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        unimplemented!("esta prueba no lista nada")
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        unimplemented!("esta prueba no pide el secreto")
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        _algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        unimplemented!("esta prueba no comprueba el mecanismo")
    }

    fn accepts_the_secret(
        &self,
        _reference: &CertificateRef,
        _secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        unimplemented!("esta prueba no comprueba el secreto")
    }

    fn sign_with_secret(
        &self,
        _reference: &CertificateRef,
        _secret: &ProtectedSecret,
        _algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        unimplemented!("esta prueba no firma")
    }

    fn import_pkcs12(
        &self,
        _directory: &Path,
        _pkcs12: &[u8],
        _password: &str,
        _pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        unimplemented!("esta prueba no instala nada")
    }

    fn remove_certificate(
        &self,
        _directory: &Path,
        _reference: &CertificateRef,
        _pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        self.0.clone()
    }
}

/// Un llavero que siempre entrega el mismo PIN.
pub(super) struct FixedPinKeyring(pub(super) &'static str);

impl Keyring for FixedPinKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Ok(ProtectedSecret::from_str(self.0))
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

/// Un llavero que todavía no tiene el PIN del Almacén de rFirma.
struct NoPinYetKeyring;

impl Keyring for NoPinYetKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::PinMissing)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

/// Un certificado del Almacén de rFirma bajo `installed_dir`, con la base ya presente en disco.
fn an_installed_reference(installed_dir: &Path, cka_id: u8) -> CertificateRef {
    std::fs::create_dir_all(installed_dir).expect("deberia poder crearse el almacen");
    std::fs::write(installed_dir.join("cert9.db"), b"").expect("deberia poder crearse cert9.db");
    CertificateRef::new(
        Store::installed_nss("/usr/lib/libsoftokn3.so", installed_dir),
        "rfirma-test",
        "FIRMA",
        vec![cka_id],
    )
}

#[test]
fn removing_refuses_a_handle_that_is_not_from_the_last_listing() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let listed = ListedCertificates::new();

    let failure = remove_installed(
        &RemovalOutcome(Ok(())),
        &FixedPinKeyring("1234"),
        &a_memory(home.path()),
        &home.path().join("certificates"),
        "00000000000000000000000000000000",
        &listed,
        &ListedCertificates::new(),
    )
    .expect_err("no es de la ultima busqueda");

    assert!(matches!(
        failure,
        InstallError::Token(token) if token.situation() == Situation::CertificateNotFound
    ));
}

#[test]
fn removing_refuses_a_certificate_from_somewhere_else() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let reference = a_certificate("FIRMA", &[]).reference().clone();
    let listed = ListedCertificates::new();
    let handle = listed.mint(reference);

    let failure = remove_installed(
        &RemovalOutcome(Ok(())),
        &FixedPinKeyring("1234"),
        &a_memory(home.path()),
        &home.path().join("certificates"),
        &handle,
        &listed,
        &ListedCertificates::new(),
    )
    .expect_err("no viene de un .p12 instalado");

    assert!(matches!(
        failure,
        InstallError::Token(token) if token.situation() == Situation::CertificateNotFound
    ));
}

#[test]
fn removing_asks_the_keyring_for_the_pin_and_propagates_its_refusal() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let installed = home.path().join("certificates");
    let reference = an_installed_reference(&installed, 0x01);
    let listed = ListedCertificates::new();
    let handle = listed.mint(reference);

    let failure = remove_installed(
        &RemovalOutcome(Ok(())),
        &NoPinYetKeyring,
        &a_memory(home.path()),
        &installed,
        &handle,
        &listed,
        &ListedCertificates::new(),
    )
    .expect_err("sin PIN todavia no hay nada que quitar");

    assert!(matches!(
        failure,
        InstallError::Keyring(KeyringError::PinMissing)
    ));
}

#[test]
fn removing_propagates_the_token_refusal() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let installed = home.path().join("certificates");
    let reference = an_installed_reference(&installed, 0x01);
    let listed = ListedCertificates::new();
    let handle = listed.mint(reference);
    let refusal = TokenError::new(Situation::CertificateNotFound, "no esta en la base");

    let failure = remove_installed(
        &RemovalOutcome(Err(refusal)),
        &FixedPinKeyring("1234"),
        &a_memory(home.path()),
        &installed,
        &handle,
        &listed,
        &ListedCertificates::new(),
    )
    .expect_err("el token se ha negado");

    assert!(matches!(
        failure,
        InstallError::Token(token) if token.situation() == Situation::CertificateNotFound
    ));
}

#[test]
fn removing_the_remembered_certificate_forgets_it() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let installed = home.path().join("certificates");
    let reference = an_installed_reference(&installed, 0x01);
    let memory = a_memory(home.path());
    remember_the_certificate(&memory, &reference);
    let listed = ListedCertificates::new();
    let handle = listed.mint(reference);

    remove_installed(
        &RemovalOutcome(Ok(())),
        &FixedPinKeyring("1234"),
        &memory,
        &installed,
        &handle,
        &listed,
        &ListedCertificates::new(),
    )
    .expect("deberia quitarse");

    assert_eq!(
        memory.remembered_certificate(),
        None,
        "ya no deberia estar recordado"
    );
}

#[test]
fn removing_a_different_certificate_leaves_the_remembered_one_alone() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let installed = home.path().join("certificates");
    let remembered = an_installed_reference(&installed, 0x01);
    let removed = an_installed_reference(&installed, 0x02);
    let memory = a_memory(home.path());
    remember_the_certificate(&memory, &remembered);
    let listed = ListedCertificates::new();
    let handle = listed.mint(removed);

    remove_installed(
        &RemovalOutcome(Ok(())),
        &FixedPinKeyring("1234"),
        &memory,
        &installed,
        &handle,
        &listed,
        &ListedCertificates::new(),
    )
    .expect("deberia quitarse");

    assert_eq!(memory.remembered_certificate(), Some(remembered));
}
