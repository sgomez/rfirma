//! Pruebas de integración de XAdES y FacturaE contra el token: variantes, curva elíptica, cofirma y contrafirma (ADR-0001, ADR-0014).

#[path = "native_cycle/support.rs"]
mod support;

mod full_cycle {
    use rfirma_lib::signing::application::cycle;
    use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation, XadesVariant};

    use super::support::{
        a_cycle_of, a_cycle_signed_by, certificate_labelled, ecdsa_composed_for_the_ec_certificate,
        the_original_validator_accepts, write_to_target, ACTIVE_EC, PIN,
    };

    /// El XML que firman las cuatro variantes XAdES.
    const A_REFERENCE_XML: &[u8] = include_bytes!("../../../testdata/reference/document.xml");

    fn sign_xades(variant: XadesVariant) -> Vec<u8> {
        a_cycle_of(
            Format::Xades(variant),
            cycle::ALGORITHM,
            A_REFERENCE_XML,
            SignatureOperation::Sign,
            &[],
        )
    }

    fn signed_xml(variant: XadesVariant, name: &str) -> String {
        let signed = sign_xades(variant);
        let path = write_to_target(name, &signed);

        the_original_validator_accepts(&path);
        String::from_utf8(signed).expect("una firma XAdES es XML en UTF-8")
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_enveloping_xades_signature_carries_the_document_inside_and_validates() {
        let signed = signed_xml(XadesVariant::Enveloping, "xades-enveloping.xml");

        assert!(
            signed.contains("Signature"),
            "el XML firmado lleva la firma: {signed}"
        );
        assert!(
            !signed.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<documento"),
            "en Enveloping la raíz es la firma y no el documento: {signed}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_detached_xades_signature_leaves_the_data_beside_it_and_validates() {
        let signed = signed_xml(XadesVariant::Detached, "xades-detached.xml");

        assert!(
            signed.contains("Documento de prueba"),
            "en Detached los datos viajan al lado de la firma: {signed}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_enveloped_xades_signature_keeps_the_root_of_the_document_and_validates() {
        let signed = signed_xml(XadesVariant::Enveloped, "xades-enveloped.xml");

        assert!(
            signed.contains("<documento") && signed.contains("Signature"),
            "en Enveloped la firma cuelga del documento y la raíz sigue siendo la suya: {signed}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_enveloping_xades_signature_made_with_the_ec_certificate_validates() {
        let certificate = certificate_labelled(ACTIVE_EC);
        let algorithm = ecdsa_composed_for_the_ec_certificate(&certificate);

        let signed = a_cycle_signed_by(
            &certificate,
            PIN,
            Format::Xades(XadesVariant::Enveloping),
            algorithm,
            A_REFERENCE_XML,
            SignatureOperation::Sign,
            &[],
        );
        let path = write_to_target("xades-ecdsa.xml", &signed);

        the_original_validator_accepts(&path);
        assert!(
            String::from_utf8_lossy(&signed).contains("ecdsa-sha256"),
            "el XML declara el algoritmo de firma de curva elíptica"
        );
    }

    /// Las entradas que el ASiC-S del original mete en el ZIP.
    const ASIC_SIGNATURE_ENTRY: &[u8] = b"META-INF/signatures.xml";
    const ASIC_DATA_ENTRY: &[u8] = b"dataobject.xml";
    const ASIC_MIME_TYPE: &[u8] = b"application/vnd.etsi.asic-s+zip";

    fn contains(container: &[u8], needle: &[u8]) -> bool {
        container
            .windows(needle.len())
            .any(|window| window == needle)
    }

    /// El ASiC-S no lo valida `afirma-crypto-validation`: su firma es externally
    /// detached y la referencia se resuelve con el fichero que viaja en el ZIP,
    /// que es lo que comprueba `XadesVariantsTest` del puente.
    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_asic_s_xades_signature_comes_back_as_a_container_with_the_document_and_its_signature() {
        let container = sign_xades(XadesVariant::AsicS);

        assert_eq!(
            &container[..4],
            b"PK\x03\x04",
            "un ASiC-S es un ZIP y empieza por su firma de fichero local"
        );
        for entry in [ASIC_MIME_TYPE, ASIC_DATA_ENTRY, ASIC_SIGNATURE_ENTRY] {
            assert!(
                contains(&container, entry),
                "al contenedor le falta {}",
                String::from_utf8_lossy(entry)
            );
        }
    }

    /// La factura de referencia del kit, la que valida el original.
    const A_REFERENCE_INVOICE: &[u8] = include_bytes!("../../../testdata/reference/invoice.xml");

    /// La política que el firmador de facturas impone, la pida la sede o no.
    const FACTURAE_POLICY: &str = "politica_de_firma_formato_facturae_v3_1.pdf";

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_facturae_signature_keeps_the_invoice_as_its_root_and_validates() {
        let signed = a_cycle_of(
            Format::FacturaE,
            cycle::ALGORITHM,
            A_REFERENCE_INVOICE,
            SignatureOperation::Sign,
            &[],
        );
        let path = write_to_target("facturae.xsig", &signed);

        the_original_validator_accepts(&path);

        let text = String::from_utf8(signed).expect("una factura firmada es XML en UTF-8");
        assert!(
            text.contains("Facturae") && text.contains("Signature"),
            "la firma cuelga de la factura y la raíz sigue siendo la suya: {text}"
        );
        assert!(
            text.contains(FACTURAE_POLICY),
            "la firma de una factura declara la política de FacturaE 3.1: {text}"
        );
    }

    /// La firma XAdES Enveloping del banco de referencia, la entrada de una cofirma o una contrafirma.
    const A_REFERENCE_XADES: &[u8] =
        include_bytes!("../../../testdata/reference/xades-enveloping.xml");

    /// Cofirma o contrafirma un XAdES Enveloping.
    fn xades_cycle(
        data: &[u8],
        operation: SignatureOperation,
        declared: &[(&str, &str)],
    ) -> Vec<u8> {
        a_cycle_of(
            Format::Xades(XadesVariant::Enveloping),
            cycle::ALGORITHM,
            data,
            operation,
            declared,
        )
    }

    /// Cuántas firmas lleva un XAdES, anidadas incluidas: la contrafirma lo está.
    fn xades_signers_in(signature: &[u8]) -> usize {
        let count = signature
            .windows(b"<ds:Signature ".len())
            .filter(|window| *window == b"<ds:Signature ")
            .count();
        assert!(
            count > 0,
            "el documento no trae ninguna firma: ¿ha cambiado el serializador el prefijo `ds:`?"
        );
        count
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_xades_cosignature_over_the_reference_signature_validates() {
        let cosigned = xades_cycle(A_REFERENCE_XADES, SignatureOperation::Cosign, &[]);
        let path = write_to_target("xades-cofirma.xml", &cosigned);

        assert_eq!(
            xades_signers_in(&cosigned),
            xades_signers_in(A_REFERENCE_XADES) + 1,
            "la cofirma añade una firma de nivel superior junto a la que cofirmó"
        );
        the_original_validator_accepts(&path);
    }

    /// La contrafirma de referencia sobre las hojas, la medida de cuántos firmantes añade una.
    const A_REFERENCE_XADES_COUNTERSIGN: &[u8] =
        include_bytes!("../../../testdata/reference/xades-enveloping.countersign-leafs.xml");

    /// Contrafirma un XAdES Enveloping con el objetivo pedido, lo valida y devuelve el resultado.
    fn xades_countersign(signature: &[u8], target: &str, name: &str) -> Vec<u8> {
        let countersigned = xades_cycle(
            signature,
            SignatureOperation::Countersign,
            &[("target", target)],
        );
        let written = write_to_target(name, &countersigned);

        assert_ne!(
            countersigned, signature,
            "la contrafirma tiene que haber añadido algo"
        );
        the_original_validator_accepts(&written);
        countersigned
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_xades_countersignature_over_the_leafs_adds_the_signer_that_the_reference_adds() {
        let countersigned =
            xades_countersign(A_REFERENCE_XADES, "leafs", "xades-contrafirma-leafs.xml");

        assert_eq!(
            xades_signers_in(&countersigned),
            xades_signers_in(A_REFERENCE_XADES_COUNTERSIGN),
            "la contrafirma sobre las hojas deja los mismos firmantes que la del original"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_xades_countersignature_over_the_whole_tree_reaches_more_signers_than_over_the_leafs() {
        let once = xades_countersign(A_REFERENCE_XADES, "leafs", "xades-contrafirma-una-vez.xml");
        let leafs = xades_countersign(&once, "leafs", "xades-contrafirma-leafs-otra-vez.xml");
        let tree = xades_countersign(&once, "tree", "xades-contrafirma-tree.xml");

        assert_eq!(
            xades_signers_in(&leafs),
            xades_signers_in(&once) + 1,
            "sobre las hojas se contrafirma solo el firmante mas profundo"
        );
        assert!(
            xades_signers_in(&tree) > xades_signers_in(&leafs),
            "sobre el arbol se contrafirma tambien el firmante de arriba: {} frente a {}",
            xades_signers_in(&tree),
            xades_signers_in(&leafs)
        );
    }
}
