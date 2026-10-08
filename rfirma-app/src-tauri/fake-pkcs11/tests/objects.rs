//! Lo que enseña la tarjeta falsa con y sin login, y que firma de verdad con la clave de su certificado.

mod common;

use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass, ObjectHandle};
use cryptoki::session::{Session, UserType};
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Public};
use openssl::sign::Verifier;
use openssl::x509::X509;

use common::{process, session};

fn labels_of(session: &cryptoki::session::Session, class: ObjectClass) -> Vec<String> {
    let mut labels: Vec<String> = session
        .find_objects(&[Attribute::Class(class)])
        .expect("C_FindObjects deberia responder")
        .into_iter()
        .map(|object| {
            let attributes = session
                .get_attributes(object, &[AttributeType::Label])
                .expect("el objeto deberia tener etiqueta");
            match attributes.as_slice() {
                [Attribute::Label(bytes)] => String::from_utf8(bytes.clone()).unwrap(),
                other => panic!("se esperaba una etiqueta, llegó {other:?}"),
            }
        })
        .collect();
    labels.sort();
    labels
}

#[test]
fn without_login_shows_the_three_certificates_and_no_private_key() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let session = session(&context, slot);

    assert_eq!(
        labels_of(&session, ObjectClass::CERTIFICATE),
        [
            "CertAutenticacion",
            "CertCAIntermediaDGP",
            "CertFirmaDigital"
        ]
    );
    assert!(labels_of(&session, ObjectClass::PRIVATE_KEY).is_empty());
}

#[test]
fn after_login_shows_the_authentication_and_signing_keys() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let session = session(&context, slot);

    session
        .login(UserType::User, Some(&AuthPin::new(FakeCard::PIN.into())))
        .unwrap();

    assert_eq!(
        labels_of(&session, ObjectClass::PRIVATE_KEY),
        ["KprivAutenticacion", "KprivFirmaDigital"]
    );
}

fn find_one(session: &Session, class: ObjectClass, label: &str) -> ObjectHandle {
    let found = session
        .find_objects(&[Attribute::Class(class), Attribute::Label(label.into())])
        .expect("C_FindObjects deberia responder");
    assert_eq!(found.len(), 1, "deberia haber un solo {label}");
    found[0]
}

fn public_key_of(session: &Session, certificate: ObjectHandle) -> PKey<Public> {
    let attributes = session
        .get_attributes(certificate, &[AttributeType::Value])
        .expect("el certificado deberia tener valor");
    match attributes.as_slice() {
        [Attribute::Value(der)] => X509::from_der(der).unwrap().public_key().unwrap(),
        other => panic!("se esperaba el DER del certificado, llegó {other:?}"),
    }
}

#[test]
fn the_signing_key_signs_what_its_certificate_verifies() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);
    let session = session(&context, slot);
    session
        .login(UserType::User, Some(&AuthPin::new(FakeCard::PIN.into())))
        .unwrap();
    let data = b"documento que se firma";

    let key = find_one(&session, ObjectClass::PRIVATE_KEY, "KprivFirmaDigital");
    let signature = session
        .sign(&Mechanism::Sha256RsaPkcs, key, data)
        .expect("C_Sign deberia firmar con la clave de firma");

    let certificate = find_one(&session, ObjectClass::CERTIFICATE, "CertFirmaDigital");
    let public_key = public_key_of(&session, certificate);
    let mut verifier = Verifier::new(MessageDigest::sha256(), &public_key).unwrap();
    verifier.update(data).unwrap();
    assert_eq!(signature.len(), 256);
    assert!(verifier.verify(&signature).unwrap());
}
