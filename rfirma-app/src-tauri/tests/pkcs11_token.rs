//! Pruebas de integración del backend contra el módulo PKCS#11 SoftHSM (ADR-0014).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rfirma_lib::identity::adapters::pkcs11::{
    self, CertificateRef, CertificateStatus, Situation, StoreClass, TokenCertificate, TokenError,
};
use rfirma_lib::identity::application::listed::ListedCertificates;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::{Digest, Sha256};
use x509_cert::der::{Decode, Encode};

const TOKEN: &str = "rfirma-test";
const PIN: &str = "1234";
const ACTIVE: &str = "FNMT-ACTIVO-99999999R";
const EXPIRED: &str = "FNMT-CADUCADO-99999999R";
const REVOKED: &str = "FNMT-REVOCADO-99999999R";
/// Dos certificados que comparten etiqueta y no comparten clave.
const TWIN: &str = "FNMT-GEMELO-99999999R";
const TWIN_OF_THE_ACTIVE_KEY: u8 = 0x04;
const TWIN_OF_THE_EXPIRED_KEY: u8 = 0x05;

/// Bloque DER de SignedAttributes para firmar.
const PRESIGN: &[u8] = b"31 5f 30 18 06 09 2a 86 SignedAttributes de mentira, sin hashear";

fn module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Las pruebas de grada B necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just token",
        module.display()
    );
    module
}

fn certificates() -> Vec<TokenCertificate> {
    let found = pkcs11::list_certificates(module()).expect("no se ha podido listar el token");
    assert!(
        !found.is_empty(),
        "el token {TOKEN} esta vacio o no existe. Montalo con:\n  just token"
    );
    found
}

fn certificate_labelled(label: &str) -> TokenCertificate {
    certificates()
        .into_iter()
        .find(|certificate| certificate.reference().label() == label)
        .unwrap_or_else(|| {
            panic!("el token {TOKEN} no tiene ningun certificado {label}. Montalo con: just token")
        })
}

/// Referencia tal y como sale del token con su CKA_ID.
fn reference(label: &str) -> CertificateRef {
    certificate_labelled(label).reference().clone()
}

fn certificate_with_cka_id(cka_id: u8) -> TokenCertificate {
    certificates()
        .into_iter()
        .find(|certificate| certificate.reference().cka_id() == Some([cka_id].as_slice()))
        .unwrap_or_else(|| {
            panic!(
                "el token {TOKEN} no tiene ningun certificado con CKA_ID {cka_id:02x}. \
                 Montalo con: just token"
            )
        })
}

