use super::*;

fn an_expired_signature() -> DocumentSignature {
    DocumentSignature {
        name: "LOVELACE BYRON ADA".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        organization_identifier: None,
        organization_name: None,
        issuer: "AC FNMT Usuarios".to_owned(),
        certificate_serial_number: "1234567890".to_owned(),
        signing_time: None,
        status: None,
        reason: None,
        validity: Validity::Expired,
        validity_reason: Some(ValidityReason::CertificateExpired {
            date: "2020-03-05T12:00:00Z".to_owned(),
            holder: None,
        }),
        signing_date: None,
        closes_document: false,
        countersignatures: Vec::new(),
    }
}

#[test]
fn the_report_view_carries_each_validity_reason_and_the_findings() {
    let report = DocumentSignatures::new(vec![an_expired_signature()], false)
        .with_findings(vec![DocumentFinding::ContentAddedOnTop]);

    let json = serde_json::to_value(PreviousSignaturesReportView::from(report))
        .expect("la vista serializa");

    assert_eq!(json["findings"], serde_json::json!(["contentAddedOnTop"]));
    assert_eq!(json["signatures"][0]["validity"], "expired");
    assert_eq!(
        json["signatures"][0]["validityReason"],
        serde_json::json!({
            "kind": "certificateExpired",
            "date": "2020-03-05T12:00:00Z",
            "holder": null
        })
    );
}
