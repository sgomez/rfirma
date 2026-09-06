use super::{store_name, CertificateView, SecretView};
use crate::commands::views::StatusView;
use crate::pkcs11::{StoreClass, StoreSecret};

#[test]
fn the_secret_crosses_as_one_of_three_kinds_and_never_as_a_string() {
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::NotNeeded)).expect("serializa"),
        r#"{"kind":"notNeeded"}"#
    );
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::TypedOnScreen {
            attempts_left: None
        }))
        .expect("serializa"),
        r#"{"kind":"typedOnScreen","attemptsLeft":null}"#
    );
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::TypedOnTheReaderKeypad))
            .expect("serializa"),
        r#"{"kind":"typedOnTheReaderKeypad"}"#
    );
}

#[test]
fn a_certificate_crosses_without_its_der_and_without_its_module() {
    let view = CertificateView {
        id: "0123456789abcdef0123456789abcdef".to_owned(),
        label: "ETIQUETA".to_owned(),
        holder_name: "Ada Lovelace Byron".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        issuer: "FNMT-RCM".to_owned(),
        store: store_name(StoreClass::Firefox).to_owned(),
        status: StatusView::Valid {
            not_after: 1_900_000_000,
        },
        remembered: false,
    };
    let json = serde_json::to_string(&view).expect("serializa");

    assert!(json.contains(r#""holderName":"Ada Lovelace Byron""#));
    assert!(!json.contains(r#""der""#), "el DER no sale: {json}");
    assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
}

#[test]
fn the_store_crosses_as_a_class_and_never_as_a_path() {
    let names = [
        store_name(StoreClass::Card),
        store_name(StoreClass::Firefox),
        store_name(StoreClass::Chrome),
        store_name(StoreClass::Nssdb),
        store_name(StoreClass::Installed),
    ];

    assert_eq!(names, ["card", "firefox", "chrome", "nssdb", "installed"]);
    for name in names {
        assert!(!name.contains('/'), "«{name}» parece una ruta");
        assert!(
            name.chars().all(|letter| letter.is_ascii_lowercase()),
            "«{name}» no es una clase en ingles"
        );
    }
}
