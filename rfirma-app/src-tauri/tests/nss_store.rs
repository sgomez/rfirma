//! Pruebas de integración del almacén NSS como módulo PKCS#11 (ADR-0014).

use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::memory::ListedCertificates;
use rfirma_lib::pkcs11::{self, CertificateStatus, Situation, Store, StoreClass, TokenCertificate};
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::Sha256;
use x509_cert::der::{Decode, Encode};

/// Apodo que NSS pone a los certificados de persona.
const HOLDER: &str = "EIDAS_CERTIFICADO_PRUEBAS___99999999R";
/// Token del perfil NSS.
const CERTIFICATE_DB: &str = "NSS Certificate DB";
/// Contraseña maestra vacía de un perfil recién instalado.
const NO_MASTER_PASSWORD: &str = "";

/// Bloque DER de SignedAttributes para firmar.
const PRESIGN: &[u8] = b"31 5f 30 18 06 09 2a 86 SignedAttributes de mentira, sin hashear";

fn softoken() -> PathBuf {
    pkcs11::stores::present_among(pkcs11::stores::CANDIDATE_SOFTOKENS, |path| path.is_file())
        .into_iter()
        .next()
        .expect(
            "falta libsoftokn3.so. Las pruebas de grada B del almacen NSS lo necesitan:\n  \
             sudo apt install -y libnss3 libnss3-tools",
        )
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("la raiz del repositorio")
        .to_path_buf()
}

/// Perfil NSS recién provisionado en un directorio temporal.
fn a_disposable_profile() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let script = repository_root().join("testdata/nss/provision-profile.sh");

    let provisioned = Command::new("bash")
        .arg(&script)
        .arg(directory.path())
        .output()
        .expect("deberia poder ejecutarse el script de aprovisionamiento");

    assert!(
        provisioned.status.success(),
        "no se ha podido montar el perfil NSS con {}:\n{}\n{}",
        script.display(),
        String::from_utf8_lossy(&provisioned.stdout),
        String::from_utf8_lossy(&provisioned.stderr),
    );

    let store = Store::nss(softoken(), directory.path());
    (directory, store)
}

/// Perfil NSS provisionado con contraseña maestra.
fn a_disposable_profile_with_a_master_password() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let script = repository_root().join("testdata/nss/provision-profile.sh");

    let provisioned = Command::new("bash")
        .arg(&script)
        .arg(directory.path())
        .arg("secreto")
        .output()
        .expect("deberia poder ejecutarse el script de aprovisionamiento");

    assert!(
        provisioned.status.success(),
        "no se ha podido montar el perfil NSS con {}:\n{}\n{}",
        script.display(),
        String::from_utf8_lossy(&provisioned.stdout),
        String::from_utf8_lossy(&provisioned.stderr),
    );

    let store = Store::nss(softoken(), directory.path());
    (directory, store)
}

fn certificates(store: &Store) -> Vec<TokenCertificate> {
    pkcs11::list_certificates(store).expect("el perfil NSS deberia listarse")
}

/// Certificado de persona en vigor del perfil.
fn the_valid_one(store: &Store) -> TokenCertificate {
    certificates(store)
        .into_iter()
        .find(|certificate| certificate.status().is_usable())
        .expect("el perfil tenia que traer un certificado en vigor")
}

#[test]
fn a_firefox_profile_is_listed_like_any_other_pkcs11_store() {
    let (_profile, store) = a_disposable_profile();

    let found = certificates(&store);

    assert!(
        found
            .iter()
            .any(|certificate| certificate.reference().label() == HOLDER),
        "el certificado del titular tenia que salir; salieron: {:?}",
        found
            .iter()
            .map(|certificate| certificate.reference().label())
            .collect::<Vec<_>>()
    );
    let valid = the_valid_one(&store);
    assert_eq!(valid.reference().token_label(), CERTIFICATE_DB);
    assert!(
        valid.reference().cka_id().is_some(),
        "sin CKA_ID no hay forma de emparejar el certificado con su clave"
    );
}

