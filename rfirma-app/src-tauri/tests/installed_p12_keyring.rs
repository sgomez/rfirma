//! El PIN del Almacén de rFirma en el llavero del escritorio, al instalar (ADR-0034).

use std::path::{Path, PathBuf};

use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::keyring::KeyringError;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::ports::{Keyring, Token};

/// Contraseña de los `.p12` del kit de pruebas (`active-rsa.p12`, `active-ecc.p12`).
const KIT_PASSWORD: &str = "1234";

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

/// El `.p12` de curva elíptica de pruebas de la FNMT.
fn elliptic_curve_kit_p12() -> PathBuf {
    repository_root().join("testdata/fnmt/active-ecc.p12")
}

fn an_empty_installation() -> tempfile::TempDir {
    tempfile::tempdir().expect("deberia poder crearse un directorio temporal")
}

/// El PIN que entrega [`FixedPinKeyring`], con el que queda cifrado el Almacén de rFirma instalado.
const KEYRING_PIN: &str = "pin-de-pruebas-del-almacen-de-rfirma";

/// El doble en memoria del llavero del escritorio (TD-112): siempre entrega el mismo PIN.
struct FixedPinKeyring;

impl Keyring for FixedPinKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Ok(ProtectedSecret::from_str(KEYRING_PIN))
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

/// El doble en memoria del llavero (TD-112): el PIN del almacén ya existente se perdió.
struct LostPinKeyring;

impl Keyring for LostPinKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::PinMissing)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Ok(ProtectedSecret::from_str(KEYRING_PIN))
    }
}

/// El doble en memoria del llavero (TD-112): entrega un PIN, pero no es el que cifra el almacén ya existente.
struct WrongPinKeyring;

impl Keyring for WrongPinKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Ok(ProtectedSecret::from_str(
            "no es el pin de la base ya existente",
        ))
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        self.pin()
    }
}

fn install(
    installed: &Path,
    p12: &Path,
    password: &str,
) -> Result<(), rfirma_lib::crossing::Failure> {
    let bytes = std::fs::read(p12).expect("el .p12 de pruebas deberia leerse");
    Ok(certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &FixedPinKeyring,
        installed,
        &bytes,
        password,
    )?)
}

/// Almacenes instalados actualmente bajo `installed`.
fn installed_stores(installed: &Path) -> Vec<Store> {
    let softoken = pkcs11::stores::softoken().expect(
        "falta libsoftokn3.so. Las pruebas de grada B del .p12 instalado lo necesitan:\n  \
         sudo apt install -y libnss3",
    );
    pkcs11::stores::installed_stores(&softoken, installed)
}

fn certificates(installed: &Path) -> Vec<TokenCertificate> {
    pkcs11::list_certificates_across(&installed_stores(installed))
        .expect("el almacen del .p12 deberia listarse")
}

#[test]
fn without_a_desktop_keyring_nothing_installs() {
    struct NoKeyringAtAll;
    impl Keyring for NoKeyringAtAll {
        fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
            Err(KeyringError::NoKeyring)
        }
        fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
            Err(KeyringError::NoKeyring)
        }
    }

    let installed = an_empty_installation();
    let bytes = std::fs::read(kit_p12()).expect("el .p12 del kit deberia leerse");

    let failure = certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &NoKeyringAtAll,
        installed.path(),
        &bytes,
        KIT_PASSWORD,
    )
    .expect_err("sin llavero del escritorio no hay instalacion (ADR-0034)");

    assert_eq!(
        rfirma_lib::crossing::Failure::from(failure).situation,
        "noKeyring"
    );
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn a_pin_that_is_not_utf8_refuses_instead_of_installing_unencrypted() {
    struct NonUtf8PinKeyring;
    impl Keyring for NonUtf8PinKeyring {
        fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
            Ok(ProtectedSecret::new([0xff, 0xfe, 0xfd]))
        }
        fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
            self.pin()
        }
    }

    let installed = an_empty_installation();
    let bytes = std::fs::read(kit_p12()).expect("el .p12 del kit deberia leerse");

    let failure = certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &NonUtf8PinKeyring,
        installed.path(),
        &bytes,
        KIT_PASSWORD,
    )
    .expect_err("un pin que no es UTF-8 no puede inicializar la base sin cifrar (ADR-0034)");

    assert_eq!(
        rfirma_lib::crossing::Failure::from(failure).situation,
        "incorrectPin"
    );
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn the_installed_store_only_opens_with_the_keyring_pin() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");
    let store = installed_stores(installed.path())
        .into_iter()
        .next()
        .expect("el almacen deberia existir");

    pkcs11::RealToken
        .list_authenticated(&store, &ProtectedSecret::from_str(""))
        .expect_err("un pin vacio no deberia abrir una base cifrada");
    pkcs11::RealToken
        .list_authenticated(&store, &ProtectedSecret::from_str("no es el pin correcto"))
        .expect_err("un pin equivocado no deberia abrir una base cifrada");

    let found = pkcs11::RealToken
        .list_authenticated(&store, &ProtectedSecret::from_str(KEYRING_PIN))
        .expect("el pin del llavero deberia abrir la base cifrada");
    assert_eq!(found.len(), 1);
}

#[test]
fn losing_the_pin_over_an_existing_store_is_told_apart_from_having_no_keyring_at_all() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");

    let bytes = std::fs::read(elliptic_curve_kit_p12()).expect("el .p12 del kit deberia leerse");
    let failure = certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &LostPinKeyring,
        installed.path(),
        &bytes,
        KIT_PASSWORD,
    )
    .expect_err("el llavero perdio el pin: no hay con que abrir el almacen ya existente");

    assert_eq!(
        rfirma_lib::crossing::Failure::from(failure).situation,
        "keyringPinMissing"
    );
    assert_eq!(
        certificates(installed.path()).len(),
        1,
        "un pin perdido no puede escribir una base nueva encima de la que ya habia"
    );
}

#[test]
fn a_keyring_pin_that_does_not_open_the_existing_store_offers_to_empty_it() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");

    let bytes = std::fs::read(elliptic_curve_kit_p12()).expect("el .p12 del kit deberia leerse");
    let failure = certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        &WrongPinKeyring,
        installed.path(),
        &bytes,
        KIT_PASSWORD,
    )
    .expect_err("el llavero entrega un pin, pero no es el que abre el almacen ya existente");

    assert_eq!(
        rfirma_lib::crossing::Failure::from(failure).situation,
        "keyringPinMissing"
    );
    assert_eq!(
        certificates(installed.path()).len(),
        1,
        "un pin equivocado no puede escribir una base nueva encima de la que ya habia"
    );
}

#[test]
fn emptying_the_store_after_losing_the_pin_allows_installing_again() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");

    certificates::empty_the_store(&RealInstalledFolder, installed.path())
        .expect("vaciar el almacen deberia funcionar tras confirmarlo la persona");
    assert!(installed_stores(installed.path()).is_empty());

    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("tras vaciarlo, el almacen vuelve a aceptar una instalacion desde cero");
    assert_eq!(certificates(installed.path()).len(), 1);
}
