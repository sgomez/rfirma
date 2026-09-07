use super::*;

#[test]
fn a_pdf_header_is_a_pdf() {
    assert_eq!(shape_of(b"%PDF-1.7\n%obj..."), DetectedShape::Pdf);
}

#[test]
fn an_xml_prolog_is_xml() {
    assert_eq!(
        shape_of(b"<?xml version=\"1.0\"?><Facturae/>"),
        DetectedShape::Xml
    );
}

#[test]
fn a_leading_angle_bracket_without_a_prolog_is_xml() {
    assert_eq!(shape_of(b"   <Facturae/>"), DetectedShape::Xml);
}

#[test]
fn anything_else_is_binary() {
    assert_eq!(shape_of(&[0x00, 0x01, 0x02, 0x03]), DetectedShape::Binary);
}
