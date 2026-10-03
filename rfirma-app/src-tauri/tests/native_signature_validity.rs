//! Prueba de grada C de la validez de cada firma de un PDF y de los hallazgos del documento, contra el puente real (ADR-0043).

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
use rfirma_lib::signing::domain::document_signatures::{
    DocumentFinding, DocumentSignatures, SigningDate, Validity, ValidityReason,
};

use support::{a_cycle_of, bridge, PAGE_HEIGHT, PAGE_WIDTH};

fn report_of(pdf: &[u8]) -> DocumentSignatures {
    bridge()
        .previous_signatures(&base64::engine::general_purpose::STANDARD.encode(pdf))
        .expect("el puente debería leer las firmas")
}

fn sample(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/previous-signatures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn signed(pdf: &[u8]) -> Vec<u8> {
    a_cycle_of(Format::Pades, ALGORITHM, pdf, SignatureOperation::Sign, &[])
}

const CONTENT: &str = "BT /F1 24 Tf 72 750 Td (rfirma: validez) Tj ET\n";

fn a_content_stream(content: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{content}endstream",
        content.len()
    )
}

/// Un PDF de una página con lo que se le añada a la página, al catálogo y, desde el 6, como objetos.
fn a_pdf_with(page: &str, catalog: &str, extra: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R {page} >>"
        ),
        a_content_stream(CONTENT),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    objects.extend(extra.iter().map(|body| (*body).to_owned()));

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref_at = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn last_number_after(pdf: &[u8], key: &str) -> u32 {
    let text = String::from_utf8_lossy(pdf);
    let at = text
        .rfind(key)
        .unwrap_or_else(|| panic!("el PDF debería traer {key}"))
        + key.len();
    text[at..]
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("{key} debería ir seguido de un número"))
}

