use super::*;

/// **Grada A**: se lee un nombre y sale un formato. No hay puente ni documento
/// que abrir más allá de su cabecera.
#[test]
fn every_name_of_the_original_reads_as_the_format_it_names() {
    let table = [
        ("PAdES", RequestedFormat::Pades),
        ("PAdEStri", RequestedFormat::Pades),
        ("Adobe PDF", RequestedFormat::Pades),
        ("CAdES", RequestedFormat::Cades),
        ("CAdEStri", RequestedFormat::Cades),
        ("CAdES-ASiC-S", RequestedFormat::CadesAsicS),
        ("CMS/PKCS#7", RequestedFormat::Cms),
        ("XAdES", RequestedFormat::Xades(XadesEnvelope::Enveloping)),
        (
            "XAdEStri",
            RequestedFormat::Xades(XadesEnvelope::Enveloping),
        ),
        (
            "XAdES Enveloping",
            RequestedFormat::Xades(XadesEnvelope::Enveloping),
        ),
        (
            "XAdES Detached",
            RequestedFormat::Xades(XadesEnvelope::Detached),
        ),
        (
            "XAdES Enveloped",
            RequestedFormat::Xades(XadesEnvelope::Enveloped),
        ),
        ("XAdES-ASiC-S", RequestedFormat::Xades(XadesEnvelope::AsicS)),
        (
            "XMLDSig",
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Enveloping),
        ),
        (
            "XMLDSig Enveloping",
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Enveloping),
        ),
        (
            "XMLDSig Detached",
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Detached),
        ),
        (
            "XMLDSig Enveloped",
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Enveloped),
        ),
        ("FacturaE", RequestedFormat::FacturaE),
        ("FacturaEtri", RequestedFormat::FacturaE),
        ("Factura-e", RequestedFormat::FacturaE),
    ];

    for (name, expected) in table {
        assert_eq!(
            RequestedFormat::named(name),
            Some(expected),
            "'{name}' es un nombre del original"
        );
    }
}

#[test]
fn a_name_is_read_without_telling_capitals_apart_and_without_its_spaces_around() {
    assert_eq!(
        RequestedFormat::named("  xAdEs DeTaChEd "),
        Some(RequestedFormat::Xades(XadesEnvelope::Detached))
    );
}

#[test]
fn what_the_original_does_not_sign_in_three_phases_names_no_format() {
    for name in ["OOXML", "ODF", "SOAP", "NONE", "PKCS1", ""] {
        assert_eq!(
            RequestedFormat::named(name),
            None,
            "'{name}' no lo firma el original en tres fases"
        );
    }
}

#[test]
fn the_effective_format_of_auto_comes_from_the_header_of_the_document() {
    assert_eq!(format_of(b"%PDF-1.7\n"), RequestedFormat::Pades);
    assert_eq!(
        format_of(b"<?xml version=\"1.0\"?><Facturae/>"),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
    assert_eq!(format_of(&[0x00, 0x01, 0x02]), RequestedFormat::Cades);
}
