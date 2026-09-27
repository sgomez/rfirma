//! Ciclo de vida de un archivo PKCS#12 instalado en almacén NSS (ADR-0014).

use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::application::certificates::ListedCertificates;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::keyring::KeyringError;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::identity::ports::Keyring;
use x509_cert::der::Decode;

/// Contraseña de los `.p12` del kit de pruebas (`active-rsa.p12`, `active-ecc.p12`).
const KIT_PASSWORD: &str = "1234";
/// Contraseña de los `.p12` que esta prueba fabrica al vuelo con openssl.
const GENERATED_PASSWORD: &str = "1234";

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

/// Genera un `.p12` con una clave DSA (que NSS sí importa, pero que no es RSA ni de curva elíptica) en `directory`.
fn a_p12_of_an_unsupported_key_kind(directory: &Path) -> PathBuf {
    let parameters = directory.join("dsaparam.pem");
    let key = directory.join("dsa.pem");
    let certificate = directory.join("dsa-cert.pem");
    let bundle = directory.join("dsa.p12");

    run_openssl(&[
        "dsaparam",
        "-out",
        parameters.to_str().expect("ruta valida"),
        "2048",
    ]);
    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        &format!("dsa:{}", parameters.to_str().expect("ruta valida")),
        "-nodes",
        "-days",
        "30",
        "-subj",
        "/CN=CLAVE NO SOPORTADA DE PRUEBAS",
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
        "CLAVE NO SOPORTADA DE PRUEBAS",
        "-passout",
        &format!("pass:{GENERATED_PASSWORD}"),
        "-out",
        bundle.to_str().expect("ruta valida"),
    ]);

    bundle
}

/// Genera un `.p12` con un certificado sin clave privada emparejada en `directory`.
fn a_p12_without_a_private_key(directory: &Path) -> PathBuf {
    let key = directory.join("nokey.pem");
    let certificate = directory.join("nokey-cert.pem");
    let bundle = directory.join("nokey.p12");

    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "30",
        "-subj",
        "/CN=SIN CLAVE PRIVADA",
        "-keyout",
        key.to_str().expect("ruta valida"),
        "-out",
        certificate.to_str().expect("ruta valida"),
    ]);
    run_openssl(&[
        "pkcs12",
        "-export",
        "-nokeys",
        "-in",
        certificate.to_str().expect("ruta valida"),
        "-name",
        "SIN CLAVE PRIVADA",
        "-passout",
        &format!("pass:{GENERATED_PASSWORD}"),
        "-out",
        bundle.to_str().expect("ruta valida"),
    ]);

    bundle
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

/// Genera un `.p12` con una clave DSA y sin `friendlyName` en `directory`.
fn a_p12_of_an_unsupported_key_kind_without_a_friendly_name(directory: &Path) -> PathBuf {
    let parameters = directory.join("dsaparam-plain.pem");
    let key = directory.join("dsa-plain.pem");
    let certificate = directory.join("dsa-plain-cert.pem");
    let bundle = directory.join("dsa-plain.p12");

    run_openssl(&[
        "dsaparam",
        "-out",
        parameters.to_str().expect("ruta valida"),
        "2048",
    ]);
    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        &format!("dsa:{}", parameters.to_str().expect("ruta valida")),
        "-nodes",
        "-days",
        "30",
        "-subj",
        "/CN=CLAVE NO SOPORTADA SIN NOMBRE AMISTOSO",
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
        &format!("pass:{GENERATED_PASSWORD}"),
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

fn subject_of(der: &[u8]) -> String {
    x509_cert::Certificate::from_der(der)
        .expect("el DER deberia parsearse")
        .tbs_certificate()
        .subject()
        .to_string()
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
fn a_p12_without_a_friendly_name_installs_and_lists() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let plain =
        a_p12_without_a_friendly_name(workshop.path(), "/CN=SIN NOMBRE AMISTOSO", KIT_PASSWORD);

    install(installed.path(), &plain, KIT_PASSWORD)
        .expect("un .p12 sin friendlyName deberia instalarse igual");

    assert_eq!(certificates(installed.path()).len(), 1);
}

#[test]
fn a_p12_without_a_friendly_name_and_without_a_common_name_installs() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let plain = a_p12_without_a_friendly_name(workshop.path(), "/O=SIN NOMBRE COMUN", KIT_PASSWORD);

    install(installed.path(), &plain, KIT_PASSWORD)
        .expect("un .p12 sin friendlyName ni nombre comun deberia instalarse con el nickname fijo");

    assert_eq!(certificates(installed.path()).len(), 1);
}

