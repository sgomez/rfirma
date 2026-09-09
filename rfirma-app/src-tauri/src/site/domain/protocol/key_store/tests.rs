use super::{key_store_named_by, refuse_a_key_store_rfirma_does_not_open};
use crate::site::domain::protocol::codes::{Parameter, SafCode};
use crate::site::domain::protocol::refusal::RefusalSituation;
use crate::site::domain::protocol::url::AfirmaUrl;
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;

/// **Grada A**: se lee una URL y sale el almacén que nombró la sede. Sin token y sin almacén
/// que abrir.
fn a_selection(parameters: &str) -> AfirmaUrl {
    AfirmaUrl::parse(&format!("afirma://selectcert?op=selectcert&{parameters}"))
        .expect("es del protocolo")
}

fn ksb64(value: &str) -> AfirmaUrl {
    a_selection(&format!("ksb64={}", URL_SAFE.encode(value.as_bytes())))
}

#[test]
fn a_value_without_a_colon_names_a_store_and_no_library() {
    let named = key_store_named_by(&ksb64("MOZ_UNI")).expect("la sede nombra un almacen");

    assert_eq!(named.name(), "MOZ_UNI");
    assert_eq!(named.library(), None);
}

#[test]
fn what_comes_after_the_first_colon_is_the_library() {
    let named =
        key_store_named_by(&ksb64("PKCS11:/usr/lib/opensc-pkcs11.so")).expect("nombra almacen");

    assert_eq!(named.name(), "PKCS11");
    assert_eq!(named.library(), Some("/usr/lib/opensc-pkcs11.so"));
}

#[test]
fn the_legacy_parameter_wins_over_the_one_in_base64() {
    let url = a_selection(&format!(
        "keystore=MOZ_UNI&ksb64={}",
        URL_SAFE.encode(b"PKCS12")
    ));

    let named = key_store_named_by(&url).expect("la sede nombra un almacen");

    assert_eq!(named.name(), "MOZ_UNI");
    assert_eq!(named.declared_in(), Parameter::LegacyKeyStore);
}

#[test]
fn a_ksb64_that_is_not_base64_is_ignored() {
    let url = a_selection("ksb64=***");

    assert_eq!(key_store_named_by(&url), None);
    refuse_a_key_store_rfirma_does_not_open(&url).expect("un valor que no es Base64 no rechaza");
}

#[test]
fn a_lonely_colon_names_nothing() {
    assert_eq!(key_store_named_by(&ksb64(":")), None);
}

#[test]
fn without_the_two_parameters_there_is_no_store_named() {
    assert_eq!(key_store_named_by(&a_selection("idsession=8jAkPZ")), None);
}

#[test]
fn the_nss_stores_of_the_original_are_the_ones_rfirma_opens() {
    for store in ["SHARED_NSS", "MOZ_UNI", "moz_uni", " SHARED_NSS :"] {
        refuse_a_key_store_rfirma_does_not_open(&ksb64(store))
            .expect("es el almacen que rFirma abre");
    }
}

#[test]
fn a_store_the_original_does_not_recognise_is_ignored_as_the_original_does() {
    refuse_a_key_store_rfirma_does_not_open(&ksb64("EL_ALMACEN_DE_LA_SEDE"))
        .expect("el original cae en el de por omision");
}

#[test]
fn a_pkcs12_named_by_the_site_is_refused_with_a_code_of_the_catalogue() {
    let url = ksb64("PKCS12:/ruta/al/almacen.p12");

    let refusal = refuse_a_key_store_rfirma_does_not_open(&url).expect_err("rFirma no lo abre");

    assert_eq!(refusal.code(), SafCode::CannotFindKeystore);
    assert_eq!(refusal.blame(), Some(Parameter::KeyStore));
    assert_eq!(refusal.situation(), RefusalSituation::UnsupportedKeyStore);
}

#[test]
fn the_stores_of_the_original_that_rfirma_does_not_open_are_refused_one_by_one() {
    for store in [
        "WINDOWS",
        "APPLE",
        "PKCS12",
        "JAVA",
        "PKCS11",
        "SINGLE",
        "JCEKS",
        "JAVACE",
        "TEMD",
        "WINADDRESSBOOK",
        "WINCA",
        "CERES",
        "DNIEJAVA",
        "KNOWN_SMARTCARDS",
        "SMARTCAFE",
        "CERES_430",
    ] {
        let refusal = refuse_a_key_store_rfirma_does_not_open(&ksb64(store))
            .expect_err("es un almacen que rFirma no abre");

        assert_eq!(refusal.code(), SafCode::CannotFindKeystore);
    }
}

#[test]
fn a_legacy_keystore_is_refused_naming_its_own_parameter() {
    let refusal = refuse_a_key_store_rfirma_does_not_open(&a_selection("keystore=DNIEJAVA"))
        .expect_err("las tarjetas se quedan fuera");

    assert_eq!(refusal.blame(), Some(Parameter::LegacyKeyStore));
}

#[test]
fn a_library_is_refused_even_when_the_store_is_the_one_rfirma_opens() {
    let refusal = refuse_a_key_store_rfirma_does_not_open(&ksb64("MOZ_UNI:/usr/lib/libnss3.so"))
        .expect_err("rFirma no carga la biblioteca que le nombren");

    assert_eq!(refusal.code(), SafCode::CannotFindKeystore);
}
