use super::super::*;
use super::fixtures::{
    a_signature_of_a_url, an_operation, dat, gzipped, read_downloading, read_operation,
    the_signed_document, ADownload, ADownloadThatFails, A_REMOTE_DOCUMENT,
};

#[test]
fn a_dat_outside_the_base64_alphabet_is_signed_as_its_text() {
    assert_eq!(the_signed_document("dato*literal!", ""), b"dato*literal!");
}

#[test]
fn a_dat_whose_length_is_not_a_multiple_of_four_is_signed_as_its_text() {
    assert_eq!(the_signed_document("abcde", ""), b"abcde");
}

#[test]
fn a_dat_with_padding_before_its_last_two_characters_is_signed_as_its_text() {
    assert_eq!(the_signed_document("ab=cdefg", ""), b"ab=cdefg");
}

#[test]
fn a_lone_padding_sign_is_signed_as_its_text() {
    assert_eq!(the_signed_document("%3D", ""), b"=");
}

#[test]
fn a_dat_the_decoder_cannot_read_is_signed_as_its_text() {
    assert_eq!(the_signed_document("abc~", ""), b"abc~");
}

#[test]
fn a_literal_dat_is_signed_in_utf8() {
    assert_eq!(
        the_signed_document("%C3%B1and%C3%BA", ""),
        "ñandú".as_bytes()
    );
}

#[test]
fn a_literal_dat_is_trimmed_like_the_original() {
    assert_eq!(the_signed_document("+dato*literal!+", ""), b"dato*literal!");
}

#[test]
fn a_base64_dat_is_still_decoded() {
    assert_eq!(
        the_signed_document(&dat(b"hola rFirma"), ""),
        b"hola rFirma"
    );
}

#[test]
fn a_base64_dat_with_leftover_bits_is_decoded_like_the_original() {
    assert_eq!(the_signed_document("YR==", ""), b"a");
}

#[test]
fn a_base64_dat_split_in_lines_is_decoded() {
    assert_eq!(the_signed_document("aG9s%0AYQ==", ""), b"hola");
}

#[test]
fn gzip_true_over_a_literal_dat_signs_its_text_uncompressed() {
    assert_eq!(
        the_signed_document("dato*literal!", "&gzip=true"),
        b"dato*literal!"
    );
}

#[test]
fn a_signature_that_asks_for_a_local_file_never_gets_read() {
    let url = an_operation("op=sign&dat=file:/etc/passwd");

    let refusal = read_operation(&url).expect_err("no se leen ficheros locales");

    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn a_signature_with_nothing_to_sign_says_exactly_that() {
    let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=A%09%09A");

    let refusal = read_operation(&url).expect_err("no hay nada que firmar");

    assert_eq!(refusal.code(), SafCode::SignWithoutData);
}

#[test]
fn gzip_true_decompresses_the_document_of_sign() {
    let plain = b"%PDF-1.7\nplain-pdf-content";
    let url = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&gzip=true&dat={}",
        dat(&gzipped(plain))
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert_eq!(request.document(), plain);
}

#[test]
fn gzip_true_decompresses_the_document_of_cosign() {
    let plain = b"%PDF-1.7\nplain-pdf-content";
    let url = an_operation(&format!(
        "op=cosign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&gzip=true&dat={}",
        dat(&gzipped(plain))
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert_eq!(request.document(), plain);
}

#[test]
fn gzip_true_decompresses_the_document_of_countersign() {
    let plain = b"cades-signature-bytes";
    let url = an_operation(&format!(
        "op=countersign&idsession=8jAkPZfRw2mQxN4TbYuL&format=CAdES&algorithm=SHA256withRSA&gzip=true&dat={}",
        dat(&gzipped(plain))
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una contrafirma");
    };
    assert_eq!(request.document(), plain);
}

#[test]
fn gzip_true_with_invalid_gzip_is_refused_as_data_parameter() {
    let not_gzipped = b"not-a-gzip-stream";
    let url = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&gzip=true&dat={}",
        dat(not_gzipped)
    ));

    let refusal = read_operation(&url).expect_err("no es gzip");
    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn gzip_false_or_absent_leaves_the_document_untouched() {
    let compressed = gzipped(b"%PDF-1.7\ncontent");
    let without_gzip = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&dat={}",
        dat(&compressed)
    ));
    let with_false = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&gzip=false&dat={}",
        dat(&compressed)
    ));

    let SiteOperation::Sign(first) = read_operation(&without_gzip).expect("se atiende") else {
        panic!("es sign");
    };
    let SiteOperation::Sign(second) = read_operation(&with_false).expect("se atiende") else {
        panic!("es sign");
    };
    assert_eq!(first.document(), compressed.as_slice());
    assert_eq!(second.document(), compressed.as_slice());
}

#[test]
fn a_dat_that_is_an_https_url_is_downloaded_and_signed() {
    let downloaded = b"%PDF-1.7\ndownloaded".to_vec();
    let url = a_signature_of_a_url(SIGN, A_REMOTE_DOCUMENT, "");

    let operation = read_downloading(
        &url,
        &ADownload {
            from: A_REMOTE_DOCUMENT,
            content: downloaded.clone(),
        },
    )
    .expect("se atiende");

    let SiteOperation::Sign(request) = operation else {
        panic!("es una firma");
    };
    assert_eq!(request.document(), downloaded);
}

#[test]
fn a_dat_that_is_an_http_url_is_downloaded_like_the_original() {
    let plain = "http://sede.example/documentos/4711.pdf";
    let url = a_signature_of_a_url(COSIGN, plain, "");

    let operation = read_downloading(
        &url,
        &ADownload {
            from: plain,
            content: b"%PDF-1.7\nplain".to_vec(),
        },
    )
    .expect("el original no distingue los dos esquemas");

    assert!(matches!(operation, SiteOperation::Sign(_)));
}

#[test]
fn a_download_that_fails_names_the_data_parameter() {
    let url = a_signature_of_a_url(SIGN, A_REMOTE_DOCUMENT, "");

    let refusal =
        read_downloading(&url, &ADownloadThatFails).expect_err("la descarga no ha salido");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn an_ftp_dat_is_refused_instead_of_signed_as_its_text() {
    let url = a_signature_of_a_url(SIGN, "ftp://sede.example/4711.pdf", "");

    let refusal = read_operation(&url).expect_err("no se baja por ftp");

    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

/// El original solo descomprime cuando el valor es Base64, así que lo que baja de una URL no
/// pasa nunca por el gunzip (`DataDownloader.downloadData`, 1.9.2).
#[test]
fn a_gzip_flag_never_reaches_what_was_downloaded() {
    let downloaded = b"%PDF-1.7\nsin comprimir".to_vec();
    let url = a_signature_of_a_url(SIGN, A_REMOTE_DOCUMENT, "&gzip=true");

    let operation = read_downloading(
        &url,
        &ADownload {
            from: A_REMOTE_DOCUMENT,
            content: downloaded.clone(),
        },
    )
    .expect("se atiende");

    let SiteOperation::Sign(request) = operation else {
        panic!("es una firma");
    };
    assert_eq!(request.document(), downloaded);
}

#[test]
fn an_empty_dat_is_still_nothing_to_sign_and_not_a_document_to_choose() {
    let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=A%09%09A");

    let refusal = read_operation(&url).expect_err("no hay nada que firmar");

    assert_eq!(refusal.code(), SafCode::SignWithoutData);
}
