use super::super::*;
use super::fixtures::{
    a_batch, an_operation, dat, gzipped, json_lote, properties, read_downloading, read_operation,
    refusal_of, url_encoded, xml_lote, ADownload, A_REMOTE_DOCUMENT,
};

#[test]
fn a_literal_local_batch_is_forwarded_in_base64() {
    let plain = json_lote("SHA256", true);
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&jsonbatch=true&dat={}",
        url_encoded(&plain)
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert_eq!(request.lote(), plain.as_bytes());
    assert_eq!(
        request.lote_base64(),
        base64::engine::general_purpose::STANDARD.encode(plain.as_bytes())
    );
}

#[test]
fn the_batch_reads_headless_from_its_properties_too() {
    let url = a_batch(&format!(
        "&properties={}&dat={}",
        properties("headless=true\n"),
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se lee el lote") else {
        panic!("es un lote");
    };

    assert!(request.is_headless());
}

#[test]
fn a_batch_reads_its_two_servlets_and_the_algorithm_of_its_xml_lote() {
    let lote = xml_lote("SHA256", true);
    let url = a_batch(&format!("&dat={}", dat(lote.as_bytes())));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert_eq!(
        request.presigner_url(),
        Some("https://presigner.example/pre")
    );
    assert_eq!(
        request.postsigner_url(),
        Some("https://postsigner.example/post")
    );
    assert!(!request.is_local());
    assert!(!request.is_json());
    assert_eq!(request.algorithm(), "SHA256");
    assert!(request.stops_on_error());
}

#[test]
fn a_batch_with_jsonbatch_reads_the_algorithm_of_its_json_lote() {
    let lote = json_lote("SHA512", false);
    let url = a_batch(&format!("&jsonbatch=true&dat={}", dat(lote.as_bytes())));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert!(request.is_json());
    assert_eq!(request.algorithm(), "SHA512");
    assert!(!request.stops_on_error());
}

#[test]
fn a_batch_in_legacy_xml_reads_compound_and_hyphenated_algorithm_names() {
    for algorithm in [
        "SHA256withRSA",
        "SHA384withRSA",
        "SHA512withRSA",
        "SHA256withECDSA",
        "SHA384withECDSA",
        "SHA512withECDSA",
        "SHA-256",
        "SHA-384",
        "SHA-512",
    ] {
        let lote = xml_lote(algorithm, true);
        let url = a_batch(&format!("&dat={}", dat(lote.as_bytes())));

        let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
            panic!("es un lote: {algorithm}");
        };
        assert!(!request.is_json());
        assert!(!request.is_local());
        assert_eq!(request.algorithm(), algorithm);
    }
}

#[test]
fn a_batch_in_json_reads_compound_and_hyphenated_algorithm_names() {
    for algorithm in [
        "SHA256withRSA",
        "SHA384withRSA",
        "SHA512withRSA",
        "SHA256withECDSA",
        "SHA384withECDSA",
        "SHA512withECDSA",
        "SHA-256",
        "SHA-384",
        "SHA-512",
    ] {
        let lote = json_lote(algorithm, false);
        let url = a_batch(&format!("&jsonbatch=true&dat={}", dat(lote.as_bytes())));

        let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
            panic!("es un lote: {algorithm}");
        };
        assert!(request.is_json());
        assert!(!request.is_local());
        assert_eq!(request.algorithm(), algorithm);
    }
}

#[test]
fn a_local_batch_in_json_reads_compound_and_hyphenated_algorithm_names() {
    for algorithm in [
        "SHA256withRSA",
        "SHA384withRSA",
        "SHA512withRSA",
        "SHA256withECDSA",
        "SHA384withECDSA",
        "SHA512withECDSA",
        "SHA-256",
        "SHA-384",
        "SHA-512",
    ] {
        let url = an_operation(&format!(
            "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&jsonbatch=true&dat={}",
            dat(json_lote(algorithm, true).as_bytes())
        ));

        let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
            panic!("es un lote: {algorithm}");
        };
        assert!(request.is_local());
        assert!(request.is_json());
        assert_eq!(request.algorithm(), algorithm);
    }
}

/// Hay sedes en producción que declaran así su lote (ADR-0023).
#[test]
fn a_batch_with_sha1_is_attended_like_the_original_attends_it() {
    for (name, lote_xml) in [
        ("sha1 en XML", xml_lote("sha1", false)),
        ("SHA1 en XML", xml_lote("SHA1", false)),
        ("SHA1withRSA en XML", xml_lote("SHA1withRSA", false)),
        ("SHA-1 en XML", xml_lote("SHA-1", false)),
    ] {
        let url = a_batch(&format!("&dat={}", dat(lote_xml.as_bytes())));
        let SiteOperation::Batch(request) = read_operation(&url).expect(name) else {
            panic!("es un lote: {name}");
        };
        assert!(!request.is_json(), "{name}");
    }

    for (name, lote_json) in [
        ("sha1 en JSON", json_lote("sha1", false)),
        ("SHA1 en JSON", json_lote("SHA1", false)),
        ("SHA1withRSA en JSON", json_lote("SHA1withRSA", false)),
        ("SHA-1 en JSON", json_lote("SHA-1", false)),
    ] {
        let url = a_batch(&format!(
            "&jsonbatch=true&dat={}",
            dat(lote_json.as_bytes())
        ));
        let SiteOperation::Batch(request) = read_operation(&url).expect(name) else {
            panic!("es un lote: {name}");
        };
        assert!(request.is_json(), "{name}");
    }

    for (name, lote_local) in [
        ("sha1 en lote local", json_lote("sha1", false)),
        ("SHA1withRSA en lote local", json_lote("SHA1withRSA", false)),
    ] {
        let url = an_operation(&format!(
            "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&jsonbatch=true&dat={}",
            dat(lote_local.as_bytes())
        ));
        let SiteOperation::Batch(request) = read_operation(&url).expect(name) else {
            panic!("es un lote: {name}");
        };
        assert!(request.is_local(), "{name}");
    }
}

