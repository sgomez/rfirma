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
            batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes()),
            Ok(algorithm.to_owned())
        );
    }
}

#[test]
fn the_json_batch_declares_its_algorithm_in_the_root_object() {
    for algorithm in COMPOUND_AND_HYPHENATED {
        assert_eq!(
            batch_algorithm(BatchFormat::Json, json_lote(algorithm).as_bytes()),
            Ok(algorithm.to_owned())
        );
    }
}

/// Hay sedes en producción que declaran así su lote (ADR-0023).
#[test]
fn sha1_is_read_like_the_original_reads_it() {
    for algorithm in ["sha1", "SHA1", "SHA1withRSA", "SHA-1"] {
        assert!(batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes()).is_ok());
        assert!(batch_algorithm(BatchFormat::Json, json_lote(algorithm).as_bytes()).is_ok());
    }
}

#[test]
fn an_algorithm_rfirma_cannot_produce_is_not_read() {
    for algorithm in ["RIPEMD160withRSA", "MD5"] {
        assert!(batch_algorithm(BatchFormat::Xml, xml_lote(algorithm).as_bytes()).is_err());
    }
}

#[test]
fn a_header_without_algorithm_is_not_read() {
    assert!(batch_algorithm(BatchFormat::Xml, b"<signbatch stoponerror=\"true\"/>").is_err());
    assert!(batch_algorithm(BatchFormat::Json, b"{\"stoponerror\":true}").is_err());
}

#[test]
fn a_json_batch_read_as_the_legacy_xml_is_not_read() {
    assert!(batch_algorithm(BatchFormat::Xml, json_lote("SHA256").as_bytes()).is_err());
}
