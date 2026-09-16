use super::*;

const AN_INVOICE: &[u8] = b"<?xml version=\"1.0\"?>\
<fe:Facturae xmlns:fe=\"http://www.facturae.es/Facturae/2009/v3.2/Facturae\">\
<FileHeader><SchemaVersion>3.2</SchemaVersion></FileHeader>\
<Parties><SellerParty/></Parties>\
<Invoices><Invoice/></Invoices>\
</fe:Facturae>";

#[test]
fn a_pdf_header_is_a_pdf() {
    assert_eq!(shape_of(b"%PDF-1.7\n%obj..."), DetectedShape::Pdf);
}

#[test]
fn an_xml_prolog_is_xml() {
    assert_eq!(
        shape_of(b"<?xml version=\"1.0\"?><documento>hola</documento>"),
        DetectedShape::Xml
    );
}

#[test]
fn a_leading_angle_bracket_without_a_prolog_is_xml() {
    assert_eq!(shape_of(b"   <documento/>"), DetectedShape::Xml);
}

#[test]
fn anything_else_is_binary() {
    assert_eq!(shape_of(&[0x00, 0x01, 0x02, 0x03]), DetectedShape::Binary);
}

#[test]
fn a_facturae_root_with_its_three_children_is_an_invoice() {
    assert_eq!(shape_of(AN_INVOICE), DetectedShape::Invoice);
}

#[test]
fn the_reference_invoice_of_the_kit_is_an_invoice() {
    assert_eq!(
        shape_of(include_bytes!(
            "../../../../../../../testdata/reference/invoice.xml"
        )),
        DetectedShape::Invoice
    );
}

#[test]
fn a_root_that_is_not_facturae_is_plain_xml_however_many_children_it_has() {
    assert_eq!(
        shape_of(b"<Factura><FileHeader/><Parties/><Invoices/></Factura>"),
        DetectedShape::Xml
    );
}

#[test]
fn a_facturae_root_missing_one_of_the_three_children_is_plain_xml() {
    assert_eq!(
        shape_of(b"<Facturae><FileHeader/><Parties/></Facturae>"),
        DetectedShape::Xml
    );
}

#[test]
fn the_three_children_have_to_hang_from_the_root_and_not_from_a_grandchild() {
    assert_eq!(
        shape_of(b"<Facturae><FileHeader/><Wrapper><Parties/><Invoices/></Wrapper></Facturae>"),
        DetectedShape::Xml
    );
}

#[test]
fn what_starts_like_xml_but_does_not_parse_is_plain_xml_and_not_an_invoice() {
    assert_eq!(
        shape_of(b"<Facturae><FileHeader></Parties>"),
        DetectedShape::Xml
    );
}

#[test]
fn a_pdf_with_byterange_or_type_sig_is_detected_as_pdf_signature() {
    assert_eq!(
        detect_signature(b"%PDF-1.7\n/ByteRange [0 10 20 30]\n"),
        Some(DetectedSignature::Pdf)
    );
    assert_eq!(
        detect_signature(b"%PDF-1.7\n<< /Type /Sig >>\n"),
        Some(DetectedSignature::Pdf)
    );
    assert_eq!(
        detect_signature(b"%PDF-1.7\n<< /Type/Sig >>\n"),
        Some(DetectedSignature::Pdf)
    );
    assert_eq!(detect_signature(b"%PDF-1.7\nsin firma"), None);
}

#[test]
fn a_signed_xml_is_detected_as_xml_signature() {
    assert_eq!(
        detect_signature(include_bytes!(
            "../../../../../../../testdata/reference/xades-enveloping.xml"
        )),
        Some(DetectedSignature::Xml)
    );
    assert_eq!(
        detect_signature(include_bytes!(
            "../../../../../../../testdata/reference/document.xml"
        )),
        None
    );
}

#[test]
fn a_cms_signed_data_container_is_detected_as_cms_signature() {
    assert_eq!(
        detect_signature(include_bytes!(
            "../../../../../../../testdata/reference/cades-implicit.p7s"
        )),
        Some(DetectedSignature::Cms)
    );
    assert_eq!(
        detect_signature(include_bytes!(
            "../../../../../../../testdata/reference/cades-explicit.p7s"
        )),
        Some(DetectedSignature::Cms)
    );
    assert_eq!(
        detect_signature(include_bytes!(
            "../../../../../../../testdata/reference/challenge.bin"
        )),
        None
    );
}

#[test]
fn an_invoice_is_detected_as_invoice_signature() {
    assert_eq!(
        detect_signature(AN_INVOICE),
        Some(DetectedSignature::Invoice)
    );
}
