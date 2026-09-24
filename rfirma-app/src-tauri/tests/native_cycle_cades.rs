//! Pruebas de integración de CAdES contra el token: variantes, curva elíptica, cofirma y contrafirma (ADR-0001, ADR-0014).

#[path = "native_cycle/support.rs"]
mod support;

mod full_cycle {
    use rfirma_lib::identity::domain::algorithm::SignatureAlgorithm;
    use rfirma_lib::signing::application::cycle;
    use rfirma_lib::signing::domain::bridge::{ExpandRequest, Format, SignatureOperation};
    use rfirma_lib::site::adapters::desk::composed_for;
    use rfirma_lib::site::domain::protocol::pairs_of;
    use rfirma_lib::site::domain::protocol::AskedAlgorithm;

    use super::support::{
        a_cycle_of, a_cycle_signed_by, an_installed_certificate, bridge, cades_cycle,
        ecdsa_composed_for_the_ec_certificate, openssl_cms_finds_no_content_in, openssl_cms_verify,
        openssl_prints_the_certificates_of, sign_cades, signing_certificate, the_cms_inside,
        the_original_validator_accepts, write_to_target, ACTIVE_EC, CHALLENGE, NO_SECRET, PIN,
    };
    use super::support::{a_one_page_pdf, certificate_labelled};

    /// La entrada con la que el ASiC-S de CAdES nombra su firma dentro del ZIP.
    const ASIC_BINARY_SIGNATURE_ENTRY: &[u8] = b"META-INF/signature.p7s";
    const ASIC_MIME_TYPE: &[u8] = b"application/vnd.etsi.asic-s+zip";

