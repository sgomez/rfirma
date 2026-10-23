use std::path::{Path, PathBuf};

use core_foundation::base::TCFType;
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkcs12::Pkcs12;
use openssl::pkey::{PKey, Private};
use openssl::rsa::{Padding as RsaPadding, Rsa};
use openssl::sign::{RsaPssSaltlen, Verifier};
use openssl::x509::extension::BasicConstraints;
use openssl::x509::{X509Name, X509};
use security_framework::certificate::SecCertificate;
use security_framework::import_export::Pkcs12ImportOptions;
use security_framework::os::macos::keychain::{CreateOptions, SecKeychain};

use super::keychain::{import_pkcs12 as install, situation_of};
use super::{installs, keychain_store, stores_besides, user_store, MacosToken};
use crate::identity::application::certificates::certificates_with_their_chains;
use crate::identity::application::readers::LastListing;
use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::Situation;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::Token;

const PASSWORD: &str = "rfirma-test";

#[link(name = "Security", kind = "framework")]
extern "C" {
    fn SecKeychainDelete(keychain: *mut std::ffi::c_void) -> i32;
}

/// Un llavero propio de la prueba, fuera de la lista de búsqueda de quien la corre, que se borra al acabar.
struct TemporaryKeychain {
    _directory: tempfile::TempDir,
    path: PathBuf,
    keychain: SecKeychain,
}

impl TemporaryKeychain {
    fn create() -> Self {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let path = directory.path().join("rfirma-test.keychain-db");
        let mut keychain = CreateOptions::new()
            .password(PASSWORD)
            .create(&path)
            .expect("deberia crearse el llavero de pruebas");
        keychain
            .unlock(Some(PASSWORD))
            .expect("deberia desbloquearse el llavero de pruebas");
        Self {
            _directory: directory,
            path,
            keychain,
        }
    }

    fn listed(&self) -> Vec<TokenCertificate> {
        MacosToken::default()
            .list(&keychain_store(&self.path))
            .expect("el llavero deberia listarse")
    }

    fn holds_an_identity(&self, certificate: &X509, key: &PKey<Private>) {
        Pkcs12ImportOptions::new()
            .passphrase(PASSWORD)
            .keychain(self.keychain.clone())
            .import(&pkcs12_of(certificate, Some(key), PASSWORD))
            .expect("el .p12 deberia entrar en el llavero de pruebas");
    }

    fn every(&self) -> Vec<TokenCertificate> {
        MacosToken::default()
            .every_certificate(&self.store())
            .expect("el llavero deberia listarse")
    }

    fn store(&self) -> Store {
        keychain_store(&self.path)
    }

    fn holding(key: &PKey<Private>) -> (Self, TokenCertificate) {
        let keychain = Self::create();
        keychain.holds_an_identity(&a_certificate("FIRMANTE", key, None, false), key);
        let listed = keychain.listed().remove(0);
        (keychain, listed)
    }

    fn holds_a_certificate(&self, certificate: &X509) {
        SecCertificate::from_der(&der_of(certificate))
            .and_then(|certificate| certificate.add_to_keychain(Some(self.keychain.clone())))
            .expect("el certificado deberia entrar en el llavero de pruebas");
    }
}

impl Drop for TemporaryKeychain {
    fn drop(&mut self) {
        unsafe {
            SecKeychainDelete(self.keychain.as_concrete_TypeRef().cast());
        }
    }
}

fn pkcs12_of(certificate: &X509, key: Option<&PKey<Private>>, password: &str) -> Vec<u8> {
    let mut builder = Pkcs12::builder();
    builder
        .name("rfirma-test")
        .cert(certificate)
        .key_algorithm(Nid::PBE_WITHSHA1AND3_KEY_TRIPLEDES_CBC)
        .cert_algorithm(Nid::PBE_WITHSHA1AND3_KEY_TRIPLEDES_CBC)
        .mac_md(MessageDigest::sha1());
    if let Some(key) = key {
        builder.pkey(key);
    }
    builder
        .build2(password)
        .and_then(|pkcs12| pkcs12.to_der())
        .expect("deberia construirse el .p12 de pruebas")
}

fn an_rsa_key() -> PKey<Private> {
    PKey::from_rsa(Rsa::generate(2048).expect("deberia generarse la clave RSA"))
        .expect("deberia envolverse la clave RSA")
}

