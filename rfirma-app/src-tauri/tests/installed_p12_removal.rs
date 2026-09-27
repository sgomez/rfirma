//! Borrado de un certificado del Almacén de rFirma: certificado, clave y solo ese (ADR-0034).

use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::application::certificates::ListedCertificates;
use rfirma_lib::identity::domain::certificate::CertificateRef;
use rfirma_lib::identity::domain::keyring::KeyringError;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::ports::{CertificateMemory, Keyring};
use rfirma_lib::memory_error::MemoryError;

/// Contraseña de los `.p12` del kit de pruebas (`active-rsa.p12`).
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

/// Genera un `.p12` de clave RSA sin `friendlyName` en `directory`.
fn a_p12_without_a_friendly_name(directory: &Path, subject: &str, password: &str) -> PathBuf {
    let key = directory.join("plain.pem");
    let certificate = directory.join("plain-cert.pem");
    let bundle = directory.join("plain.p12");

    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "30",
        "-subj",
        subject,
        "-addext",
        "basicConstraints=critical,CA:FALSE",
        "-addext",
        "keyUsage=critical,digitalSignature",
        "-keyout",
        key.to_str().expect("ruta valida"),
        "-out",
        certificate.to_str().expect("ruta valida"),
    ]);
    run_openssl(&[
        "pkcs12",
        "-export",
        "-inkey",
        key.to_str().expect("ruta valida"),
        "-in",
        certificate.to_str().expect("ruta valida"),
        "-passout",
        &format!("pass:{password}"),
        "-out",
        bundle.to_str().expect("ruta valida"),
    ]);

    bundle
}

fn run_openssl(arguments: &[&str]) {
    let output = Command::new("openssl").args(arguments).output().expect(
        "falta openssl. Las pruebas del .p12 instalado lo necesitan: sudo apt install -y openssl",
    );
    assert!(
        output.status.success(),
        "openssl {arguments:?} ha fallado:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Directorio temporal desechable para almacenes instalados.
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

fn remove(
    installed: &Path,
    listed: &ListedCertificates,
    handle: &str,
) -> Result<(), rfirma_lib::crossing::Failure> {
    Ok(certificates::remove_installed(
        &pkcs11::RealToken,
        &FixedPinKeyring,
        &NoMemory,
        installed,
        handle,
        listed,
        &ListedCertificates::new(),
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

fn certificates(
    installed: &Path,
) -> Vec<rfirma_lib::identity::domain::certificate::TokenCertificate> {
    pkcs11::list_certificates_across(&installed_stores(installed))
        .expect("el almacen del .p12 deberia listarse")
}

#[test]
fn removing_an_installed_certificate_deletes_it_from_the_store() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");
    let listed = ListedCertificates::new();
    let found = certificates(installed.path());
    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );

    remove(installed.path(), &listed, &handles[0]).expect("deberia quitarse");

    assert!(
        certificates(installed.path()).is_empty(),
        "el certificado quitado ya no deberia listarse"
    );
}

#[test]
fn removing_one_of_two_certificates_with_the_same_common_name_keeps_the_other() {
    let installed = an_empty_installation();
    let first_workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let second_workshop =
        tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let first =
        a_p12_without_a_friendly_name(first_workshop.path(), "/CN=MISMO NOMBRE", KIT_PASSWORD);
    let second =
        a_p12_without_a_friendly_name(second_workshop.path(), "/CN=MISMO NOMBRE", KIT_PASSWORD);
    install(installed.path(), &first, KIT_PASSWORD).expect("el primero deberia instalarse");
    install(installed.path(), &second, KIT_PASSWORD).expect("el segundo deberia instalarse");
    let listed = ListedCertificates::new();
    let found = certificates(installed.path());
    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );

    let survivor = found[1].reference().clone();

    remove(installed.path(), &listed, &handles[0]).expect("deberia quitarse el primero");

    let left = certificates(installed.path());
    assert_eq!(
        left.len(),
        1,
        "el otro certificado con el mismo nombre comun deberia seguir"
    );
    assert_eq!(
        left[0].reference().cka_id(),
        survivor.cka_id(),
        "deberia seguir el otro, no el quitado"
    );
}

#[test]
fn a_certificate_from_somewhere_else_is_not_removed() {
    let installed = an_empty_installation();
    let elsewhere = an_empty_installation();
    install(elsewhere.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");
    let listed = ListedCertificates::new();
    let found = certificates(elsewhere.path());
    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );

    let failure =
        remove(installed.path(), &listed, &handles[0]).expect_err("no viene de este directorio");

    assert_eq!(failure.situation, "certificateNotFound");
    assert_eq!(
        installed_stores(elsewhere.path()).len(),
        1,
        "el almacen de al lado sigue donde estaba"
    );
}