    fn contains(container: &[u8], needle: &[u8]) -> bool {
        container
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn the_cms_carries_the_signer_and_the_authority_that_issued_it() {
        let installed = tempfile::tempdir().expect("deberia haber directorio temporal");
        let certificate = an_installed_certificate(installed.path());

        let signed = a_cycle_signed_by(
            &certificate,
            NO_SECRET,
            Format::Cades,
            cycle::ALGORITHM,
            CHALLENGE,
            SignatureOperation::Sign,
            &[("mode", "implicit")],
        );
        let signature = write_to_target("cades-con-cadena.p7s", &signed);

        let carried = openssl_prints_the_certificates_of(&signature);
        assert_eq!(
            carried.matches("subject=").count(),
            2,
            "el firmante y la intermedia de la FNMT, y nada mas:\n{carried}"
        );
        assert!(
            carried.contains("AC FNMT Usuarios"),
            "la intermedia que emitio al firmante va dentro del CMS:\n{carried}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn the_cms_of_a_signed_pdf_carries_the_signer_and_the_authority_that_issued_it() {
        let installed = tempfile::tempdir().expect("deberia haber directorio temporal");
        let certificate = an_installed_certificate(installed.path());

        let signed = a_cycle_signed_by(
            &certificate,
            NO_SECRET,
            Format::Pades,
            cycle::ALGORITHM,
            &a_one_page_pdf(),
            SignatureOperation::Sign,
            &[],
        );
        let pdf = write_to_target("pades-con-cadena.pdf", &signed);

        let carried = openssl_prints_the_certificates_of(&the_cms_inside(&pdf));
        assert_eq!(
            carried.matches("subject=").count(),
            2,
            "el CMS del PDF lleva al firmante y a la intermedia de la FNMT, y nada mas:\n{carried}"
        );
        assert!(
            carried.contains("AC FNMT Usuarios"),
            "la intermedia que emitio al firmante viaja dentro del PDF:\n{carried}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_implicit_cades_signature_carries_the_challenge_and_openssl_verifies_it() {
        let signature = write_to_target("cades-implicito.p7s", &sign_cades(CHALLENGE, "implicit"));

        assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
        the_original_validator_accepts(&signature);
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_signature_with_the_sha512_the_site_asked_for_validates() {
        let algorithm = composed_for(AskedAlgorithm::Sha512, signing_certificate().key_kind());
        assert_eq!(algorithm, SignatureAlgorithm::Sha512Rsa);

        let signed = a_cycle_of(
            Format::Cades,
            algorithm,
            CHALLENGE,
            SignatureOperation::Sign,
            &[("mode", "implicit")],
        );
        let signature = write_to_target("cades-sha512.p7s", &signed);

        assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
        the_original_validator_accepts(&signature);
    }

    const THE_AGE_POLICY_OID: &[u8] = &[
        0x06, 0x0a, 0x60, 0x85, 0x54, 0x01, 0x03, 0x01, 0x01, 0x02, 0x01, 0x09,
    ];

    fn signed_under_the_age_policy(format: Format, data: &[u8]) -> Vec<u8> {
        let expanded = bridge()
            .expand_extra_params(ExpandRequest {
                extra_params: "expPolicy=FirmaAGE\n",
                format: format.name(),
                signed_data_length: data.len(),
            })
            .expect("el expansor tiene que contestar");
        let declared = pairs_of(&expanded);
        let declared: Vec<(&str, &str)> = declared
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let installed = tempfile::tempdir().expect("deberia haber directorio temporal");

        a_cycle_signed_by(
            &an_installed_certificate(installed.path()),
            NO_SECRET,
            format,
            cycle::ALGORITHM,
            data,
            SignatureOperation::Sign,
            &declared,
        )
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_signature_under_the_age_policy_carries_the_policy_and_the_data() {
        let signed = signed_under_the_age_policy(Format::Cades, CHALLENGE);
        let signature = write_to_target("cades-politica-age.p7s", &signed);

        assert!(contains(&signed, THE_AGE_POLICY_OID));
        assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_pades_signature_under_the_age_policy_carries_the_policy() {
        let signed = signed_under_the_age_policy(Format::Pades, &a_one_page_pdf());
        let pdf = write_to_target("pades-politica-age.pdf", &signed);

        let cms = std::fs::read(the_cms_inside(&pdf)).expect("el CMS del PDF");
        assert!(contains(&cms, THE_AGE_POLICY_OID));
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_explicit_cades_signature_is_detached_and_openssl_verifies_it_against_the_challenge() {
        let signature = write_to_target("cades-explicito.p7s", &sign_cades(CHALLENGE, "explicit"));
        let challenge = write_to_target("cades-reto.bin", CHALLENGE);

        openssl_cms_finds_no_content_in(&signature);
        assert_eq!(openssl_cms_verify(&signature, Some(&challenge)), CHALLENGE);
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn an_asic_s_cades_signature_comes_back_as_a_container_with_its_signature_inside() {
        let container = a_cycle_of(
            Format::CadesAsicS,
            cycle::ALGORITHM,
            CHALLENGE,
            SignatureOperation::Sign,
            &[],
        );

        assert_eq!(
            &container[..4],
            b"PK\x03\x04",
            "un ASiC-S es un ZIP y empieza por su firma de fichero local"
        );
        for entry in [ASIC_MIME_TYPE, ASIC_BINARY_SIGNATURE_ENTRY] {
            assert!(
                contains(&container, entry),
                "al contenedor le falta {}",
                String::from_utf8_lossy(entry)
            );
        }
        the_original_validator_accepts(&write_to_target("cades-asic-s.asics", &container));
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_signature_made_with_the_ec_certificate_validates() {
        let certificate = certificate_labelled(ACTIVE_EC);
        let algorithm = ecdsa_composed_for_the_ec_certificate(&certificate);

        let signed = a_cycle_signed_by(
            &certificate,
            PIN,
            Format::Cades,
            algorithm,
            CHALLENGE,
            SignatureOperation::Sign,
            &[("mode", "implicit")],
        );
        let signature = write_to_target("cades-ecdsa.p7s", &signed);

        assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
        the_original_validator_accepts(&signature);
    }

    /// La firma CAdES implícita del banco de referencia, la entrada de una cofirma o una contrafirma.
    const A_REFERENCE_CADES: &[u8] =
        include_bytes!("../../../testdata/reference/cades-implicit.p7s");

    /// El reto que firma `cades-implicit.p7s`, para verificar la cofirma con openssl.
    const A_REFERENCE_CHALLENGE: &[u8] =
        include_bytes!("../../../testdata/reference/challenge.bin");

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_cosignature_over_the_reference_signature_validates() {
        let cosigned = cades_cycle(
            A_REFERENCE_CADES,
            SignatureOperation::Cosign,
            &[("mode", "implicit")],
        );
        let signature = write_to_target("cades-cofirma.p7s", &cosigned);

        assert_eq!(
            openssl_cms_verify(&signature, None),
            A_REFERENCE_CHALLENGE,
            "la cofirma conserva el contenido de la firma que cofirmó"
        );
        the_original_validator_accepts(&signature);
    }

    /// La contrafirma de referencia sobre las hojas, la medida de cuántos firmantes añade una.
    const A_REFERENCE_COUNTERSIGN: &[u8] =
        include_bytes!("../../../testdata/reference/cades-implicit.countersign-leafs.p7s");

    /// El atributo `messageDigest` (1.2.840.113549.1.9.4), uno por `SignerInfo` del CMS.
    const MESSAGE_DIGEST_OID: &[u8] = &[
        0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x04,
    ];

    fn signers_in(signature: &[u8]) -> usize {
        signature
            .windows(MESSAGE_DIGEST_OID.len())
            .filter(|window| *window == MESSAGE_DIGEST_OID)
            .count()
    }

    /// Contrafirma un CAdES con el objetivo pedido, lo valida y devuelve el resultado.
    fn countersign(signature: &[u8], target: &str, name: &str) -> Vec<u8> {
        let countersigned = cades_cycle(
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
    fn a_cades_countersignature_over_the_leafs_adds_the_signer_that_the_reference_adds() {
        let countersigned = countersign(A_REFERENCE_CADES, "leafs", "cades-contrafirma-leafs.p7s");

        assert_eq!(
            signers_in(&countersigned),
            signers_in(A_REFERENCE_COUNTERSIGN),
            "la contrafirma sobre las hojas deja los mismos firmantes que la del original"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_countersignature_over_the_whole_tree_reaches_more_signers_than_over_the_leafs() {
        let once = countersign(A_REFERENCE_CADES, "leafs", "cades-contrafirma-una-vez.p7s");
        let leafs = countersign(&once, "leafs", "cades-contrafirma-leafs-otra-vez.p7s");
        let tree = countersign(&once, "tree", "cades-contrafirma-tree.p7s");

        assert_eq!(
            signers_in(&leafs),
            signers_in(&once) + 1,
            "sobre las hojas se contrafirma solo el firmante mas profundo"
        );
        assert!(
            signers_in(&tree) > signers_in(&leafs),
            "sobre el arbol se contrafirma tambien el firmante de arriba: {} frente a {}",
            signers_in(&tree),
            signers_in(&leafs)
        );
    }
}