fn an_elliptic_curve_key() -> PKey<Private> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).expect("deberia estar P-256");
    PKey::from_ec_key(EcKey::generate(&group).expect("deberia generarse la clave EC"))
        .expect("deberia envolverse la clave EC")
}

/// Un certificado de `common_name` para `key`, emitido por `issuer` o autofirmado.
fn a_certificate(
    common_name: &str,
    key: &PKey<Private>,
    issuer: Option<(&X509, &PKey<Private>)>,
    authority: bool,
) -> X509 {
    let mut name = X509Name::builder().expect("deberia construirse un nombre");
    name.append_entry_by_nid(Nid::COMMONNAME, common_name)
        .expect("el nombre comun deberia entrar");
    let name = name.build();
    let mut builder = X509::builder().expect("deberia construirse un certificado");
    builder.set_version(2).expect("la version deberia ponerse");
    let serial = BigNum::from_u32(rand_serial())
        .and_then(|serial| serial.to_asn1_integer())
        .expect("el serie deberia construirse");
    builder
        .set_serial_number(&serial)
        .expect("el serie deberia ponerse");
    builder
        .set_subject_name(&name)
        .expect("el titular deberia ponerse");
    builder
        .set_issuer_name(issuer.map_or(&name, |(certificate, _)| certificate.subject_name()))
        .expect("el emisor deberia ponerse");
    builder.set_pubkey(key).expect("la clave deberia ponerse");
    builder
        .set_not_before(&Asn1Time::days_from_now(0).expect("deberia haber fecha"))
        .expect("el inicio deberia ponerse");
    builder
        .set_not_after(&Asn1Time::days_from_now(30).expect("deberia haber fecha"))
        .expect("el fin deberia ponerse");
    if authority {
        builder
            .append_extension(
                BasicConstraints::new()
                    .critical()
                    .ca()
                    .build()
                    .expect("deberia construirse la extension"),
            )
            .expect("la extension deberia ponerse");
    }
    builder
        .sign(issuer.map_or(key, |(_, key)| key), MessageDigest::sha256())
        .expect("el certificado deberia firmarse");
    builder.build()
}

fn rand_serial() -> u32 {
    let mut bytes = [0u8; 4];
    openssl::rand::rand_bytes(&mut bytes).expect("deberia haber azar");
    u32::from_be_bytes(bytes) >> 1
}

fn der_of(certificate: &X509) -> Vec<u8> {
    certificate
        .to_der()
        .expect("el certificado deberia salir en DER")
}

#[test]
fn the_user_store_is_the_macos_store_and_not_a_card() {
    assert_eq!(user_store().class(), StoreClass::Macos);
    assert_eq!(
        keychain_store(Path::new("/tmp/otro.keychain-db")).class(),
        StoreClass::Macos
    );
}

#[test]
fn the_macos_store_comes_before_the_pkcs11_stores_unless_a_module_is_named() {
    let card = Store::module("/opt/homebrew/lib/opensc-pkcs11.so");

    assert_eq!(
        stores_besides(vec![card.clone()], false),
        vec![user_store(), card.clone()]
    );
    assert_eq!(stores_besides(vec![card.clone()], true), vec![card]);
}

#[test]
fn macos_asks_for_the_secret_and_never_warns_about_the_pin() {
    let keychain = TemporaryKeychain::create();
    let key = an_rsa_key();
    keychain.holds_an_identity(&a_certificate("FIRMA RSA", &key, None, false), &key);
    let listed = keychain.listed();
    let reference = listed[0].reference();

    assert_eq!(
        MacosToken::default().secret_of(reference),
        Ok(StoreSecret::NotNeeded)
    );
    assert_eq!(
        MacosToken::default().pin_warning(reference),
        Ok(PinWarning::Quiet)
    );
}

#[test]
fn lists_an_rsa_and_an_elliptic_curve_certificate_and_leaves_out_one_without_its_key() {
    let keychain = TemporaryKeychain::create();
    let rsa = an_rsa_key();
    let rsa_certificate = a_certificate("FIRMA RSA", &rsa, None, false);
    let ec = an_elliptic_curve_key();
    let ec_certificate = a_certificate("FIRMA CURVA ELIPTICA", &ec, None, false);
    let keyless = a_certificate("SIN CLAVE", &an_rsa_key(), None, false);
    keychain.holds_an_identity(&rsa_certificate, &rsa);
    keychain.holds_an_identity(&ec_certificate, &ec);
    keychain.holds_a_certificate(&keyless);

    let mut listed: Vec<String> = keychain
        .listed()
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    listed.sort();

    assert_eq!(listed, vec!["FIRMA CURVA ELIPTICA", "FIRMA RSA"]);
}

