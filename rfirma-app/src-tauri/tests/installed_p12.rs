//! Ciclo de vida de un archivo PKCS#12 instalado en almacén NSS (ADR-0014).

use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::identity::adapters::pkcs11::{self, Store, TokenCertificate};
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::application::listed::ListedCertificates;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::Sha256;
use tauri_plugin_dialog::FilePath;
use x509_cert::der::{Decode, Encode};

/// Contraseña de `active-rsa.p12` del kit de pruebas.
const KIT_PASSWORD: &str = "1234";
/// Contraseña del `.p12` de clave elíptica que fabrica esta prueba.
const EC_PASSWORD: &str = "1234";
/// Bloque DER de prueba de `SignedAttributes` sin hashear.
const PRESIGN: &[u8] = b"31 5f 30 18 06 09 2a 86 SignedAttributes de mentira, sin hashear";

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

/// Genera un `.p12` con clave elíptica en `directory`.
fn an_elliptic_curve_p12(directory: &Path) -> PathBuf {
    let key = directory.join("ec.pem");
    let certificate = directory.join("ec-cert.pem");
    let bundle = directory.join("ec.p12");

    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:prime256v1",
        "-nodes",
        "-days",
        "30",
        "-subj",
        "/CN=CLAVE ELIPTICA DE PRUEBAS",
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
        "-name",
        "CLAVE ELIPTICA DE PRUEBAS",
        "-passout",
        &format!("pass:{EC_PASSWORD}"),
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

fn install(
    installed: &Path,
    p12: &Path,
    password: &str,
) -> Result<(), rfirma_lib::commands::Failure> {
    Ok(certificates::install_pkcs12(
        &pkcs11::RealToken,
        installed,
        FilePath::from(p12),
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

fn verifying_key(certificate: &TokenCertificate) -> VerifyingKey<Sha256> {
    let parsed =
        x509_cert::Certificate::from_der(certificate.der()).expect("el DER deberia parsearse");
    let spki = parsed
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .expect("el SPKI deberia serializarse");
    let public_key = RsaPublicKey::from_public_key_der(&spki).expect("clave publica RSA");
    VerifyingKey::<Sha256>::new(public_key)
}

#[test]
fn an_rsa_p12_installs_and_its_certificates_list_without_the_password() {
    let installed = an_empty_installation();

    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");

    let found = certificates(installed.path());
    assert_eq!(found.len(), 1, "el .p12 trae un certificado de persona");
    assert!(found[0]
        .subject()
        .is_some_and(|subject| subject.contains("EIDAS")));
}

#[test]
fn a_p12_with_an_elliptic_curve_key_is_refused_at_install() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let elliptic = an_elliptic_curve_p12(workshop.path());

    let failure = install(installed.path(), &elliptic, EC_PASSWORD)
        .expect_err("una clave eliptica no se puede instalar");

    assert_eq!(failure.situation, "keyNotRsa");
}

#[test]
fn a_refused_p12_leaves_no_store_behind() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let elliptic = an_elliptic_curve_p12(workshop.path());

    let _ = install(installed.path(), &elliptic, EC_PASSWORD);

    assert!(
        installed_stores(installed.path()).is_empty(),
        "el rechazo tenia que borrar el almacen a medio escribir"
    );
}

#[test]
fn a_wrong_password_is_told_apart_from_a_key_that_does_not_serve() {
    let installed = an_empty_installation();

    let failure = install(installed.path(), &kit_p12(), "no es la suya")
        .expect_err("con otra contrasena no se puede abrir el fichero");

    assert_eq!(failure.situation, "pkcs12Unreadable");
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn nothing_of_the_file_is_kept_beyond_the_two_databases() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");

    let store_directory = std::fs::read_dir(installed.path())
        .expect("deberia leerse")
        .flatten()
        .map(|entry| entry.path())
        .next()
        .expect("tenia que quedar un almacen");
    let mut inside: Vec<String> = std::fs::read_dir(&store_directory)
        .expect("deberia leerse")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    inside.sort();

    assert_eq!(inside, vec!["cert9.db".to_owned(), "key4.db".to_owned()]);
    assert!(
        !store_directory
            .file_name()
            .expect("el almacen tiene nombre")
            .to_string_lossy()
            .contains("active-rsa"),
        "el nombre del almacen no puede llevar el del fichero dentro"
    );
}

#[test]
fn two_installed_files_are_two_stores() {
    let installed = an_empty_installation();

    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");
    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el segundo deberia instalarse");

    assert_eq!(installed_stores(installed.path()).len(), 2);
    assert_eq!(certificates(installed.path()).len(), 2);
}

#[test]
fn a_certificate_that_came_from_a_p12_signs() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");
    let certificate = certificates(installed.path())
        .into_iter()
        .next()
        .expect("tenia que haber un certificado");

    let raw = pkcs11::sign(certificate.reference(), "", PRESIGN)
        .expect("un .p12 instalado tiene que poder firmar sin secreto que teclear");

    assert_eq!(raw.len(), 256, "RSA 2048: la firma cruda mide el modulo");
    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

#[test]
fn an_installed_p12_asks_for_no_secret() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");
    let certificate = certificates(installed.path())
        .into_iter()
        .next()
        .expect("tenia que haber un certificado");

    let secret = pkcs11::store_secret(certificate.reference()).expect("deberia poder preguntarse");

    assert_eq!(secret, pkcs11::StoreSecret::NotNeeded);
}

#[test]
fn removing_an_installed_certificate_deletes_its_store() {
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

    certificates::remove_installed(installed.path(), &handles[0], &listed)
        .expect("deberia poder quitarse");

    assert!(installed_stores(installed.path()).is_empty());
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

    let failure = certificates::remove_installed(installed.path(), &handles[0], &listed)
        .expect_err("no viene de este directorio");

    assert_eq!(
        rfirma_lib::commands::Failure::from(failure).situation,
        "certificateNotFound"
    );
    assert_eq!(
        installed_stores(elsewhere.path()).len(),
        1,
        "el almacen de al lado sigue donde estaba"
    );
}
