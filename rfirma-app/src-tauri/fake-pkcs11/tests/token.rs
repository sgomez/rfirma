//! Lo que rFirma pregunta a la ranura antes de firmar: cómo se presenta la tarjeta, sus mecanismos y sus sesiones.

mod common;

use cryptoki::error::{Error, RvError};
use cryptoki::mechanism::{Mechanism, MechanismType};
use cryptoki::object::{Attribute, AttributeType, ObjectClass};
use cryptoki::session::{Session, UserType};
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;
use openssl::hash::{hash, MessageDigest};
use openssl::sign::Verifier;
use openssl::x509::X509;

use common::{process, session};

fn logged_in(session: &Session) {
    session
        .login(UserType::User, Some(&AuthPin::new(FakeCard::PIN.into())))
        .unwrap();
}

fn signing_key(session: &Session) -> cryptoki::object::ObjectHandle {
    session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(b"KprivFirmaDigital".to_vec()),
        ])
        .unwrap()[0]
}

fn signing_certificate(session: &Session) -> X509 {
    let handle = session
        .find_objects(&[
            Attribute::Class(ObjectClass::CERTIFICATE),
            Attribute::Label(b"CertFirmaDigital".to_vec()),
        ])
        .unwrap()[0];
    match session
        .get_attributes(handle, &[AttributeType::Value])
        .unwrap()
        .as_slice()
    {
        [Attribute::Value(der)] => X509::from_der(der).unwrap(),
        other => panic!("se esperaba el DER del certificado, llegó {other:?}"),
    }
}

#[test]
fn the_card_presents_itself_as_a_dnie_in_a_removable_reader() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);

    let token = context.get_token_info(slot).unwrap();
    assert_eq!(token.label().trim(), "DNI electrónico");
    assert!(token.login_required());
    assert!(token.token_initialized());
    assert!(!token.protected_authentication_path());
    let reader = context.get_slot_info(slot).unwrap();
    assert!(reader.token_present());
    assert!(reader.removable_device());
    assert_eq!(
        context
            .get_library_info()
            .unwrap()
            .cryptoki_version()
            .major(),
        2
    );
}

#[test]
fn the_slot_offers_rsa_signing_with_its_digest() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);

    let mechanisms = context.get_mechanism_list(slot).unwrap();
    assert!(mechanisms.contains(&MechanismType::SHA256_RSA_PKCS));
    assert!(mechanisms.contains(&MechanismType::RSA_PKCS));
    let info = context
        .get_mechanism_info(slot, MechanismType::SHA256_RSA_PKCS)
        .unwrap();
    assert!(info.sign());
    assert_eq!(info.max_key_size(), 2048);
}

#[test]
fn the_raw_mechanism_signs_a_digest_info() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let session = session(&context, slot);
    logged_in(&session);
    let data = b"documento que se firma";
    let sha256_prefix = [
        0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01,
        0x05, 0x00, 0x04, 0x20,
    ];
    let digest_info = [
        &sha256_prefix[..],
        &hash(MessageDigest::sha256(), data).unwrap(),
    ]
    .concat();

    let signature = session
        .sign(&Mechanism::RsaPkcs, signing_key(&session), &digest_info)
        .unwrap();

    let public_key = signing_certificate(&session).public_key().unwrap();
    let mut verifier = Verifier::new(MessageDigest::sha256(), &public_key).unwrap();
    verifier.update(data).unwrap();
    assert!(verifier.verify(&signature).unwrap());
}

#[test]
fn the_private_key_shows_its_modulus_and_never_its_value() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let session = session(&context, slot);
    logged_in(&session);

    let attributes = session
        .get_attributes(
            signing_key(&session),
            &[AttributeType::Modulus, AttributeType::Value],
        )
        .unwrap();

    let certificate_modulus = signing_certificate(&session)
        .public_key()
        .unwrap()
        .rsa()
        .unwrap()
        .n()
        .to_vec();
    assert_eq!(attributes, [Attribute::Modulus(certificate_modulus)]);
}

#[test]
fn logging_out_hides_the_keys_and_stops_the_signing() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let session = session(&context, slot);
    logged_in(&session);
    let key = signing_key(&session);

    session.logout().unwrap();

    let keys = session
        .find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)])
        .unwrap();
    assert!(keys.is_empty());
    let refused = session.sign(&Mechanism::Sha256RsaPkcs, key, b"datos");
    assert!(matches!(
        refused,
        Err(Error::Pkcs11(RvError::UserNotLoggedIn, _))
    ));
}

#[test]
fn a_login_reaches_every_session_until_the_last_one_closes() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let first = session(&context, slot);
    logged_in(&first);

    let second = session(&context, slot);
    assert!(!signing_key_is_hidden(&second));
    drop(first);
    drop(second);

    let third = session(&context, slot);
    assert!(signing_key_is_hidden(&third));
}

fn signing_key_is_hidden(session: &Session) -> bool {
    session
        .find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)])
        .unwrap()
        .is_empty()
}