#[test]
fn a_certificate_with_its_key_in_software_stays_in_the_macos_store() {
    let keychain = TemporaryKeychain::create();
    let key = an_rsa_key();
    keychain.holds_an_identity(&a_certificate("FIRMA RSA", &key, None, false), &key);

    let listed = keychain.listed();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].reference().key_provider(), None);
    assert_eq!(
        listed[0].reference().class_under(Path::new("/nowhere")),
        StoreClass::Macos
    );
    assert_eq!(listed[0].reference().label(), "FIRMA RSA");
}

#[test]
fn the_chain_of_a_keychain_certificate_is_completed_with_its_issuer_from_the_keychain() {
    let keychain = TemporaryKeychain::create();
    let root_key = an_rsa_key();
    let root = a_certificate("RAIZ DE PRUEBAS", &root_key, None, true);
    let intermediate_key = an_rsa_key();
    let intermediate = a_certificate(
        "INTERMEDIA DE PRUEBAS",
        &intermediate_key,
        Some((&root, &root_key)),
        true,
    );
    let key = an_rsa_key();
    let signer = a_certificate(
        "FIRMANTE",
        &key,
        Some((&intermediate, &intermediate_key)),
        false,
    );
    keychain.holds_a_certificate(&root);
    keychain.holds_a_certificate(&intermediate);
    keychain.holds_an_identity(&signer, &key);

    let offered = certificates_with_their_chains(
        &MacosToken::default(),
        &[keychain_store(&keychain.path)],
        &LastListing::default(),
    )
    .expect("el llavero deberia listarse");

    assert_eq!(offered.len(), 1);
    let chain: Vec<String> = offered[0]
        .chain()
        .iter()
        .map(|der| common_name_of(der))
        .collect();
    assert_eq!(chain, vec!["FIRMANTE", "INTERMEDIA DE PRUEBAS"]);
}

fn common_name_of(der: &[u8]) -> String {
    let certificate = X509::from_der(der).expect("deberia leerse el certificado");
    let entry = certificate
        .subject_name()
        .entries_by_nid(Nid::COMMONNAME)
        .next()
        .expect("deberia tener nombre comun");
    entry
        .data()
        .to_string()
        .expect("el nombre comun deberia leerse")
}

const DATA: &[u8] = b"lo que Java manda firmar";

fn signed(certificate: &TokenCertificate, algorithm: SignatureAlgorithm) -> Vec<u8> {
    MacosToken::default()
        .sign_with_secret(
            certificate.reference(),
            &ProtectedSecret::new(b""),
            algorithm,
            DATA,
        )
        .expect("el llavero deberia firmar")
}

fn verifies(
    certificate: &TokenCertificate,
    algorithm: SignatureAlgorithm,
    signature: &[u8],
) -> bool {
    let key = X509::from_der(certificate.der())
        .and_then(|certificate| certificate.public_key())
        .expect("el certificado deberia traer su clave publica");
    let digest = digest_of(algorithm);
    let mut verifier = Verifier::new(digest, &key).expect("verificador");
    if algorithm.name().ends_with("MGF1") {
        with_pss_padding(&mut verifier, digest);
    }
    verifier.verify_oneshot(signature, DATA).unwrap_or(false)
}

fn digest_of(algorithm: SignatureAlgorithm) -> MessageDigest {
    match algorithm {
        SignatureAlgorithm::Sha1Rsa | SignatureAlgorithm::Sha1Ecdsa => MessageDigest::sha1(),
        SignatureAlgorithm::Sha384Rsa
        | SignatureAlgorithm::Sha384RsaPss
        | SignatureAlgorithm::Sha384Ecdsa => MessageDigest::sha384(),
        SignatureAlgorithm::Sha512Rsa
        | SignatureAlgorithm::Sha512RsaPss
        | SignatureAlgorithm::Sha512Ecdsa => MessageDigest::sha512(),
        _ => MessageDigest::sha256(),
    }
}

fn with_pss_padding(verifier: &mut Verifier<'_>, digest: MessageDigest) {
    verifier.set_rsa_padding(RsaPadding::PKCS1_PSS).unwrap();
    verifier
        .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
        .unwrap();
    verifier.set_rsa_mgf1_md(digest).unwrap();
}

