//! Pruebas de integración del backend contra el módulo PKCS#11 SoftHSM (ADR-0014).

#[path = "pkcs11_token/support.rs"]
mod support;

use std::path::Path;

use openssl::hash::MessageDigest;
use openssl::rsa::Padding;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates::ListedCertificates;
use rfirma_lib::identity::domain::algorithm::SignatureAlgorithm;
use rfirma_lib::identity::domain::certificate::{
    CertificateRef, CertificateStatus, TokenCertificate,
};
use rfirma_lib::identity::domain::error::Situation;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::store::StoreClass;
use rsa::pkcs1v15::Signature;
use rsa::signature::Verifier;
use sha2::{Digest, Sha256};

use support::{
    certificate_labelled, certificate_with_cka_id, certificates, epoch, module, openssl_verifies,
    openssl_verifies_for, reference, sign_with_bare_rsa_pkcs, signing_error, verifying_key, ACTIVE,
    ACTIVE_EC, EXPIRED, PIN, PRESIGN, REVOKED, TOKEN, TWIN, TWIN_OF_THE_ACTIVE_KEY,
    TWIN_OF_THE_EXPIRED_KEY,
};

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
fn the_pin_dialog_names_the_holder_and_never_the_label_of_the_object() {
    let certificate = certificate_labelled(ACTIVE);

    let holder = rfirma_lib::identity::domain::holder::prompted_holder_of(certificate.der())
        .expect("el DER de pruebas deberia dar titular");

    let (name, id_number) =
        rfirma_lib::identity::domain::holder::holder_of(certificate.subject().as_deref());
    assert_eq!(holder.name, name);
    assert_eq!(holder.id_number, id_number);
    assert_ne!(
        holder.name,
        certificate.reference().label(),
        "el dialogo estaria enseñando el CKA_LABEL en lugar del titular"
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
        6,
        "los tokens de pruebas tienen seis certificados con clave: cinco de RSA y uno de curva eliptica"
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

#[test]
fn signing_produces_a_signature_that_the_certificate_public_key_verifies() {
    let certificate = certificate_labelled(ACTIVE);
    let raw = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha256Rsa,
        PRESIGN,
    )
    .expect("la firma deberia salir");

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

    let ours = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha256Rsa,
        PRESIGN,
    )
    .expect("la firma deberia salir");
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

#[test]
fn each_rsa_digest_signs_and_openssl_verifies_it_with_the_one_it_names() {
    for (algorithm, digest) in [
        (SignatureAlgorithm::Sha256Rsa, MessageDigest::sha256()),
        (SignatureAlgorithm::Sha384Rsa, MessageDigest::sha384()),
        (SignatureAlgorithm::Sha512Rsa, MessageDigest::sha512()),
    ] {
        let signature = pkcs11::sign(&reference(ACTIVE), PIN, algorithm, PRESIGN)
            .unwrap_or_else(|error| panic!("{} deberia firmar: {error}", algorithm.name()));

        assert!(
            openssl_verifies(digest, Padding::PKCS1, &signature),
            "{} no verifica con su propio resumen",
            algorithm.name()
        );
    }
}

#[test]
fn a_signature_does_not_verify_under_a_digest_that_is_not_the_one_it_was_made_with() {
    let signature = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha384Rsa,
        PRESIGN,
    )
    .expect("SHA384withRSA deberia firmar");

    assert!(!openssl_verifies(
        MessageDigest::sha512(),
        Padding::PKCS1,
        &signature
    ));
}

#[test]
fn the_pss_form_signs_and_openssl_verifies_it_as_pss_and_not_as_pkcs1() {
    let signature = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha256RsaPss,
        PRESIGN,
    )
    .expect("SoftHSM ofrece CKM_SHA256_RSA_PKCS_PSS");

    assert!(openssl_verifies(
        MessageDigest::sha256(),
        Padding::PKCS1_PSS,
        &signature
    ));
    assert!(!openssl_verifies(
        MessageDigest::sha256(),
        Padding::PKCS1,
        &signature
    ));
}

#[test]
fn an_ec_algorithm_over_an_rsa_key_is_refused_naming_the_key_and_not_a_ckr() {
    let error = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha256Ecdsa,
        PRESIGN,
    )
    .expect_err("la clave del certificado activo es RSA");

    assert_eq!(error.situation(), Situation::MechanismNotOffered);
    assert_eq!(error.ckr(), None, "esto no viene de ningun CKR_*");
    assert!(
        error.detail().contains("SHA256withECDSA") && error.detail().contains("RSA"),
        "{}",
        error.detail()
    );
}

#[test]
fn an_rsa_algorithm_over_an_ec_key_is_refused_the_same_way() {
    let error = pkcs11::sign(
        &reference(ACTIVE_EC),
        PIN,
        SignatureAlgorithm::Sha256Rsa,
        PRESIGN,
    )
    .expect_err("la clave del certificado de curva eliptica no es RSA");

    assert_eq!(error.situation(), Situation::MechanismNotOffered);
    assert!(
        error.detail().contains("SHA256withRSA") && error.detail().contains("EC"),
        "{}",
        error.detail()
    );
}