fn epoch(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn listing_gives_back_what_it_takes_to_find_each_certificate_again() {
    let certificate = certificate_labelled(ACTIVE);
    let reference = certificate.reference();

    assert_eq!(reference.module(), module().as_path());
    assert_eq!(reference.token_label(), TOKEN);
    assert_eq!(reference.label(), ACTIVE);
    assert_eq!(reference.cka_id(), Some([0x01].as_slice()));
}

#[test]
fn the_holder_is_readable_for_display_but_is_not_part_of_the_reference() {
    let certificate = certificate_labelled(ACTIVE);

    let subject = certificate.subject().expect("el DER deberia leerse");
    assert!(
        subject.contains("EIDAS CERTIFICADO PRUEBAS"),
        "titular leido: {subject}"
    );

    let reference = format!("{:?}", certificate.reference());
    assert!(
        !reference.contains("EIDAS CERTIFICADO PRUEBAS"),
        "la referencia persistible no puede llevar el titular: {reference}"
    );
}

#[test]
fn the_issuer_is_the_authority_and_the_subject_has_no_organisation_to_confuse_it_with() {
    let certificate = certificate_labelled(ACTIVE);

    let issuer = certificate.issuer().expect("el DER deberia leerse");
    let subject = certificate.subject().expect("el DER deberia leerse");

    assert!(
        issuer.contains("AC FNMT Usuarios"),
        "emisor leido: {issuer}"
    );
    assert!(
        !subject.contains("O="),
        "el subject de este certificado no lleva organizacion: {subject}"
    );
}

#[test]
fn listing_without_a_session_still_lists_them() {
    let found = pkcs11::list_certificates(module()).expect("no deberia fallar sin PIN");
    assert!(
        found
            .iter()
            .any(|certificate| certificate.reference().label() == ACTIVE),
        "el certificado activo tenia que salir sin PIN"
    );
    assert_eq!(
        found.len(),
        5,
        "el token de pruebas tiene cinco certificados, todos con clave"
    );
}

#[test]
fn an_expired_certificate_is_told_apart_from_a_token_failure() {
    let status = certificate_labelled(EXPIRED).status();

    match status {
        CertificateStatus::Expired { not_after } => {
            assert_eq!(not_after, 1_604_839_715);
        }
        other => panic!("el certificado caducado se ha clasificado como {other:?}"),
    }
}

#[test]
fn a_certificate_in_date_is_usable_even_if_its_issuer_revoked_it() {
    for label in [ACTIVE, REVOKED] {
        assert!(
            certificate_labelled(label).status().is_usable(),
            "{label} deberia estar en vigor"
        );
    }
}

#[test]
fn the_same_certificate_changes_status_with_the_clock_and_not_with_the_token() {
    let certificate = certificate_labelled(ACTIVE);

    assert!(certificate.status_at(epoch(1_856_513_218)).is_usable());
    assert!(matches!(
        certificate.status_at(epoch(1_856_513_220)),
        CertificateStatus::Expired { .. }
    ));
    assert!(matches!(
        certificate.status_at(epoch(0)),
        CertificateStatus::NotYetValid { .. }
    ));
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
fn signing_produces_a_signature_that_the_certificate_public_key_verifies() {
    let certificate = certificate_labelled(ACTIVE);
    let raw = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("la firma deberia salir");

    assert_eq!(raw.len(), 256);

    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

#[test]
fn signing_a_hash_with_the_bare_rsa_mechanism_would_not_verify() {
    let certificate = certificate_labelled(ACTIVE);
    let key = verifying_key(&certificate);

    let ours = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("la firma deberia salir");
    let over_a_hash = sign_with_bare_rsa_pkcs(&Sha256::digest(PRESIGN));

    assert!(
        key.verify(PRESIGN, &Signature::try_from(ours.as_slice()).unwrap())
            .is_ok(),
        "la firma de rfirma tiene que verificar sobre los bytes SIN hashear"
    );
    assert!(
        key.verify(
            PRESIGN,
            &Signature::try_from(over_a_hash.as_slice()).unwrap()
        )
        .is_err(),
        "CKM_RSA_PKCS sobre un hash ha verificado: el mecanismo del ID-16 ya no \
         es el que dice serlo"
    );
    assert_ne!(ours, over_a_hash);
}

/// Mecanismo CKM_RSA_PKCS invocado a mano como contraejemplo.
fn sign_with_bare_rsa_pkcs(data: &[u8]) -> Vec<u8> {
    pkcs11::with_token_turn(|| sign_with_bare_rsa_pkcs_holding_the_turn(data))
}

fn sign_with_bare_rsa_pkcs_holding_the_turn(data: &[u8]) -> Vec<u8> {
    use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
    use cryptoki::mechanism::Mechanism;
    use cryptoki::object::{Attribute, ObjectClass};
    use cryptoki::session::UserType;
    use cryptoki::types::AuthPin;

    let context = Pkcs11::new(module()).expect("modulo");
    let _ = context.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK));

    let slot = context
        .get_slots_with_token()
        .expect("ranuras")
        .into_iter()
        .find(|slot| {
            context
                .get_token_info(*slot)
                .map(|info| info.label().trim() == TOKEN)
                .unwrap_or(false)
        })
        .expect("el token rfirma-test");

    let session = context.open_ro_session(slot).expect("sesion");
    session
        .login(UserType::User, Some(&AuthPin::new(PIN.into())))
        .expect("el login del contraejemplo, con el turno del token cogido");
    let key = session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(ACTIVE.as_bytes().to_vec()),
        ])
        .expect("busqueda")
        .into_iter()
        .next()
        .expect("la clave del camino feliz");

    let signature = session
        .sign(&Mechanism::RsaPkcs, key, data)
        .expect("CKM_RSA_PKCS deberia firmar cualquier bloque que le quepa");

    let _ = session.logout();

    signature
}

#[test]
fn signing_the_same_bytes_twice_gives_the_same_signature() {
    let once = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("firma");
    let twice = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("firma");

    assert_eq!(once, twice);
}

#[test]
fn two_certificates_sharing_a_label_each_sign_with_their_own_key() {
    let one = certificate_with_cka_id(TWIN_OF_THE_ACTIVE_KEY);
    let other = certificate_with_cka_id(TWIN_OF_THE_EXPIRED_KEY);

    assert_eq!(one.reference().label(), TWIN);
    assert_eq!(other.reference().label(), TWIN);
    assert_ne!(one.reference().cka_id(), other.reference().cka_id());

    let signed_by_one = pkcs11::sign(one.reference(), PIN, PRESIGN).expect("firma del primero");
    let signed_by_other = pkcs11::sign(other.reference(), PIN, PRESIGN).expect("firma del segundo");

    for (certificate, signature, twin) in [
        (&one, &signed_by_one, &other),
        (&other, &signed_by_other, &one),
    ] {
        let signature = Signature::try_from(signature.as_slice()).expect("firma RSA");
        verifying_key(certificate)
            .verify(PRESIGN, &signature)
            .expect("cada gemelo tiene que firmar con la clave de SU certificado");
        assert!(
            verifying_key(twin).verify(PRESIGN, &signature).is_err(),
            "la firma verifica contra el otro gemelo: se esta emparejando por etiqueta"
        );
    }
}