fn algorithms_for(kind: KeyKind) -> Vec<SignatureAlgorithm> {
    SignatureAlgorithm::ALL
        .into_iter()
        .filter(|algorithm| algorithm.key_kind() == kind)
        .collect()
}

#[test]
fn signs_with_rsa_pkcs1_and_pss_through_the_keychain_with_every_hash_sha1_included() {
    let (_keychain, certificate) = TemporaryKeychain::holding(&an_rsa_key());

    for algorithm in algorithms_for(KeyKind::Rsa) {
        let signature = signed(&certificate, algorithm);

        assert!(
            verifies(&certificate, algorithm, &signature),
            "{algorithm:?}"
        );
    }
}

#[test]
fn signs_with_ecdsa_through_the_keychain_in_der_with_every_hash_sha1_included() {
    let (_keychain, certificate) = TemporaryKeychain::holding(&an_elliptic_curve_key());

    for algorithm in algorithms_for(KeyKind::Ec) {
        let signature = signed(&certificate, algorithm);

        assert_eq!(signature[0], 0x30, "ECDSA viaja en DER, no en r||s");
        assert!(
            verifies(&certificate, algorithm, &signature),
            "{algorithm:?}"
        );
    }
}

#[test]
fn a_keychain_key_offers_only_the_algorithms_of_its_kind() {
    let (_keychain, certificate) = TemporaryKeychain::holding(&an_rsa_key());

    assert_eq!(
        MacosToken::default().offers(certificate.reference(), SignatureAlgorithm::Sha256RsaPss),
        Ok(())
    );
    assert_eq!(
        MacosToken::default()
            .offers(certificate.reference(), SignatureAlgorithm::Sha256Ecdsa)
            .map_err(|error| error.situation()),
        Err(Situation::MechanismNotOffered)
    );
}

#[test]
fn a_certificate_gone_from_the_keychain_is_not_found_when_signing() {
    let (keychain, certificate) = TemporaryKeychain::holding(&an_rsa_key());
    let other = TemporaryKeychain::create();
    let moved = CertificateRef::new(
        keychain_store(&other.path),
        certificate.reference().token_label(),
        certificate.reference().label(),
        certificate
            .reference()
            .cka_id()
            .unwrap_or_default()
            .to_vec(),
    );
    drop(keychain);

    assert_eq!(
        MacosToken::default()
            .sign_with_secret(
                &moved,
                &ProtectedSecret::new(b""),
                SignatureAlgorithm::Sha256Rsa,
                DATA
            )
            .map_err(|error| error.situation()),
        Err(Situation::CertificateNotFound)
    );
}

const OS_STATUS: &str = "NSOSStatusErrorDomain";
const CRYPTO_TOKEN_KIT: &str = "CryptoTokenKit";

#[test]
fn denying_the_macos_dialog_is_the_person_cancelling() {
    assert_eq!(situation_of(OS_STATUS, -128), Situation::PinEntryCancelled);
    assert_eq!(
        situation_of(CRYPTO_TOKEN_KIT, -4),
        Situation::PinEntryCancelled
    );
}

#[test]
fn system_errors_map_to_their_situations() {
    assert_eq!(situation_of(OS_STATUS, -25293), Situation::IncorrectPin);
    assert_eq!(situation_of(CRYPTO_TOKEN_KIT, -5), Situation::IncorrectPin);
    assert_eq!(
        situation_of(OS_STATUS, 0x8010_006B_u32 as i64),
        Situation::IncorrectPin
    );
    assert_eq!(
        situation_of(OS_STATUS, 0x8010_006C_u32 as i64),
        Situation::PinLocked
    );
    assert_eq!(situation_of(CRYPTO_TOKEN_KIT, -7), Situation::TokenAbsent);
    assert_eq!(
        situation_of(OS_STATUS, 0x8010_0069_u32 as i64),
        Situation::TokenAbsent
    );
    assert_eq!(
        situation_of(OS_STATUS, 0x8010_000C_u32 as i64),
        Situation::TokenAbsent
    );
    assert_eq!(
        situation_of(OS_STATUS, -25300),
        Situation::CertificateNotFound
    );
    assert_eq!(
        situation_of(CRYPTO_TOKEN_KIT, -6),
        Situation::CertificateNotFound
    );
    assert_eq!(situation_of(OS_STATUS, -4), Situation::Unknown);
    assert_eq!(situation_of("NSCocoaErrorDomain", -128), Situation::Unknown);
}