#[test]
fn a_batch_with_ripemd160_is_still_refused_naming_the_parameter() {
    let url = a_batch(&format!(
        "&dat={}",
        dat(xml_lote("RIPEMD160withRSA", false).as_bytes())
    ));

    let refusal = read_operation(&url).expect_err("RIPEMD160 no esta en el catalogo del lote");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

#[test]
fn a_batch_header_without_algorithm_is_refused_naming_the_parameter() {
    let xml_without_algo = "<signbatch stoponerror=\"true\"><singlesign id=\"001\"/></signbatch>";
    let url = a_batch(&format!("&dat={}", dat(xml_without_algo.as_bytes())));
    let refusal = read_operation(&url).expect_err("falta algorithm en XML");
    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));

    let json_without_algo = "{\"stoponerror\":true}";
    let url = a_batch(&format!(
        "&jsonbatch=true&dat={}",
        dat(json_without_algo.as_bytes())
    ));
    let refusal = read_operation(&url).expect_err("falta algorithm en JSON");
    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

#[test]
fn a_batch_without_the_presigner_url_names_it() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let refusal = read_operation(&url).expect_err("falta la url de prefirma");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::BatchPresignerUrl));
}

#[test]
fn a_batch_servlet_url_over_http_is_attended_like_the_original_attends_it() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpresignerurl=http%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    read_operation(&url).expect("'validateURL' del original admite 'http'");
}

#[test]
fn a_local_batch_needs_no_servlet_urls() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&jsonbatch=true&dat={}",
        dat(json_lote("SHA256", true).as_bytes())
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert!(request.is_local());
    assert!(request.is_json());
    assert_eq!(request.presigner_url(), None);
    assert_eq!(request.postsigner_url(), None);
    assert!(request.stops_on_error());
}

/// El original manda el XML heredado a los servlets aunque la sede pida el lote
/// local; rFirma no lo atiende (ver el encabezado del módulo `protocol`).
#[test]
fn a_local_batch_in_the_legacy_xml_is_refused() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let refusal = read_operation(&url).expect_err("el lote local solo existe en JSON");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn a_batch_algorithm_rfirma_cannot_produce_names_the_algorithm_parameter() {
    let url = a_batch(&format!("&dat={}", dat(xml_lote("MD5", false).as_bytes())));

    let refusal = read_operation(&url).expect_err("MD5 no esta en el catalogo del lote");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

#[test]
fn a_batch_reads_needcert_the_filter_and_the_sticky_flags() {
    let url = a_batch(&format!(
        "&needcert=true&sticky=true&resetsticky=true&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert!(request.needcert());
    assert!(request.filter().declares_nothing());
    assert!(request.sticky().is_sticky());
    assert!(request.sticky().resets());
    assert_eq!(
        request.lote_base64(),
        dat(xml_lote("SHA256", false).as_bytes())
    );
    assert_eq!(request.lote(), xml_lote("SHA256", false).as_bytes());
}

#[test]
fn gzip_true_decompresses_the_document_of_local_batch() {
    let plain = json_lote("SHA256", true);
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&jsonbatch=true&gzip=true&dat={}",
        dat(&gzipped(plain.as_bytes()))
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert_eq!(request.lote(), plain.as_bytes());
}

#[test]
fn gzip_true_decompresses_the_document_of_remote_batch() {
    let plain = xml_lote("SHA256", false);
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&jsonbatch=false&gzip=true&\
         batchpresignerurl=https://batch.example/pre&batchpostsignerurl=https://batch.example/post&dat={}",
        dat(&gzipped(plain.as_bytes()))
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert_eq!(request.lote(), plain.as_bytes());
}

#[test]
fn a_batch_of_a_url_downloads_the_batch() {
    let lote =
        br#"{"algorithm":"SHA256withRSA","stoponerror":true,"format":"PAdES","singlesigns":[]}"#;
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&jsonbatch=true&localBatchProcess=true&dat={A_REMOTE_DOCUMENT}"
    ));

    let operation = read_downloading(
        &url,
        &ADownload {
            from: A_REMOTE_DOCUMENT,
            content: lote.to_vec(),
        },
    )
    .expect("se atiende");

    let SiteOperation::Batch(request) = operation else {
        panic!("es un lote");
    };
    assert_eq!(request.algorithm(), "SHA256withRSA");
}

#[test]
fn a_batch_servlet_on_a_local_address_is_refused_as_a_local_access() {
    let refusal = refusal_of(
        "afirma://batch?dat=ZmlybWFkbw&batchpresignerurl=https://localhost/presign\
         &batchpostsignerurl=https://lote.example/postsign",
    );

    assert_eq!(refusal.code(), SafCode::LocalAccessBlocked);
}

#[test]
fn a_batch_servlet_carrying_its_own_parameters_is_refused_naming_it() {
    let refusal = refusal_of(
        "afirma://batch?dat=ZmlybWFkbw&batchpresignerurl=https://lote.example/presign\
         &batchpostsignerurl=https://lote.example/postsign%3Fop%3Dput",
    );

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::BatchPostsignerUrl));
}

#[test]
fn a_batch_servlet_with_an_unsupported_scheme_is_refused_naming_it() {
    let refusal = refusal_of(
        "afirma://batch?dat=ZmlybWFkbw&batchpresignerurl=ftp://lote.example/presign\
         &batchpostsignerurl=https://lote.example/postsign",
    );

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::BatchPresignerUrl));
}