#[test]
fn a_certificate_without_a_private_key_is_filtered_out_and_the_holders_is_not() {
    let (_profile, store) = a_disposable_profile();

    let found = certificates(&store);

    assert!(
        found
            .iter()
            .any(|certificate| certificate.reference().label() == HOLDER),
        "el certificado del titular tiene clave y tenia que salir"
    );
    assert!(
        !found
            .iter()
            .any(|certificate| certificate.reference().label().contains("AC ")),
        "la CA suelta no tiene clave privada y no deberia salir en la lista firmable"
    );
}

#[test]
fn a_profile_with_a_real_master_password_lists_nothing_without_it() {
    let (_profile, store) = a_disposable_profile_with_a_master_password();

    let found = pkcs11::list_certificates(&store).expect("listar no deberia pedir la contrasena");

    assert!(
        found.is_empty(),
        "sin la contrasena maestra no deberia verse ninguna clave privada, \
         asi que la lista firmable tenia que salir vacia: {found:?}"
    );
}

#[test]
fn the_holder_and_the_issuer_are_read_the_same_as_from_a_card() {
    let (_profile, store) = a_disposable_profile();

    let certificate = the_valid_one(&store);

    let subject = certificate.subject().expect("el DER deberia leerse");
    assert!(
        subject.contains("EIDAS CERTIFICADO PRUEBAS"),
        "titular leido: {subject}"
    );
    assert!(
        subject.contains("IDCES-99999999R"),
        "el DNI tenia que leerse del subject: {subject}"
    );
    let issuer = certificate.issuer().expect("el DER deberia leerse");
    assert!(
        issuer.contains("AC FNMT Usuarios"),
        "emisor leido: {issuer}"
    );
}

#[test]
fn an_expired_certificate_from_nss_is_marked_as_expired() {
    let (_profile, store) = a_disposable_profile();

    let expired: Vec<_> = certificates(&store)
        .into_iter()
        .filter(|certificate| matches!(certificate.status(), CertificateStatus::Expired { .. }))
        .collect();

    assert!(
        expired
            .iter()
            .any(|certificate| certificate.reference().label() == HOLDER),
        "el certificado que caduco en 2020 tenia que salir marcado como caducado"
    );
}

#[test]
fn two_certificates_share_the_label_and_are_told_apart_by_their_cka_id() {
    let (_profile, store) = a_disposable_profile();

    let same_label: Vec<_> = certificates(&store)
        .into_iter()
        .filter(|certificate| certificate.reference().label() == HOLDER)
        .collect();

    assert_eq!(
        same_label.len(),
        2,
        "el perfil tenia que traer los dos certificados del titular"
    );
    assert_ne!(
        same_label[0].reference().cka_id(),
        same_label[1].reference().cka_id(),
        "dos certificados con la misma etiqueta tienen que distinguirse por CKA_ID"
    );
}

#[test]
fn each_certificate_sharing_a_label_comes_back_by_its_own_handle() {
    let (_profile, store) = a_disposable_profile();
    let found = certificates(&store);
    let listed = ListedCertificates::new();

    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );

    let holders: Vec<(&String, &TokenCertificate)> = handles
        .iter()
        .zip(found.iter())
        .filter(|(_, certificate)| certificate.reference().label() == HOLDER)
        .collect();
    assert_eq!(holders.len(), 2, "el perfil tenia que traer los dos");
    assert_ne!(holders[0].0, holders[1].0, "dos filas, dos asas");
    for (handle, certificate) in holders {
        assert_eq!(listed.get(handle).as_ref(), Some(certificate.reference()));
    }
}

#[test]
fn the_handle_carries_nothing_of_the_profile_it_came_from() {
    let (profile, store) = a_disposable_profile();
    let listed = ListedCertificates::new();

    let handles = listed.replace([the_valid_one(&store).reference().clone()]);

    let handle = &handles[0];
    assert_eq!(handle.len(), 32);
    assert!(handle.chars().all(|letter| letter.is_ascii_hexdigit()));
    for leak in ["/", "tmp", HOLDER, CERTIFICATE_DB] {
        assert!(!handle.contains(leak), "el asa «{handle}» lleva «{leak}»");
    }
    let name = profile
        .path()
        .file_name()
        .expect("el perfil tiene nombre")
        .to_string_lossy()
        .into_owned();
    assert!(!handle.contains(&name), "el asa lleva el nombre del perfil");
}

