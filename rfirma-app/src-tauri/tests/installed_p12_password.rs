//! Petición de la contraseña al instalar un PKCS#12: reintentos y cancelación.

use std::path::{Path, PathBuf};

use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::ports::{PromptedError, SecretPromptError};
use rfirma_lib::signing::adapters::gtk_prompter::{
    MockSecretPrompter, PreconfiguredSecretPrompter,
};
use rfirma_lib::signing::domain::Language;

const KIT_PASSWORD: &str = "1234";

fn kit_p12() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("la raiz del repositorio")
        .join("testdata/fnmt/active-rsa.p12")
}

fn an_empty_installation() -> tempfile::TempDir {
    tempfile::tempdir().expect("deberia poder crearse un directorio temporal")
}

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
fn a_wrong_password_is_retried_until_it_installs() {
    let installed = an_empty_installation();
    let bytes = std::fs::read(kit_p12()).expect("el .p12 del kit deberia leerse");
    let prompter = MockSecretPrompter::with_secrets(&["no es la suya", KIT_PASSWORD]);

    certificates::install_pkcs12_asking_its_password(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        installed.path(),
        &bytes,
        "active-rsa.p12",
        &prompter,
        Language::Spanish,
    )
    .expect("la segunda contrasena es la correcta");

    let recorded = prompter.recorded_requests();
    assert_eq!(
        recorded.len(),
        2,
        "la primera contrasena no valia, hace falta un segundo intento"
    );
    assert!(!recorded[0].incorrect_secret);
    assert!(recorded[1].incorrect_secret);
    assert_eq!(certificates(installed.path()).len(), 1);
}

#[test]
fn cancelling_the_password_prompt_installs_nothing_and_fails_nothing() {
    let installed = an_empty_installation();
    let bytes = std::fs::read(kit_p12()).expect("el .p12 del kit deberia leerse");
    let prompter = PreconfiguredSecretPrompter::cancelling();

    let error = certificates::install_pkcs12_asking_its_password(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        installed.path(),
        &bytes,
        "active-rsa.p12",
        &prompter,
        Language::Spanish,
    )
    .expect_err("cancelar no instala");

    assert!(matches!(
        error,
        PromptedError::Prompt(SecretPromptError::Cancelled)
    ));
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn an_unreadable_file_does_not_retry_the_password() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let not_a_p12 = workshop.path().join("not-a.p12");
    std::fs::write(&not_a_p12, b"esto no es un pkcs12").expect("deberia poder escribirse");
    let bytes = std::fs::read(&not_a_p12).expect("deberia leerse");
    let prompter = MockSecretPrompter::with_secrets(&[KIT_PASSWORD]);

    let error = certificates::install_pkcs12_asking_its_password(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        installed.path(),
        &bytes,
        "not-a.p12",
        &prompter,
        Language::Spanish,
    )
    .expect_err("un fichero ilegible no se instala");

    assert!(matches!(error, PromptedError::Attempt(_)));
    assert_eq!(
        prompter.recorded_requests().len(),
        1,
        "un fichero ilegible no vuelve a pedir contrasena"
    );
}