#[test]
fn each_ecdsa_digest_signs_and_openssl_verifies_it_with_the_one_it_names() {
    for (algorithm, digest) in [
        (SignatureAlgorithm::Sha256Ecdsa, MessageDigest::sha256()),
        (SignatureAlgorithm::Sha384Ecdsa, MessageDigest::sha384()),
        (SignatureAlgorithm::Sha512Ecdsa, MessageDigest::sha512()),
    ] {
        let signature = pkcs11::sign(&reference(ACTIVE_EC), PIN, algorithm, PRESIGN)
            .unwrap_or_else(|error| panic!("{} deberia firmar: {error}", algorithm.name()));

        assert!(
            openssl_verifies_for(ACTIVE_EC, digest, None, &signature),
            "{} no verifica con su propio resumen",
            algorithm.name()
        );
    }
}

#[test]
fn an_ecdsa_signature_comes_back_in_der_and_not_as_the_raw_r_and_s_of_pkcs11() {
    let signature = pkcs11::sign(
        &reference(ACTIVE_EC),
        PIN,
        SignatureAlgorithm::Sha256Ecdsa,
        PRESIGN,
    )
    .expect("SHA256withECDSA deberia firmar");

    assert_eq!(
        signature[0], 0x30,
        "una firma ECDSA para CMS empieza por el SEQUENCE de r y s"
    );
    assert_ne!(
        signature.len(),
        64,
        "el r||s crudo de una curva P-256 ocupa 64 bytes y no vale como SignatureValue"
    );
}

#[test]
fn an_ecdsa_signature_does_not_verify_under_a_digest_that_is_not_its_own() {
    let signature = pkcs11::sign(
        &reference(ACTIVE_EC),
        PIN,
        SignatureAlgorithm::Sha384Ecdsa,
        PRESIGN,
    )
    .expect("SHA384withECDSA deberia firmar");

    assert!(!openssl_verifies_for(
        ACTIVE_EC,
        MessageDigest::sha512(),
        None,
        &signature
    ));
}

#[test]
fn signing_the_same_bytes_twice_gives_the_same_signature() {
    let once = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha256Rsa,
        PRESIGN,
    )
    .expect("firma");
    let twice = pkcs11::sign(
        &reference(ACTIVE),
        PIN,
        SignatureAlgorithm::Sha256Rsa,
        PRESIGN,
    )
    .expect("firma");

    assert_eq!(once, twice);
}

#[test]
fn two_certificates_sharing_a_label_each_sign_with_their_own_key() {
    let one = certificate_with_cka_id(TWIN_OF_THE_ACTIVE_KEY);
    let other = certificate_with_cka_id(TWIN_OF_THE_EXPIRED_KEY);

    assert_eq!(one.reference().label(), TWIN);
    assert_eq!(other.reference().label(), TWIN);
    assert_ne!(one.reference().cka_id(), other.reference().cka_id());

    let signed_by_one = pkcs11::sign(one.reference(), PIN, SignatureAlgorithm::Sha256Rsa, PRESIGN)
        .expect("firma del primero");
    let signed_by_other = pkcs11::sign(
        other.reference(),
        PIN,
        SignatureAlgorithm::Sha256Rsa,
        PRESIGN,
    )
    .expect("firma del segundo");

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

#[test]
fn a_wrong_pin_is_a_situation_and_carries_its_raw_ckr_apart() {
    let error = signing_error(&reference(ACTIVE), "0000");

    assert_eq!(error.situation(), Situation::IncorrectPin);
    assert_eq!(error.ckr(), Some("CKR_PIN_INCORRECT"));
    assert!(error.detail().contains("CKR_PIN_INCORRECT"));
}

#[test]
fn the_token_accepts_its_pin_without_signing_anything() {
    let secret = ProtectedSecret::from_str(PIN);

    assert_eq!(
        pkcs11::accepts_the_secret(&reference(ACTIVE), &secret),
        Ok(())
    );
}

#[test]
fn a_wrong_pin_is_refused_by_the_token_as_an_incorrect_pin() {
    let secret = ProtectedSecret::from_str("0000");

    let error = pkcs11::accepts_the_secret(&reference(ACTIVE), &secret)
        .expect_err("el token no acepta un PIN que no es el suyo");

    assert_eq!(error.situation(), Situation::IncorrectPin);
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
        rfirma_lib::identity::domain::store::Store::module("/usr/lib/no-hay-ningun-modulo-aqui.so"),
        rfirma_lib::identity::domain::store::Store::module(module()),
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
        rfirma_lib::identity::domain::store::Store::module("/usr/lib/no-hay-ningun-modulo-aqui.so"),
        rfirma_lib::identity::domain::store::Store::module("/usr/lib/tampoco-hay-este-otro.so"),
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