#[test]
fn an_nss_store_says_its_class_and_never_its_configdir() {
    let (profile, store) = a_disposable_profile();

    let class = the_valid_one(&store).reference().store().class();

    assert_eq!(class, StoreClass::Nssdb);
    assert_eq!(
        Store::nss(
            softoken(),
            &profile.path().join(".mozilla/firefox/x.default")
        )
        .class(),
        StoreClass::Firefox
    );
    assert_eq!(
        Store::nss(softoken(), &profile.path().join(".pki/nssdb")).class(),
        StoreClass::Chrome
    );
}

#[test]
fn two_profiles_are_read_as_two_stores_and_not_as_the_first_one_twice() {
    let (_first, one) = a_disposable_profile();
    let (_second, other) = a_disposable_profile();

    let both = pkcs11::list_certificates_across(&[one.clone(), other.clone()])
        .expect("los dos perfiles tenian que listarse");
    let alone = certificates(&one);

    assert_eq!(
        both.len(),
        alone.len() * 2,
        "dos perfiles tienen que dar el doble de certificados que uno"
    );
    assert_ne!(one.init_args(), other.init_args());
}

#[test]
fn opening_the_store_with_the_wrong_init_args_is_a_failure_and_not_an_empty_list() {
    let nowhere = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let store = Store::nss(softoken(), nowhere.path());

    let error = pkcs11::list_certificates(&store)
        .expect_err("un almacen abierto donde no hay perfil no puede parecer un perfil vacio");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
    assert!(
        error.detail().contains("configdir"),
        "el detalle tiene que decir con que init args se iba a abrir: {}",
        error.detail()
    );
}

#[test]
fn a_profile_that_leads_nowhere_does_not_hide_the_one_that_works() {
    let (_profile, works) = a_disposable_profile();
    let nowhere = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    let found =
        pkcs11::list_certificates_across(&[Store::nss(softoken(), nowhere.path()), works.clone()])
            .expect("el perfil que si abre tiene que seguir contando");

    assert!(found
        .iter()
        .any(|certificate| certificate.reference().label() == HOLDER));
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
fn signing_with_an_nss_certificate_verifies_against_its_public_key() {
    let (_profile, store) = a_disposable_profile();
    let certificate = the_valid_one(&store);

    let raw = pkcs11::sign(certificate.reference(), NO_MASTER_PASSWORD, PRESIGN)
        .expect("un perfil sin contrasena maestra tiene que poder firmar con la cadena vacia");

    assert_eq!(raw.len(), 256);
    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

#[test]
fn a_remembered_nss_certificate_still_signs_after_a_round_trip_through_the_state_file() {
    let (_profile, store) = a_disposable_profile();
    let certificate = the_valid_one(&store);

    let written = serde_json::to_string(certificate.reference()).expect("deberia serializarse");
    let remembered: pkcs11::CertificateRef =
        serde_json::from_str(&written).expect("deberia leerse");

    let raw = pkcs11::sign(&remembered, NO_MASTER_PASSWORD, PRESIGN)
        .expect("la referencia recordada tenia que volver a encontrar su perfil");

    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

#[test]
fn a_certificate_without_a_private_key_says_so_instead_of_failing_generically() {
    let (_profile, store) = a_disposable_profile();
    let authority = pkcs11::list_certificates_unfiltered_for_test(&store)
        .expect("el perfil deberia listarse sin filtrar")
        .into_iter()
        .find(|certificate| certificate.reference().label().contains("AC "))
        .expect("el perfil tenia que traer alguna CA suelta");

    let error = pkcs11::sign(authority.reference(), NO_MASTER_PASSWORD, PRESIGN)
        .expect_err("una CA no tiene con que firmar");

    assert_eq!(error.situation(), Situation::CertificateNotFound);
}
