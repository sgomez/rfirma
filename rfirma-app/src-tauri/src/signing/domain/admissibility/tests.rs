use std::collections::BTreeMap;

use super::{unlocked_with, AdmissibleDocument, Refusal, Waivers};
use crate::signing::domain::bridge::Format;

/// **Grada A**: son bytes, y las reglas se prueban en el carril rápido.
fn a_pdf(body: &str) -> Vec<u8> {
    format!("%PDF-1.7\n{body}\n::EOF\n").into_bytes()
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

fn a_certified_pdf() -> Vec<u8> {
    a_pdf("9 0 obj\n<< /Type /Sig /Reference [ << /TransformMethod /DocMDP >> ] >>\nendobj")
}

fn an_encrypted_pdf() -> Vec<u8> {
    a_pdf("trailer\n<< /Size 9 /Encrypt 8 0 R /Root 1 0 R >>")
}

fn declaring(params: &[(&str, &str)]) -> Waivers {
    Waivers::declared_in(params.iter().copied())
}

#[test]
fn allow_signing_certified_pdfs_admits_a_certified_pdf() {
    let pdf = a_certified_pdf();

    let waivers = declaring(&[("allowSigningCertifiedPdfs", "TRUE")]);

    assert!(AdmissibleDocument::check_waiving(&pdf, waivers).is_ok());
}

#[test]
fn a_certified_pdf_the_request_forbids_is_still_refused_without_awaiting_anyone() {
    let pdf = a_certified_pdf();
    let waivers = declaring(&[("allowSigningCertifiedPdfs", "false")]);

    let refusal = AdmissibleDocument::check_waiving(&pdf, waivers).expect_err("prohibido");

    assert_eq!(refusal, Refusal::Certified);
    assert!(!refusal.awaits_the_person(waivers));
}

#[test]
fn a_certified_pdf_the_request_says_nothing_about_awaits_the_person() {
    let refusal = AdmissibleDocument::check(&a_certified_pdf()).expect_err("certificado");

    assert!(refusal.awaits_the_person(Waivers::NONE));
}

#[test]
fn a_password_in_the_request_admits_an_encrypted_pdf() {
    for key in ["userPassword", "ownerPassword"] {
        let waivers = declaring(&[(key, "1234")]);

        assert!(
            AdmissibleDocument::check_waiving(&an_encrypted_pdf(), waivers).is_ok(),
            "{key} abre el PDF"
        );
    }
}

#[test]
fn an_encrypted_pdf_without_a_password_awaits_the_person() {
    let refusal = AdmissibleDocument::check(&an_encrypted_pdf()).expect_err("cifrado");

    assert_eq!(refusal, Refusal::Encrypted);
    assert!(refusal.awaits_the_person(Waivers::NONE));
}

#[test]
fn an_encrypted_pdf_is_admitted_when_the_person_will_type_its_password() {
    let waivers = Waivers::NONE.the_person_types_the_password();

    assert!(AdmissibleDocument::check_waiving(&an_encrypted_pdf(), waivers).is_ok());
    assert!(!Waivers::NONE.declares_a_password());
    assert!(declaring(&[("userPassword", "1234")]).declares_a_password());
}

#[test]
fn the_typed_password_opens_the_pdf_as_its_owner_and_the_declared_one_is_forgotten() {
    let declared = BTreeMap::from([
        ("userPassword".to_owned(), "mal".to_owned()),
        ("headless".to_owned(), "false".to_owned()),
    ]);

    let unlocked = unlocked_with(&declared, "1234");

    assert_eq!(
        unlocked,
        BTreeMap::from([
            ("ownerPassword".to_owned(), "1234".to_owned()),
            ("headless".to_owned(), "false".to_owned()),
        ])
    );
}

#[test]
fn what_is_not_a_pdf_awaits_no_one() {
    assert!(!Refusal::NotAPdf.awaits_the_person(Waivers::NONE));
}

#[test]
fn a_password_does_not_waive_the_certification_behind_it() {
    let mut pdf = an_encrypted_pdf();
    pdf.extend_from_slice(&a_certified_pdf());

    let refusal = AdmissibleDocument::check_waiving(&pdf, declaring(&[("userPassword", "1234")]))
        .expect_err("sigue certificado");

    assert_eq!(refusal, Refusal::Certified);
}

#[test]
fn a_format_other_than_pades_is_admitted_whatever_the_waivers() {
    assert!(AdmissibleDocument::check_for(Format::Cades, b"no soy un PDF", Waivers::NONE).is_ok());
}