#[test]
fn installing_a_pkcs12_puts_its_identity_in_the_keychain_for_rsa_and_elliptic_curve() {
    let keychain = TemporaryKeychain::create();
    let rsa = an_rsa_key();
    let ec = an_elliptic_curve_key();

    let mut thumbprints = Vec::new();
    for (name, key) in [("INSTALADO RSA", &rsa), ("INSTALADO CURVA", &ec)] {
        let pkcs12 = pkcs12_of(&a_certificate(name, key, None, false), Some(key), "clave");
        thumbprints
            .extend(install(&keychain.store(), &pkcs12, "clave").expect("deberia instalarse"));
    }

    let listed = keychain.listed();
    let mut labels: Vec<String> = listed
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    labels.sort();
    assert_eq!(labels, vec!["INSTALADO CURVA", "INSTALADO RSA"]);
    let mut listed_thumbprints: Vec<Vec<u8>> = listed
        .iter()
        .filter_map(|certificate| certificate.reference().cka_id().map(<[u8]>::to_vec))
        .collect();
    listed_thumbprints.sort();
    thumbprints.sort();
    assert_eq!(thumbprints, listed_thumbprints);
}

#[test]
fn the_record_of_what_rfirma_installed_keeps_each_thumbprint_until_it_is_removed() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let directory = home.path().join("certificates");

    installs::record(&directory, &[vec![0x01], vec![0x02]]).expect("deberia apuntarse");
    installs::forget(&directory, &[0x01]).expect("deberia borrarse");

    assert!(!installs::holds(&directory, &[0x01]));
    assert!(installs::holds(&directory, &[0x02]));
}

#[test]
fn a_wrong_pkcs12_password_installs_nothing() {
    let keychain = TemporaryKeychain::create();
    let key = an_rsa_key();
    let pkcs12 = pkcs12_of(
        &a_certificate("FIRMA", &key, None, false),
        Some(&key),
        "clave",
    );

    let failure = install(&keychain.store(), &pkcs12, "otra").expect_err("la clave es otra");

    assert_eq!(failure.situation(), Situation::IncorrectPkcs12Password);
    assert!(keychain.listed().is_empty());
}

#[test]
fn a_pkcs12_without_a_private_key_installs_nothing() {
    let keychain = TemporaryKeychain::create();
    let key = an_rsa_key();
    let pkcs12 = pkcs12_of(
        &a_certificate("SIN CLAVE", &key, None, false),
        None,
        "clave",
    );

    let failure = install(&keychain.store(), &pkcs12, "clave").expect_err("no trae clave");

    assert_eq!(failure.situation(), Situation::Pkcs12NoPrivateKey);
    assert!(keychain.every().is_empty());
}

#[test]
fn removing_an_installed_certificate_deletes_it_and_its_key_from_the_keychain() {
    let keychain = TemporaryKeychain::create();
    let (kept_key, gone_key) = (an_rsa_key(), an_rsa_key());
    keychain.holds_an_identity(
        &a_certificate("SE QUEDA", &kept_key, None, false),
        &kept_key,
    );
    keychain.holds_an_identity(&a_certificate("SE VA", &gone_key, None, false), &gone_key);
    let gone = keychain
        .listed()
        .into_iter()
        .find(|certificate| certificate.reference().label() == "SE VA")
        .expect("deberia estar en el llavero");

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let thumbprint = gone.reference().cka_id().expect("lleva huella").to_vec();
    installs::record(home.path(), &[thumbprint]).expect("deberia apuntarse");
    assert!(MacosToken::default().installed_in_the_system(home.path(), gone.reference()));

    MacosToken::default()
        .remove_certificate(home.path(), gone.reference(), &ProtectedSecret::new(b""))
        .expect("deberia quitarse");

    let labels: Vec<String> = keychain
        .listed()
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    assert_eq!(labels, vec!["SE QUEDA"]);
    assert!(!MacosToken::default().installed_in_the_system(home.path(), gone.reference()));
}

#[test]
fn a_keychain_certificate_rfirma_did_not_install_is_not_installed_in_the_system() {
    let keychain = TemporaryKeychain::create();
    let key = an_rsa_key();
    keychain.holds_an_identity(&a_certificate("DE SAFARI", &key, None, false), &key);
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let listed = keychain.listed();

    assert!(!MacosToken::default().installed_in_the_system(home.path(), listed[0].reference()));
}

#[test]
fn macos_installs_pkcs12_in_the_system() {
    assert!(MacosToken::default().installs_in_the_system());
}
