use super::*;

const COMPOUND_AND_HYPHENATED: [&str; 9] = [
    "SHA256withRSA",
    "SHA384withRSA",
    "SHA512withRSA",
    "SHA256withECDSA",
    "SHA384withECDSA",
    "SHA512withECDSA",
    "SHA-256",
    "SHA-384",
    "SHA-512",
];

fn xml_lote(algorithm: &str) -> String {
    format!("<signbatch algorithm=\"{algorithm}\" stoponerror=\"true\"><singlesign id=\"001\"/></signbatch>")
}

fn json_lote(algorithm: &str) -> String {
    format!("{{\"algorithm\":\"{algorithm}\",\"stoponerror\":false}}")
}

#[test]
fn the_legacy_xml_declares_its_algorithm_in_signbatch() {
    for algorithm in COMPOUND_AND_HYPHENATED {
        assert_eq!(
            batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes(), false),
            Ok(algorithm.to_owned())
        );
    }
}

#[test]
fn the_json_batch_declares_its_algorithm_in_the_root_object() {
    for algorithm in COMPOUND_AND_HYPHENATED {
        assert_eq!(
            batch_algorithm(BatchFormat::Json, json_lote(algorithm).as_bytes(), false),
            Ok(algorithm.to_owned())
        );
    }
}

#[test]
fn without_the_preference_sha1_is_read_as_the_refused_sha1_in_both_formats() {
    for algorithm in ["sha1", "SHA1", "SHA1withRSA", "SHA-1"] {
        assert!(matches!(
            batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes(), false),
            Err(HeaderRefusal::Sha1(_))
        ));
        assert!(matches!(
            batch_algorithm(BatchFormat::Json, json_lote(algorithm).as_bytes(), false),
            Err(HeaderRefusal::Sha1(_))
        ));
    }
}

#[test]
fn with_the_preference_sha1_is_admitted_as_the_site_wrote_it_in_both_formats() {
    for algorithm in ["sha1", "SHA1", "SHA1withRSA", "SHA-1"] {
        assert_eq!(
            batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes(), true),
            Ok(algorithm.to_owned())
        );
        assert_eq!(
            batch_algorithm(BatchFormat::Json, json_lote(algorithm).as_bytes(), true),
            Ok(algorithm.to_owned())
        );
    }
}

#[test]
fn only_a_header_with_sha1_asks_for_sha1() {
    assert!(batch_asks_for_sha1(
        BatchFormat::Xml,
        xml_lote("SHA1withRSA").as_bytes()
    ));
    assert!(batch_asks_for_sha1(
        BatchFormat::Json,
        json_lote("SHA-1").as_bytes()
    ));
    assert!(!batch_asks_for_sha1(
        BatchFormat::Json,
        json_lote("SHA256").as_bytes()
    ));
    assert!(!batch_asks_for_sha1(
        BatchFormat::Xml,
        json_lote("SHA1").as_bytes()
    ));
}

#[test]
fn an_algorithm_rfirma_cannot_produce_is_not_read() {
    for algorithm in ["RIPEMD160withRSA", "MD5"] {
        assert!(matches!(
            batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes(), false),
            Err(HeaderRefusal::Unreadable(_))
        ));
    }
}

#[test]
fn a_header_without_algorithm_is_not_read() {
    assert!(batch_algorithm(
        BatchFormat::Xml,
        b"<signbatch stoponerror=\"true\"/>",
        false
    )
    .is_err());
    assert!(batch_algorithm(BatchFormat::Json, b"{\"stoponerror\":true}", false).is_err());
}

#[test]
fn a_json_batch_read_as_the_legacy_xml_is_not_read() {
    assert!(batch_algorithm(BatchFormat::Xml, json_lote("SHA256").as_bytes(), false).is_err());
}