#[test]
fn each_of_two_certificates_sharing_a_label_comes_back_by_its_own_handle() {
    let found = certificates();
    let listed = ListedCertificates::new();

    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );

    let twins: Vec<(&String, &TokenCertificate)> = handles
        .iter()
        .zip(found.iter())
        .filter(|(_, certificate)| certificate.reference().label() == TWIN)
        .collect();
    assert_eq!(twins.len(), 2, "el token tenia que traer los dos gemelos");
    assert_ne!(twins[0].0, twins[1].0, "dos filas, dos asas");
    for (handle, certificate) in twins {
        assert_eq!(
            listed.get(handle).as_ref(),
            Some(certificate.reference()),
            "el asa tiene que llevar a SU certificado y no al primero con esa etiqueta"
        );
    }
}

#[test]
fn the_handle_of_a_real_certificate_carries_nothing_of_it() {
    let listed = ListedCertificates::new();

    let handles = listed.replace([reference(ACTIVE)]);

    let handle = &handles[0];
    assert_eq!(handle.len(), 32);
    assert!(handle.chars().all(|letter| letter.is_ascii_hexdigit()));
    for leak in ["softhsm", "libsofthsm2", TOKEN, ACTIVE, "/"] {
        assert!(!handle.contains(leak), "el asa «{handle}» lleva «{leak}»");
    }
}

#[test]
fn a_plain_pkcs11_module_is_a_card_store() {
    let store = certificate_labelled(ACTIVE).reference().store();

    assert_eq!(store.class(), StoreClass::Card);
}

fn signing_error(reference: &CertificateRef, pin: &str) -> TokenError {
    pkcs11::sign(reference, pin, PRESIGN).expect_err("esto tenia que fallar")
}

#[test]
fn a_wrong_pin_is_a_situation_and_carries_its_raw_ckr_apart() {
    let error = signing_error(&reference(ACTIVE), "0000");

    assert_eq!(error.situation(), Situation::IncorrectPin);
    assert_eq!(error.ckr(), Some("CKR_PIN_INCORRECT"));
    assert!(error.detail().contains("CKR_PIN_INCORRECT"));
}

#[test]
fn a_token_that_is_not_there_is_told_apart_from_a_wrong_pin() {
    let absent = CertificateRef::new(module(), "no-existe-este-token", ACTIVE, vec![0x01]);
    let error = signing_error(&absent, PIN);

    assert_eq!(error.situation(), Situation::TokenAbsent);
    assert!(!error.detail().is_empty());
}

#[test]
fn a_module_that_is_not_there_is_not_a_token_error() {
    let error = pkcs11::list_certificates(Path::new("/usr/lib/no-hay-ningun-modulo-aqui.so"))
        .expect_err("un modulo inexistente no puede cargarse");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
    assert_eq!(error.ckr(), None, "esto no viene de ningun CKR_*");
    assert!(error.detail().contains("no-hay-ningun-modulo-aqui.so"));
}

#[test]
fn a_store_that_cannot_be_loaded_does_not_hide_the_ones_that_can() {
    let stores = vec![
        pkcs11::Store::module("/usr/lib/no-hay-ningun-modulo-aqui.so"),
        pkcs11::Store::module(module()),
    ];

    let found = pkcs11::list_certificates_across(&stores)
        .expect("el almacen que si carga tiene que seguir contando");

    assert!(
        found
            .iter()
            .any(|certificate| certificate.reference().label() == ACTIVE),
        "el certificado activo del token tenia que salir pese al almacen roto"
    );
}

#[test]
fn tells_the_failure_apart_from_an_empty_list_when_no_store_loads() {
    let stores = vec![
        pkcs11::Store::module("/usr/lib/no-hay-ningun-modulo-aqui.so"),
        pkcs11::Store::module("/usr/lib/tampoco-hay-este-otro.so"),
    ];

    let error = pkcs11::list_certificates_across(&stores)
        .expect_err("sin ningun almacen cargado no hay lista que devolver");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
}

#[test]
fn having_nowhere_to_look_is_a_failure_and_not_an_empty_list() {
    let error =
        pkcs11::list_certificates_across(&[]).expect_err("sin almacenes no hay donde buscar");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
    assert!(!error.detail().is_empty());
}

#[test]
fn a_cka_id_that_is_not_in_the_token_says_so_instead_of_failing_generically() {
    let missing = CertificateRef::new(module(), TOKEN, "ETIQUETA-QUE-NO-EXISTE", vec![0xff]);
    let error = signing_error(&missing, PIN);

    assert_eq!(error.situation(), Situation::CertificateNotFound);
}

#[test]
fn a_reference_without_a_cka_id_refuses_to_sign_instead_of_guessing_by_label() {
    let remembered = CertificateRef::new(module(), TOKEN, ACTIVE, None);

    let error = signing_error(&remembered, PIN);

    assert_eq!(error.situation(), Situation::CertificateNotFound);
    assert!(error.detail().contains("CKA_ID"), "{}", error.detail());
}
