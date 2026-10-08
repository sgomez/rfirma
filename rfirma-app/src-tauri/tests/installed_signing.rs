//! Firma con un certificado del Almacén de rFirma: el PIN sale del llavero, sin pedir nada (ADR-0034).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::application::certificates::ListedCertificates;
use rfirma_lib::identity::domain::algorithm::SignatureAlgorithm;
use rfirma_lib::identity::domain::certificate::CertificateRef;
use rfirma_lib::identity::domain::keyring::KeyringError;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::secret::StoreSecret;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::ports::{
    CertificateMemory, Keyring, KeyringFactory, SecretPromptError, SecretPromptRequest,
    SecretPrompter,
};
use rfirma_lib::identity::IdentityRoot;
use rfirma_lib::memory_error::MemoryError;
use rfirma_lib::signing::ports::Signer;

const KIT_PASSWORD: &str = "1234";
const KEYRING_PIN: &str = "pin-de-pruebas-del-almacen-de-rfirma";

const CARD_MODULE: &str = "/usr/lib/softhsm/libsofthsm2.so";
const CARD_ACTIVE: &str = "FNMT-ACTIVO-99999999R";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("la raiz del repositorio")
        .to_path_buf()
}

fn kit_p12() -> PathBuf {
    repository_root().join("testdata/fnmt/active-rsa.p12")
}

fn elliptic_curve_kit_p12() -> PathBuf {
    repository_root().join("testdata/fnmt/active-ecc.p12")
}

/// El doble en memoria del llavero del escritorio: siempre entrega el mismo PIN.
struct FixedPinKeyring;

impl Keyring for FixedPinKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Ok(ProtectedSecret::from_str(KEYRING_PIN))
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

/// El llavero que ninguna firma con tarjeta debería alcanzar; cuenta si algo lo intenta.
#[derive(Clone, Default)]
struct UnreachableKeyring {
    calls: Arc<AtomicUsize>,
}

impl Keyring for UnreachableKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(KeyringError::NoKeyring)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

/// Una memoria que no recuerda ningún certificado ni escribe en ningún sitio.
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

/// Un diálogo que no debería abrirse nunca en estas pruebas.
struct UnreachablePrompter;

impl SecretPrompter for UnreachablePrompter {
    fn prompt_secret(
        &self,
        _request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        panic!("esta prueba no deberia necesitar el dialogo del secreto")
    }
}

fn an_empty_installation() -> tempfile::TempDir {
    tempfile::tempdir().expect("deberia poder crearse un directorio temporal")
}

fn install(installed: &Path, p12: &Path, password: &str) {
    let bytes = std::fs::read(p12).expect("el .p12 de pruebas deberia leerse");
    certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &FixedPinKeyring,
        installed,
        &bytes,
        password,
    )
    .expect("el .p12 del kit deberia instalarse");
}

fn identity_root_over(installed: &Path, keyring: KeyringFactory) -> IdentityRoot {
    IdentityRoot {
        token: Box::new(pkcs11::RealToken),
        stores: Vec::new(),
        installed_certificates: installed.to_path_buf(),
        listed: ListedCertificates::new(),
        installed_copies: ListedCertificates::new(),
        last_listing: Default::default(),
        reader_now: Default::default(),
        memory: Arc::new(NoMemory),
        folder: Arc::new(RealInstalledFolder),
        prompter: Arc::new(UnreachablePrompter),
        keyring,
    }
}

fn installed_certificate(installed: &Path) -> CertificateRef {
    let softoken = pkcs11::stores::softoken().expect(
        "falta libsoftokn3.so. Las pruebas de grada B del .p12 instalado lo necesitan:\n  \
         sudo apt install -y libnss3",
    );
    let stores = pkcs11::stores::installed_stores(&softoken, installed);
    pkcs11::list_certificates_across(&stores)
        .expect("el almacen del .p12 deberia listarse")
        .into_iter()
        .next()
        .expect("el .p12 instalado deberia traer un certificado")
        .reference()
        .clone()
}

fn card_certificate() -> CertificateRef {
    let module = PathBuf::from(CARD_MODULE);
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Las pruebas de grada B necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just certs install",
        module.display()
    );
    pkcs11::list_certificates(Store::module(module))
        .expect("no se ha podido listar el token")
        .into_iter()
        .find(|certificate| certificate.reference().label() == CARD_ACTIVE)
        .unwrap_or_else(|| {
            panic!("falta {CARD_ACTIVE} en el token de pruebas. Montalo con:\n  just certs install")
        })
        .reference()
        .clone()
}

fn fixed_pin_keyring_factory() -> KeyringFactory {
    Arc::new(|| Ok(Box::new(FixedPinKeyring) as Box<dyn Keyring + Send + Sync>))
}

fn unreachable_keyring_factory() -> (KeyringFactory, Arc<AtomicUsize>) {
    let keyring = UnreachableKeyring::default();
    let calls = keyring.calls.clone();
    let factory: KeyringFactory =
        Arc::new(move || Ok(Box::new(keyring.clone()) as Box<dyn Keyring + Send + Sync>));
    (factory, calls)
}

#[test]
fn an_rsa_certificate_installed_signs_with_the_keyring_pin_and_without_asking_anything() {
    let installation = an_empty_installation();
    install(installation.path(), &kit_p12(), KIT_PASSWORD);
    let identity = identity_root_over(installation.path(), fixed_pin_keyring_factory());
    let reference = installed_certificate(installation.path());
    let signer = identity.signer();

    assert_eq!(
        signer
            .secret_of(&reference)
            .expect("deberia decir como pide el secreto"),
        StoreSecret::NotNeeded
    );

    let signature = signer
        .sign_with_secret(
            &reference,
            &ProtectedSecret::from_str(""),
            SignatureAlgorithm::Sha256Rsa,
            b"datos de prueba",
        )
        .expect("deberia firmar con el pin que entrega el llavero");
    assert!(!signature.is_empty());
}

#[test]
fn an_elliptic_curve_certificate_installed_also_signs_with_the_keyring_pin() {
    let installation = an_empty_installation();
    install(installation.path(), &elliptic_curve_kit_p12(), KIT_PASSWORD);
    let identity = identity_root_over(installation.path(), fixed_pin_keyring_factory());
    let reference = installed_certificate(installation.path());
    let signer = identity.signer();

    let signature = signer
        .sign_with_secret(
            &reference,
            &ProtectedSecret::from_str(""),
            SignatureAlgorithm::Sha256Ecdsa,
            b"datos de prueba",
        )
        .expect("deberia firmar con el pin que entrega el llavero");
    assert!(!signature.is_empty());
}

#[test]
fn signing_with_a_card_never_reaches_the_keyring() {
    let installation = an_empty_installation();
    let (keyring, calls) = unreachable_keyring_factory();
    let identity = identity_root_over(installation.path(), keyring);
    let reference = card_certificate();
    let signer = identity.signer();

    assert_eq!(
        signer
            .secret_of(&reference)
            .expect("deberia decir como pide el secreto"),
        StoreSecret::TypedOnScreen
    );

    let signature = signer
        .sign_with_secret(
            &reference,
            &ProtectedSecret::from_str("1234"),
            SignatureAlgorithm::Sha256Rsa,
            b"datos de prueba",
        )
        .expect("deberia firmar con el pin tecleado");
    assert!(!signature.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