fn number_before(text: &str, key: &str) -> Option<u32> {
    let at = text.find(key)? + key.len();
    text[at..]
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// Los objetos sin comprimir, en el orden del fichero.
fn plain_objects_of(pdf: &[u8]) -> Vec<(u32, String)> {
    let text = String::from_utf8_lossy(pdf);
    text.match_indices(" 0 obj")
        .filter_map(|(at, _)| {
            let line = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
            let number = text[line..at].parse().ok()?;
            let body = &text[at + " 0 obj".len()..];
            Some((number, body[..body.find("endobj")?].trim().to_owned()))
        })
        .collect()
}

/// Los objetos guardados dentro de los flujos de objetos (`/ObjStm`) que escribe la firma.
fn compressed_objects_of(pdf: &[u8]) -> Vec<(u32, String)> {
    use std::io::Read;

    let mut objects = Vec::new();
    for (_, dictionary) in plain_objects_of(pdf) {
        if !dictionary.contains("/ObjStm") {
            continue;
        }
        let (Some(length), Some(first)) = (
            number_before(&dictionary, "/Length"),
            number_before(&dictionary, "/First"),
        ) else {
            continue;
        };
        let marker = &dictionary[..dictionary.find("stream").expect("es un flujo")];
        let at = pdf
            .windows(marker.len())
            .position(|window| window == marker.as_bytes())
            .expect("el diccionario está en el fichero")
            + marker.len()
            + "stream".len();
        let start = at
            + pdf[at..]
                .iter()
                .take_while(|b| **b == b'\r' || **b == b'\n')
                .count();
        let mut decoded = Vec::new();
        flate2::read::ZlibDecoder::new(&pdf[start..start + length as usize])
            .read_to_end(&mut decoded)
            .expect("el flujo de objetos se descomprime");
        let decoded = String::from_utf8_lossy(&decoded).into_owned();
        let first = first as usize;
        let header: Vec<usize> = decoded[..first]
            .split_whitespace()
            .map(|number| number.parse().expect("cabecera numérica"))
            .collect();
        for (index, pair) in header.chunks(2).enumerate() {
            let end = header
                .get(2 * index + 3)
                .map_or(decoded.len(), |next| first + next);
            let number = u32::try_from(pair[0]).expect("cabe");
            objects.push((number, decoded[first + pair[1]..end].trim().to_owned()));
        }
    }
    objects
}

/// El último objeto que contiene `needle`, comprimido o no: su número y su cuerpo.
fn object_containing(pdf: &[u8], needle: &str) -> (u32, String) {
    plain_objects_of(pdf)
        .into_iter()
        .chain(compressed_objects_of(pdf))
        .rfind(|(_, body)| body.contains(needle))
        .unwrap_or_else(|| panic!("el PDF debería traer un objeto con «{needle}»"))
}

fn latest_body_of(pdf: &[u8], number: u32) -> String {
    plain_objects_of(pdf)
        .into_iter()
        .filter(|(candidate, _)| *candidate == number)
        .map(|(_, body)| body)
        .next_back()
        .unwrap_or_else(|| panic!("el PDF debería traer el objeto {number} sin comprimir"))
}

/// Una revisión incremental sin firma, con una tabla de referencias en flujo como la de la firma.
fn with_a_revision_after_signing(pdf: &[u8], objects: &[(u32, String)]) -> Vec<u8> {
    let previous = last_number_after(pdf, "startxref");
    let table = objects
        .iter()
        .map(|(number, _)| number + 1)
        .fold(last_number_after(pdf, "/Size"), u32::max);
    let root = last_number_after(pdf, "/Root");
    let mut out = pdf.to_vec();
    out.push(b'\n');
    let mut rows = vec![0u8, 0, 0, 0, 0, 0xff, 0xff];
    let mut index = String::from("0 1");
    let in_use = |rows: &mut Vec<u8>, offset: usize| {
        rows.push(1);
        rows.extend_from_slice(&u32::try_from(offset).expect("cabe").to_be_bytes());
        rows.extend_from_slice(&[0, 0]);
    };
    for (number, body) in objects {
        in_use(&mut rows, out.len());
        index.push_str(&format!(" {number} 1"));
        out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = out.len();
    in_use(&mut rows, table_at);
    index.push_str(&format!(" {table} 1"));
    out.extend_from_slice(
        format!(
            "{table} 0 obj\n<< /Type /XRef /Size {} /Index [{index}] /W [1 4 2] /Root {root} 0 R \
             /Prev {previous} /Length {} >>\nstream\n",
            table + 1,
            rows.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&rows);
    out.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{table_at}\n%%EOF\n").as_bytes(),
    );
    out
}

/// Lo que añade un perfil LT después de firmar: un `/DSS` en el catálogo.
fn with_a_dss(pdf: &[u8]) -> Vec<u8> {
    let root = last_number_after(pdf, "/Root");
    let catalog = latest_body_of(pdf, root);
    let with_dss = catalog.replacen("<<", "<< /DSS << >> ", 1);
    with_a_revision_after_signing(pdf, &[(root, with_dss)])
}

/// La versión del encabezado entra en el `/ByteRange`: el resumen de la firma deja de cuadrar.
fn with_the_signed_bytes_altered(pdf: &[u8]) -> Vec<u8> {
    const HEADER: &[u8] = b"%PDF-1.";
    let at = pdf
        .windows(HEADER.len())
        .position(|window| window == HEADER)
        .expect("el encabezado tiene que estar")
        + HEADER.len();
    let mut altered = pdf.to_vec();
    altered[at] = if altered[at] == b'7' { b'4' } else { b'7' };
    altered
}

/// El PKCS#7 de `/Contents` queda fuera del `/ByteRange`: se rompe sin tocar lo firmado.
fn with_the_signature_container_unreadable(pdf: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(pdf);
    let contents = ["/Contents <", "/Contents<"]
        .iter()
        .filter_map(|key| text.rfind(key).map(|at| at + key.len()))
        .max()
        .expect("la firma trae /Contents en hexadecimal");
    let mut broken = pdf.to_vec();
    broken[contents..contents + 64].fill(b'0');
    broken
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_long_term_signature_with_its_certificate_in_force_is_valid() {
    let report = report_of(&with_a_dss(&sample("pades-long-term-active.pdf")));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Valid);
    assert_eq!(signature.validity_reason, None);
    assert_eq!(report.findings(), []);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_signature_with_several_problems_takes_the_worst() {
    let report = report_of(&with_the_signed_bytes_altered(&sample(
        "pades-long-term-expired.pdf",
    )));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(
        signature.validity_reason,
        Some(ValidityReason::ModifiedAfterSigning)
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_long_term_signature_with_its_certificate_expired_is_expired_with_the_date() {
    let report = report_of(&sample("pades-long-term-expired.pdf"));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Expired);
    assert!(
        matches!(
            &signature.validity_reason,
            Some(ValidityReason::CertificateExpired { date, holder: None }) if date.starts_with("2020-")
        ),
        "{:?}",
        signature.validity_reason
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn an_unreadable_signature_is_listed_as_invalid_and_damaged() {
    let report = report_of(&with_the_signature_container_unreadable(&signed(
        &a_pdf_with("", "", &[]),
    )));

    assert_eq!(report.count(), 1);
    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(signature.validity_reason, Some(ValidityReason::Damaged));
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_page_repainted_after_the_last_signature_is_a_finding_of_the_document_only() {
    let signed = signed(&a_pdf_with("", "", &[]));
    let (content, _) = object_containing(&signed, "rfirma: validez");

    let report = report_of(&with_a_revision_after_signing(
        &signed,
        &[(
            content,
            a_content_stream("1 0 0 RG 1 0 0 rg 50 50 400 300 re f\n"),
        )],
    ));

    assert_eq!(
        report.findings(),
        [DocumentFinding::ModifiedAfterLastSignature]
    );
    assert!(report.changed_after_last_signature());
    assert_eq!(report.signatures()[0].validity, Validity::Valid);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_form_filled_after_signing_is_a_finding_of_the_document_only() {
    let field = |value: &str| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (nombre) /V ({value}) \
             /Rect [72 650 272 670] /P 3 0 R >>"
        )
    };
    let form = a_pdf_with(
        "/Annots [6 0 R]",
        "/AcroForm << /Fields [6 0 R] >>",
        &[&field("Ada")],
    );

    let signed = signed(&form);
    let (number, ada) = object_containing(&signed, "(nombre)");

    let report = report_of(&with_a_revision_after_signing(
        &signed,
        &[(number, ada.replace("(Ada)", "(Babbage)"))],
    ));

    assert_eq!(report.findings(), [DocumentFinding::FormFilledAfterSigning]);
    assert_eq!(report.signatures()[0].validity, Validity::Valid);
}

const A_WIDGET: &str = "<< /Type /Annot /Subtype /Widget /Rect [0 0 200 100] /F 4 >>";

const OVER_THE_WIDGET: &str = "<< /Type /Annot /Subtype /Square /Rect [100 50 300 150] /F 4 >>";

/// El objeto de la página con `annotation` añadida al final de su `/Annots`.
fn with_an_annotation_on_the_page(pdf: &[u8], annotation: u32) -> (u32, String) {
    let (page, body) = object_containing(pdf, "/MediaBox");
    let annots = body.find("/Annots").expect("la página trae /Annots");
    let close = annots
        + body[annots..]
            .find(']')
            .expect("/Annots es un array directo");
    let mut updated = body.clone();
    updated.insert_str(close, &format!(" {annotation} 0 R"));
    (page, updated)
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn content_laid_over_another_after_signing_is_a_finding_of_the_document_only() {
    let signed = signed(&a_pdf_with("/Annots [6 0 R]", "", &[A_WIDGET]));
    let added = last_number_after(&signed, "/Size");

    let report = report_of(&with_a_revision_after_signing(
        &signed,
        &[
            with_an_annotation_on_the_page(&signed, added),
            (added, OVER_THE_WIDGET.to_owned()),
        ],
    ));

    assert_eq!(report.findings(), [DocumentFinding::ContentAddedOnTop]);
    assert_eq!(report.signatures()[0].validity, Validity::Valid);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn annotations_already_overlapping_when_signed_are_not_content_added_after_a_dss() {
    let overlapping = a_pdf_with("/Annots [6 0 R 7 0 R]", "", &[A_WIDGET, OVER_THE_WIDGET]);

    let report = report_of(&with_a_dss(&signed(&overlapping)));

    assert_eq!(report.findings(), []);
    assert!(!report.changed_after_last_signature());
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn annotations_touching_only_at_an_edge_are_not_content_added_on_top() {
    let signed = signed(&a_pdf_with("/Annots [6 0 R]", "", &[A_WIDGET]));
    let added = last_number_after(&signed, "/Size");
    let beside = "<< /Type /Annot /Subtype /Square /Rect [200 0 400 100] /F 4 >>";

    let report = report_of(&with_a_revision_after_signing(
        &signed,
        &[
            with_an_annotation_on_the_page(&signed, added),
            (added, beside.to_owned()),
        ],
    ));

    assert_eq!(report.findings(), []);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_freshly_signed_pdf_has_a_valid_signature_and_no_findings() {
    let report = report_of(&signed(&a_pdf_with("", "", &[])));

    assert_eq!(report.signatures()[0].validity, Validity::Valid);
    assert_eq!(report.signatures()[0].validity_reason, None);
    assert_eq!(report.findings(), []);
}

fn stamped_by(signing_date: Option<&SigningDate>) -> (&str, &str) {
    match signing_date {
        Some(SigningDate::Stamped { at, tsa }) => (at, tsa),
        other => panic!("la fecha debería estar sellada: {other:?}"),
    }
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_signature_stamped_while_its_certificate_was_in_force_is_valid_though_it_expired_later() {
    let report = report_of(&sample("pades-stamped-while-in-force.pdf"));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Valid);
    assert_eq!(signature.validity_reason, None);
    assert_eq!(
        stamped_by(signature.signing_date.as_ref()),
        ("2019-06-01T00:00:00Z", "CN=rfirma backdated TSA")
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_signature_stamped_after_its_certificate_expired_is_expired_and_dated_by_its_tsa() {
    let report = report_of(&sample("pades-long-term-expired.pdf"));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Expired);
    assert_eq!(
        stamped_by(signature.signing_date.as_ref()).1,
        "CN=rfirma fake TSA"
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_signature_without_a_stamp_has_its_date_declared() {
    let report = report_of(&signed(&a_pdf_with("", "", &[])));

    let signature = &report.signatures()[0];
    assert!(
        matches!(&signature.signing_date, Some(SigningDate::Declared { at }) if Some(at) == signature.signing_time.as_ref()),
        "{:?}",
        signature.signing_date
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_cosign_of_a_pdf_closed_by_its_certification_is_invalid_and_names_who_closed_it() {
    let report = report_of(&sample("pades-certified-then-cosigned.pdf"));

    let [closer, cosign] = report.signatures() else {
        panic!("debería traer dos firmas: {:?}", report.signatures());
    };
    assert!(closer.closes_document);
    assert_eq!(closer.validity, Validity::Valid);
    assert!(!cosign.closes_document);
    assert_eq!(cosign.validity, Validity::Invalid);
    assert!(
        matches!(
            &cosign.validity_reason,
            Some(ValidityReason::CosignNotAdmitted { closed_by: Some(name) }) if name.contains("99999999R")
        ),
        "{:?}",
        cosign.validity_reason
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn with_several_certifications_the_last_one_closes_and_an_expired_cosign_is_invalid() {
    let report = report_of(&sample("pades-certified-twice-then-cosigned-expired.pdf"));

    let [forms_allowed, closer, cosign] = report.signatures() else {
        panic!("debería traer tres firmas: {:?}", report.signatures());
    };
    assert!(!forms_allowed.closes_document);
    assert_eq!(forms_allowed.validity, Validity::Valid);
    assert!(closer.closes_document);
    assert_eq!(
        closer.validity,
        Validity::Valid,
        "{:?}",
        closer.validity_reason
    );
    assert_eq!(cosign.validity, Validity::Invalid, "gana el peor problema");
    assert!(
        matches!(
            &cosign.validity_reason,
            Some(ValidityReason::CosignNotAdmitted { closed_by: Some(name) }) if name.contains("TEST-0000")
        ),
        "{:?}",
        cosign.validity_reason
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn two_ordinary_signatures_without_certification_are_both_valid() {
    let report = report_of(&signed(&signed(&a_pdf_with("", "", &[]))));

    assert_eq!(report.count(), 2);
    for signature in report.signatures() {
        assert_eq!(
            signature.validity,
            Validity::Valid,
            "{:?}",
            signature.validity_reason
        );
        assert!(!signature.closes_document);
    }
}
