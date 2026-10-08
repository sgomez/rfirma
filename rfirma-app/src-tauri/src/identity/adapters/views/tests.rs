use super::{store_name, CertificateView, SecretView, StatusView};
use crate::identity::domain::certificate::{CertificateStatus, ListedCertificate};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::StoreClass;

#[test]
fn the_secret_crosses_as_one_of_three_kinds_and_never_as_a_string() {
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::NotNeeded)).expect("serializa"),
        r#"{"kind":"notNeeded"}"#
    );
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::TypedOnScreen)).expect("serializa"),
        r#"{"kind":"typedOnScreen"}"#
    );
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::TypedOnTheReaderKeypad))
            .expect("serializa"),
        r#"{"kind":"typedOnTheReaderKeypad"}"#
    );
}

#[test]
fn a_certificate_crosses_without_its_der_and_without_its_module() {
    let view = a_view();
    let json = serde_json::to_string(&view).expect("serializa");

    assert!(json.contains(r#""holderName":"LOVELACE BYRON ADA""#));
    assert!(json.contains(r#""givenName":"ADA""#));
    assert!(json.contains(r#""surname":"LOVELACE BYRON""#));
    assert!(!json.contains(r#""der""#), "el DER no sale: {json}");
    assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
}

#[test]
fn a_representative_certificate_crosses_with_its_entity_name() {
    let mut view = a_view();
    view.entity_name = Some("ENTIDAD DE PRUEBAS".to_owned());

    let json = serde_json::to_string(&view).expect("serializa");

    assert!(json.contains(r#""entityName":"ENTIDAD DE PRUEBAS""#));
}

fn a_view() -> CertificateView {
    CertificateView {
        id: "0123456789abcdef0123456789abcdef".to_owned(),
        label: "ETIQUETA".to_owned(),
        holder_name: "LOVELACE BYRON ADA".to_owned(),
        stamped_signer: "LOVELACE BYRON ADA".to_owned(),
        given_name: "ADA".to_owned(),
        surname: "LOVELACE BYRON".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        organization_identifier: None,
        entity_name: None,
        issuer: "FNMT-RCM".to_owned(),
        certificate_serial_number: "1234567890".to_owned(),
        stores: vec![store_name(StoreClass::Firefox).to_owned()],
        status: StatusView::Valid {
            not_after: 1_900_000_000,
        },
        remembered: false,
    }
}

fn a_row_in(stores: Vec<StoreClass>, from_a_dnie: bool) -> ListedCertificate {
    ListedCertificate {
        id: "0123456789abcdef0123456789abcdef".to_owned(),
        label: "ETIQUETA".to_owned(),
        holder_name: String::new(),
        stamped_signer: String::new(),
        given_name: String::new(),
        surname: String::new(),
        id_number: String::new(),
        organization_identifier: None,
        entity_name: None,
        issuer: String::new(),
        certificate_serial_number: String::new(),
        store: stores[0],
        stores,
        from_a_dnie,
        status: CertificateStatus::Valid { not_after: 0 },
        remembered: false,
    }
}

#[test]
fn a_row_crosses_every_store_it_is_in_but_not_the_one_behind_its_handle() {
    let view = CertificateView::from(a_row_in(vec![StoreClass::Card, StoreClass::Firefox], false));
    let json = serde_json::to_string(&view).expect("serializa");

    assert!(!json.contains(r#""store":"#), "{json}");
    assert!(json.contains(r#""stores":["card","firefox"]"#), "{json}");
}

#[test]
fn a_dnie_row_carries_the_dnie_chip_in_place_of_the_card_one() {
    let view = CertificateView::from(a_row_in(vec![StoreClass::Card], true));

    assert_eq!(view.stores, ["dnie"]);
}

#[test]
fn a_dnie_certificate_copied_into_another_store_keeps_that_store_label() {
    let view = CertificateView::from(a_row_in(vec![StoreClass::Firefox], true));

    assert_eq!(view.stores, ["firefox"]);
}

#[test]
fn the_store_crosses_as_a_class_and_never_as_a_path() {
    let names = [
        store_name(StoreClass::Card),
        store_name(StoreClass::Firefox),
        store_name(StoreClass::Chrome),
        store_name(StoreClass::Nssdb),
        store_name(StoreClass::Installed),
        store_name(StoreClass::Windows),
    ];

    assert_eq!(
        names,
        ["card", "firefox", "chrome", "nssdb", "installed", "windows"]
    );
    for name in names {
        assert!(!name.contains('/'), "«{name}» parece una ruta");
        assert!(
            name.chars().all(|letter| letter.is_ascii_lowercase()),
            "«{name}» no es una clase en ingles"
        );
    }
}

#[test]
fn the_status_crosses_with_its_payload() {
    let not_yet = StatusView::from(CertificateStatus::NotYetValid { not_before: 42 });
    let unreadable = StatusView::from(CertificateStatus::Unreadable {
        detail: "PEM error".to_owned(),
    });

    assert_eq!(
        serde_json::to_string(&not_yet).expect("serializa"),
        r#"{"kind":"notYetValid","notBefore":42}"#
    );
    assert_eq!(
        serde_json::to_string(&unreadable).expect("serializa"),
        r#"{"kind":"unreadable","detail":"PEM error"}"#
    );
}