#[test]
fn a_p12_of_an_unsupported_key_kind_without_a_friendly_name_gives_the_key_rejection_not_a_read_failure(
) {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let unsupported = a_p12_of_an_unsupported_key_kind_without_a_friendly_name(workshop.path());

    let failure = install(installed.path(), &unsupported, GENERATED_PASSWORD).expect_err(
        "una clave que no es RSA ni de curva eliptica no se puede instalar, con o sin friendlyName",
    );

    assert_eq!(failure.situation, "keyKindUnsupported");
}

#[test]
fn an_elliptic_curve_p12_installs_and_its_certificates_list_without_the_password() {
    let installed = an_empty_installation();

    install(installed.path(), &elliptic_curve_kit_p12(), KIT_PASSWORD)
        .expect("el .p12 de curva eliptica del kit deberia instalarse");

    let found = certificates(installed.path());
    assert_eq!(found.len(), 1, "el .p12 trae un certificado de persona");
    assert!(found[0]
        .subject()
        .is_some_and(|subject| subject.contains("99949991H")));
}

#[test]
fn a_p12_of_an_unsupported_key_kind_is_refused_at_install() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let unsupported = a_p12_of_an_unsupported_key_kind(workshop.path());

    let failure = install(installed.path(), &unsupported, GENERATED_PASSWORD)
        .expect_err("una clave que no es RSA ni de curva eliptica no se puede instalar");

    assert_eq!(failure.situation, "keyKindUnsupported");
}

#[test]
fn a_refused_p12_leaves_no_store_behind() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let unsupported = a_p12_of_an_unsupported_key_kind(workshop.path());

    let _ = install(installed.path(), &unsupported, GENERATED_PASSWORD);

    assert!(
        installed_stores(installed.path()).is_empty(),
        "el rechazo tenia que borrar el almacen a medio escribir"
    );
}

#[test]
fn a_refused_p12_leaves_an_already_installed_certificate_alone() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let unsupported = a_p12_of_an_unsupported_key_kind(workshop.path());
    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");

    let failure = install(installed.path(), &unsupported, GENERATED_PASSWORD)
        .expect_err("una clave que no es RSA ni de curva eliptica no se puede instalar");

    assert_eq!(failure.situation, "keyKindUnsupported");
    assert_eq!(
        certificates(installed.path()).len(),
        1,
        "el rechazo no puede llevarse lo que ya estaba instalado"
    );
}

#[test]
fn a_wrong_password_is_told_apart_from_a_key_that_does_not_serve() {
    let installed = an_empty_installation();

    let failure = install(installed.path(), &kit_p12(), "no es la suya")
        .expect_err("con otra contrasena no se puede abrir el fichero");

    assert_eq!(failure.situation, "incorrectPkcs12Password");
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn a_file_that_is_not_a_pkcs12_is_told_apart_from_a_wrong_password() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let not_a_p12 = workshop.path().join("not-a.p12");
    std::fs::write(&not_a_p12, b"esto no es un pkcs12").expect("deberia poder escribirse");

    let failure = install(installed.path(), &not_a_p12, KIT_PASSWORD)
        .expect_err("un fichero que no decodifica como pkcs12 no se puede instalar");

    assert_eq!(failure.situation, "pkcs12Unreadable");
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn a_p12_without_a_private_key_gives_its_own_situation() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let certificate_only = a_p12_without_a_private_key(workshop.path());

    let failure = install(installed.path(), &certificate_only, GENERATED_PASSWORD)
        .expect_err("un .p12 sin clave privada no se puede instalar");

    assert_eq!(failure.situation, "pkcs12NoPrivateKey");
    assert!(installed_stores(installed.path()).is_empty());
}

