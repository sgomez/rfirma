use super::StatusView;
use crate::pkcs11::CertificateStatus;

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
