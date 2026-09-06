use super::{AdmissibleDocument, Refusal};

/// **Grada A**: son bytes, y las reglas se prueban en el carril rápido.
fn a_pdf(body: &str) -> Vec<u8> {
    format!("%PDF-1.7\n{body}\n%%EOF\n").into_bytes()
}

#[test]
fn admits_an_ordinary_pdf() {
    let pdf = a_pdf("1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Size 2 >>");

    let document = AdmissibleDocument::check(&pdf).expect("es un PDF corriente");

    assert_eq!(document.bytes(), pdf.as_slice());
    assert!(!document.already_signed());
}

#[test]
fn refuses_something_that_is_not_a_pdf() {
    let refusal = AdmissibleDocument::check(b"no soy un PDF").expect_err("no es un PDF");

    assert_eq!(refusal, Refusal::NotAPdf);
}

#[test]
fn refuses_an_empty_file_without_reading_past_it() {
    assert_eq!(
        AdmissibleDocument::check(b"").expect_err("está vacío"),
        Refusal::NotAPdf
    );
}

#[test]
fn admits_a_pdf_with_rubbish_before_the_header() {
    // Todos los visores lo abren; rechazarlo sería más estricto que ellos.
    let mut pdf = b"Content-Type: application/pdf\r\n\r\n".to_vec();
    pdf.extend_from_slice(&a_pdf("trailer\n<< /Size 2 >>"));

    assert!(AdmissibleDocument::check(&pdf).is_ok());
}

#[test]
fn refuses_a_pdf_encrypted_through_an_indirect_reference() {
    let pdf = a_pdf("trailer\n<< /Size 9 /Encrypt 8 0 R /Root 1 0 R >>");

    assert_eq!(
        AdmissibleDocument::check(&pdf).expect_err("está cifrado"),
        Refusal::Encrypted
    );
}

#[test]
fn refuses_a_pdf_whose_encryption_dictionary_sits_in_place() {
    let pdf = a_pdf("trailer\n<< /Size 9 /Encrypt << /Filter /Standard /P -44 >> >>");

    assert_eq!(
        AdmissibleDocument::check(&pdf).expect_err("está cifrado"),
        Refusal::Encrypted
    );
}

#[test]
fn says_the_same_thing_about_restricted_permissions_and_about_a_password() {
    // `/P` son los permisos y vive **dentro** del diccionario de cifrado:
    // un PDF que solo prohíbe modificar está cifrado igual, y la negativa
    // es la misma porque la causa es la misma entrada del tráiler.
    let restricted = a_pdf("trailer\n<< /Encrypt 8 0 R >>\n8 0 obj\n<< /P -1340 >>\nendobj");

    assert_eq!(
        AdmissibleDocument::check(&restricted).expect_err("tiene permisos restringidos"),
        Refusal::Encrypted
    );
}

#[test]
fn does_not_mistake_the_word_for_the_entry() {
    // Un documento que **habla** de `/Encrypt` se firma como cualquier
    // otro. Sin esta distinción, la negativa caería sobre un PDF válido.
    let pdf = a_pdf("(La entrada /Encrypt del trailer cifra el documento) Tj");

    assert!(AdmissibleDocument::check(&pdf).is_ok());
}

#[test]
fn refuses_a_certified_pdf() {
    let pdf =
        a_pdf("9 0 obj\n<< /Type /Sig /Reference [ << /TransformMethod /DocMDP >> ] >>\nendobj");

    assert_eq!(
        AdmissibleDocument::check(&pdf).expect_err("está certificado"),
        Refusal::Certified
    );
}

#[test]
fn admits_an_already_signed_pdf_because_that_is_the_cosigning_path() {
    let pdf = a_pdf("9 0 obj\n<< /Type /Sig /ByteRange [0 840 960 240] >>\nendobj");

    let document = AdmissibleDocument::check(&pdf).expect("se cofirma");

    assert!(document.already_signed());
}

#[test]
fn admits_a_pdf_whose_previous_signatures_it_cannot_read_and_says_so() {
    let pdf = a_pdf(
        "9 0 obj\n<< /Type /Sig /SubFilter /ETSI.CAdES.detached /ByteRange [0 8 9 2] >>\nendobj\n             10 0 obj\n<< /Type /Sig /SubFilter /adbe.pkcs7.somethingelse >>\nendobj",
    );

    let document = AdmissibleDocument::check(&pdf).expect("no se rechaza, se pregunta");

    assert!(document.has_unregistered_signatures());
}

#[test]
fn says_nothing_about_a_pdf_signed_only_with_subfilters_the_bridge_reads() {
    let pdf = a_pdf(
        "9 0 obj\n<< /Type /Sig /SubFilter/adbe.pkcs7.detached >>\nendobj\n             10 0 obj\n<< /Type /Sig /SubFilter /adbe.pkcs7.sha1 >>\nendobj\n             11 0 obj\n<< /Type /DocTimeStamp /SubFilter /ETSI.RFC3161 >>\nendobj",
    );

    let document = AdmissibleDocument::check(&pdf).expect("es un PDF corriente");

    assert!(!document.has_unregistered_signatures());
}

#[test]
fn does_not_ask_about_a_subfilter_it_cannot_even_read() {
    let pdf = a_pdf("9 0 obj\n<< /Type /Sig /SubFilter 12 0 R >>\nendobj");

    let document = AdmissibleDocument::check(&pdf).expect("es un PDF");

    assert!(!document.has_unregistered_signatures());
}

#[test]
fn an_ordinary_pdf_has_no_unregistered_signatures() {
    let pdf = a_pdf("1 0 obj\n<< /Type /Catalog >>\nendobj");

    let document = AdmissibleDocument::check(&pdf).expect("es un PDF corriente");

    assert!(!document.has_unregistered_signatures());
    assert!(!document.already_signed());
}

#[test]
fn every_refusal_says_why_and_names_a_situation() {
    for refusal in [Refusal::NotAPdf, Refusal::Encrypted, Refusal::Certified] {
        assert!(!refusal.to_string().is_empty(), "{refusal:?} no dice nada");
        assert!(!refusal.situation().is_empty(), "{refusal:?} no se traduce");
    }
}