#[test]
fn a_p12_without_a_private_key_is_refused_even_over_an_already_installed_certificate() {
    let installed = an_empty_installation();
    let workshop = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let certificate_only = a_p12_without_a_private_key(workshop.path());
    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");

    let failure = install(installed.path(), &certificate_only, GENERATED_PASSWORD)
        .expect_err("un .p12 sin clave privada no se puede instalar, ni con el almacen ya poblado");

    assert_eq!(failure.situation, "pkcs12NoPrivateKey");
    assert_eq!(
        certificates(installed.path()).len(),
        1,
        "el rechazo no puede llevarse lo que ya estaba instalado"
    );
}

#[test]
fn nothing_of_the_file_is_kept_beyond_the_two_databases() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");

    let mut inside: Vec<String> = std::fs::read_dir(installed.path())
        .expect("deberia leerse")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    inside.sort();

    assert_eq!(inside, vec!["cert9.db".to_owned(), "key4.db".to_owned()]);
}

#[test]
fn reinstalling_the_same_file_does_not_duplicate_it() {
    let installed = an_empty_installation();

    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("reinstalar lo que ya esta es un exito sin cambios");

    assert_eq!(installed_stores(installed.path()).len(), 1);
    assert_eq!(certificates(installed.path()).len(), 1);
}

#[test]
fn two_different_files_land_in_the_same_store() {
    let installed = an_empty_installation();

    install(installed.path(), &kit_p12(), KIT_PASSWORD).expect("el primero deberia instalarse");
    install(installed.path(), &elliptic_curve_kit_p12(), KIT_PASSWORD)
        .expect("el segundo deberia instalarse");

    assert_eq!(
        installed_stores(installed.path()).len(),
        1,
        "el Almacen de rFirma es una unica base NSS"
    );
    assert_eq!(certificates(installed.path()).len(), 2);
}

#[test]
fn two_certificates_with_the_same_common_name_coexist() {
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

    assert_eq!(certificates(installed.path()).len(), 2);
}

#[test]
fn removing_an_installed_certificate_is_refused_instead_of_deleting_the_shared_store() {
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

    let failure = certificates::remove_installed(
        &RealInstalledFolder,
        installed.path(),
        &handles[0],
        &listed,
    )
    .expect_err("el borrado fino aun no existe: quitar no puede llevarse el almacen entero");

    assert_eq!(
        rfirma_lib::crossing::Failure::from(failure).situation,
        "removalNotSupported"
    );
    assert_eq!(
        installed_stores(installed.path()).len(),
        1,
        "el almacen compartido tiene que seguir intacto"
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

    let failure = certificates::remove_installed(
        &RealInstalledFolder,
        installed.path(),
        &handles[0],
        &listed,
    )
    .expect_err("no viene de este directorio");

    assert_eq!(
        rfirma_lib::crossing::Failure::from(failure).situation,
        "certificateNotFound"
    );
    assert_eq!(
        installed_stores(elsewhere.path()).len(),
        1,
        "el almacen de al lado sigue donde estaba"
    );
}

#[test]
fn a_certificate_from_a_p12_carries_the_authority_that_came_inside_it() {
    let installed = an_empty_installation();
    install(installed.path(), &kit_p12(), KIT_PASSWORD)
        .expect("el .p12 del kit deberia instalarse");

    let found = certificates::certificates_with_their_chains(
        &pkcs11::RealToken,
        &installed_stores(installed.path()),
    )
    .expect("el almacen del .p12 deberia listarse");

    let chain = found[0].chain();
    assert_eq!(chain.len(), 2, "el firmante y la intermedia de la FNMT");
    assert_eq!(chain[0], found[0].der(), "el firmante va delante");
    assert_eq!(
        subject_of(&chain[1]),
        found[0].issuer().expect("el firmante tiene emisor"),
        "detras va quien lo emitio"
    );
}
