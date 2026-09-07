use super::*;

/// Capturado con `TriphaseData.toString()` de 1.9.2 (`~/.m2`), con `NEED_PRE`.
const XML_WITH_NEED_PRE: &str = "<xml>
 <firmas format=\"PAdES\">
  <firma Id=\"001\">
   <param n=\"NEED_PRE\">true</param>
   <param n=\"PRE\">MYICXDAYBgkqhkiG9w0BA=</param>
   <param n=\"NEED_DATA\">true</param>
  </firma>
 </firmas>
</xml>";

fn a_sign(id: &str, params: &[(&str, &str)]) -> TriSign {
    TriSign::new(
        Some(id.to_owned()),
        None,
        params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    )
}

#[test]
fn the_xml_fixture_with_need_pre_round_trips_byte_for_byte() {
    let data =
        TriphaseData::parse_xml(XML_WITH_NEED_PRE.as_bytes()).expect("es el XML del original");

    assert_eq!(data.format(), Some("PAdES"));
    assert_eq!(data.signs().len(), 1);
    assert_eq!(data.signs()[0].id(), Some("001"));
    assert_eq!(data.signs()[0].param("NEED_PRE"), Some("true"));
    assert_eq!(data.signs()[0].param("PRE"), Some("MYICXDAYBgkqhkiG9w0BA="));
    assert_eq!(data.signs()[0].param("NEED_DATA"), Some("true"));

    assert_eq!(data.to_xml(), XML_WITH_NEED_PRE);
}

#[test]
fn a_constructed_xml_without_need_pre_round_trips_too() {
    let data = TriphaseData::new(
        Some("PAdES".to_owned()),
        vec![a_sign("002", &[("PRE", "QUJD"), ("NEED_DATA", "true")])],
    );

    let xml = data.to_xml();
    let parsed = TriphaseData::parse_xml(xml.as_bytes()).expect("es el XML que se acaba de emitir");

    assert_eq!(parsed.to_xml(), xml);
    assert_eq!(parsed.signs()[0].param("PRE"), Some("QUJD"));
}

#[test]
fn xml_without_the_firmas_node_is_rejected() {
    let result = TriphaseData::parse_xml(b"<xml><otro/></xml>");

    assert!(result.is_err());
}

#[test]
fn the_json_fixture_round_trips_byte_for_byte() {
    let data = TriphaseData::new(
        Some("PAdES".to_owned()),
        vec![a_sign(
            "001",
            &[("PRE", "MYICXDAYBgkqhkiG9w0BA="), ("NEED_PRE", "true")],
        )],
    );
    let json = data.to_json();

    let parsed =
        TriphaseData::parse_json(json.as_bytes()).expect("es el JSON que se acaba de emitir");

    assert_eq!(parsed.to_json(), json);
    assert_eq!(parsed.format(), Some("PAdES"));
    assert_eq!(parsed.signs()[0].id(), Some("001"));
    assert_eq!(
        parsed.signs()[0].param("PRE"),
        Some("MYICXDAYBgkqhkiG9w0BA=")
    );
}

#[test]
fn the_nested_signs_signinfo_json_shape_is_accepted_too() {
    let nested = "{\"format\":\"PAdES\",\"signs\":[{\"signinfo\":[{\"id\":\"001\",\"params\":{\"NEED_PRE\":\"true\",\"PRE\":\"MYICXDAYBgkqhkiG9w0BA=\"}}]}]}";

    let parsed =
        TriphaseData::parse_json(nested.as_bytes()).expect("es la forma anidada del original");

    assert_eq!(parsed.format(), Some("PAdES"));
    assert_eq!(parsed.signs().len(), 1);
    assert_eq!(parsed.signs()[0].id(), Some("001"));
    assert_eq!(parsed.signs()[0].param("NEED_PRE"), Some("true"));
}

#[test]
fn json_without_signinfo_nor_signs_is_rejected() {
    let result = TriphaseData::parse_json(b"{\"format\":\"PAdES\"}");

    assert!(result.is_err());
}

#[test]
fn applying_pk1_adds_pk1_and_deletes_pre_without_need_pre() {
    let data = TriphaseData::new(
        Some("PAdES".to_owned()),
        vec![a_sign("002", &[("PRE", "QUJD"), ("NEED_DATA", "true")])],
    );

    let signed = apply_pk1(data, |bytes| {
        assert_eq!(bytes, b"ABC");
        b"firma-pkcs1".to_vec()
    })
    .expect("hay PRE que firmar");

    let sign = &signed.signs()[0];
    assert_eq!(sign.param("PRE"), None);
    assert_eq!(
        sign.param("PK1"),
        Some(STANDARD.encode(b"firma-pkcs1").as_str())
    );
    assert_eq!(sign.param("NEED_DATA"), Some("true"));
}

#[test]
fn applying_pk1_keeps_pre_when_need_pre_is_true() {
    let data = TriphaseData::new(
        Some("PAdES".to_owned()),
        vec![a_sign("001", &[("NEED_PRE", "true"), ("PRE", "QUJD")])],
    );

    let signed = apply_pk1(data, |_| b"firma-pkcs1".to_vec()).expect("hay PRE que firmar");

    let sign = &signed.signs()[0];
    assert_eq!(sign.param("PRE"), Some("QUJD"));
    assert_eq!(
        sign.param("PK1"),
        Some(STANDARD.encode(b"firma-pkcs1").as_str())
    );
}

#[test]
fn applying_pk1_without_a_pre_names_the_sign() {
    let data = TriphaseData::new(None, vec![a_sign("003", &[])]);

    let result = apply_pk1(data, |_| b"nunca-se-llama".to_vec());

    assert!(result.is_err());
}
