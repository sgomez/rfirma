//! Pruebas de integración de PAdES contra el token: validador, colocación del recuadro y prefirma en seco (ADR-0001, ADR-0014, ADR-0016).

#[path = "native_cycle/support.rs"]
mod support;

mod full_cycle {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rfirma_lib::signing::adapters::ffi::NativeBridge;
    use rfirma_lib::signing::adapters::orders::SigningOrder;
    use rfirma_lib::signing::application::cycle::{self, SigningRequest};
    use rfirma_lib::signing::application::session::config_for;
    use rfirma_lib::signing::domain::bridge::{
        BridgeError, Format, SignatureOperation, SignatureVerdict, ValidationRequest, XadesVariant,
    };
    use rfirma_lib::signing::domain::{AdmissibleDocument, PadesRect, Placement, SignatureConfig};

    use base64::Engine;

    use super::support::{
        a_black_rubric, a_config_of, a_one_page_pdf, bridge, reference, signing_certificate,
        write_to_target, BOX_BOTTOM, BOX_LEFT, BOX_RIGHT, BOX_TOP, PAGE_HEIGHT, PAGE_WIDTH, PIN,
    };

    /// Ciclo trifásico completo contra el token (ADR-0001).
    fn sign(pdf: &[u8], config: &SignatureConfig) -> Vec<u8> {
        let bridge = bridge();
        let certificate = signing_certificate();
        let chain = vec![certificate.der().to_vec()];
        let reference = reference();

        let cycle = cycle::presign(
            &bridge,
            SigningRequest {
                format: Format::Pades,
                algorithm: cycle::ALGORITHM,
                operation: SignatureOperation::Sign,
                document: AdmissibleDocument::check(pdf).expect("el PDF generado es admisible"),
                chain: &chain,
                config,
                from_the_site: &cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )
        .expect("la prefirma deberia salir");

        let signature = cycle
            .sign_on_token(&rfirma_lib::identity::adapters::pkcs11::RealToken, PIN)
            .expect("el token debería firmar los atributos");

        cycle
            .postsign(&bridge, signature, &cycle.seal_in_transit())
            .expect("la postfirma deberia ensamblar el PDF")
            .into_signed_document()
    }

    fn verdict_of(bridge: &NativeBridge, document: &[u8]) -> SignatureVerdict {
        bridge
            .validate_signatures(ValidationRequest {
                document_b64: &base64::engine::general_purpose::STANDARD.encode(document),
                format: Format::Pades,
            })
            .expect("el validador tiene que contestar desde dentro de la imagen")
    }

    /// La version del encabezado entra en el `/ByteRange`: el resumen deja de cuadrar.
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

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_validator_of_the_original_survives_inside_the_native_image() {
        let bridge = bridge();
        let pdf = a_one_page_pdf();

        assert_eq!(
            verdict_of(&bridge, &pdf),
            SignatureVerdict::Unsigned,
            "un documento sin firmas cruza como tal"
        );

        let signed = sign(&pdf, &a_config_of("", None));
        assert_eq!(
            verdict_of(&bridge, &signed),
            SignatureVerdict::Valid,
            "y el que acaba de firmar el ciclo trifasico, tambien"
        );

        let altered = with_the_signed_bytes_altered(&signed);
        assert!(
            matches!(
                verdict_of(&bridge, &altered),
                SignatureVerdict::Invalid { .. }
            ),
            "una firma que ya no cuadra con el documento cruza como invalida"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn a_format_without_a_validator_in_the_original_is_not_validated_either() {
        let bridge = bridge();

        for format in [Format::CadesAsicS, Format::Xades(XadesVariant::AsicS)] {
            let refused = bridge
                .validate_signatures(ValidationRequest {
                    document_b64: "",
                    format,
                })
                .expect_err("el contenedor ASiC-S no tiene validador propio en el original");

            assert!(
                matches!(refused, BridgeError::FormatNotBridged(_)),
                "{refused}"
            );
        }
    }

    /// Valida la firma con pdfsig (ADR-0014).
    fn pdfsig(pdf: &Path) -> String {
        let output = Command::new("pdfsig")
            .arg(pdf)
            .output()
            .unwrap_or_else(|error| {
                panic!(
                    "falta pdfsig: es la puerta de validez de la grada C (ADR-0014).\n  \
                     sudo apt install -y poppler-utils\n{error}"
                )
            });
        let report = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "pdfsig ha fallado:\n{report}");
        report
    }

    /// Rasteriza la primera página a 72 ppp.
    fn rasterise(pdf: &Path) -> image::GrayImage {
        let prefix = pdf.with_extension("");
        let output = Command::new("pdftoppm")
            .args(["-png", "-r", "72", "-f", "1", "-l", "1"])
            .arg(pdf)
            .arg(&prefix)
            .output()
            .unwrap_or_else(|error| {
                panic!(
                    "falta pdftoppm: la rubrica se comprueba rasterizando (TD-03).\n  \
                     sudo apt install -y poppler-utils\n{error}"
                )
            });
        assert!(
            output.status.success(),
            "pdftoppm ha fallado:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let page = format!("{}-1.png", prefix.display());
        let image = image::open(&page)
            .unwrap_or_else(|error| panic!("no se ha podido leer {page}: {error}"))
            .to_luma8();

        assert_eq!(
            image.dimensions(),
            (PAGE_WIDTH, PAGE_HEIGHT),
            "pdftoppm ha rasterizado a otra escala: los recortes del recuadro dejarian \
             de medir el recuadro entero"
        );

        image
    }

    /// Cuenta píxeles oscuros dentro del recuadro de firma.
    fn dark_pixels_in_the_signature_box(page: &image::GrayImage) -> u32 {
        let (top_row, bottom_row) = (PAGE_HEIGHT - BOX_TOP, PAGE_HEIGHT - BOX_BOTTOM);
        let mut dark = 0;
        for y in top_row..bottom_row.min(page.height()) {
            for x in BOX_LEFT..BOX_RIGHT.min(page.width()) {
                if page.get_pixel(x, y).0[0] < 128 {
                    dark += 1;
                }
            }
        }
        dark
    }

    /// Salida esperada de pdfsig para una firma válida.
    const VALID: &str = "Signature Validation: Signature is Valid.";

    /// Nombre del PDF generado para la puerta manual.
    const MANUAL_GATE_PDF: &str = "manual-gate.pdf";

    /// Firma el PDF y devuelve la ruta y la página rasterizada.
    fn signed_page(name: &str, config: &SignatureConfig) -> (PathBuf, image::GrayImage) {
        let pdf = a_one_page_pdf();

        let before = rasterise(&write_to_target(&format!("before-{name}"), &pdf));
        assert_eq!(
            dark_pixels_in_the_signature_box(&before),
            0,
            "el recuadro no estaba vacio antes de firmar: la prueba no mide nada"
        );

        let signed = sign(&pdf, config);
        assert!(
            signed.len() > pdf.len(),
            "el PDF firmado tiene que crecer: {} contra {}",
            signed.len(),
            pdf.len()
        );

        let path = write_to_target(name, &signed);
        let report = pdfsig(&path);
        assert!(
            report.contains(VALID),
            "pdfsig no da la firma por valida:\n{report}"
        );

        let page = rasterise(&path);
        (path, page)
    }

    /// Prefirma en seco sin PIN con firma inventada.
    fn dry_run(pdf: &[u8], config: &SignatureConfig) -> Vec<u8> {
        let bridge = bridge();
        let certificate = signing_certificate();
        let chain = vec![certificate.der().to_vec()];
        let reference = reference();

        let cycle = cycle::presign(
            &bridge,
            SigningRequest {
                format: Format::Pades,
                algorithm: cycle::ALGORITHM,
                operation: SignatureOperation::Sign,
                document: AdmissibleDocument::check(pdf).expect("el PDF generado es admisible"),
                chain: &chain,
                config,
                from_the_site: &cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )
        .expect("la prefirma en seco deberia salir");

        cycle
            .postsign(
                &bridge,
                cycle.invented_signatures(),
                &cycle.seal_in_transit(),
            )
            .expect("la postfirma deberia componer el PDF con el PK1 inventado")
            .into_signed_document()
    }

    /// Área del recuadro en píxeles.
    fn box_area() -> u32 {
        (BOX_RIGHT - BOX_LEFT) * (BOX_TOP - BOX_BOTTOM)
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_signature_with_neither_text_nor_rubric_leaves_the_box_empty() {
        let (_, page) = signed_page("cycle-bare.pdf", &a_config_of("", None));

        assert_eq!(
            dark_pixels_in_the_signature_box(&page),
            0,
            "sin texto y sin rubrica el recuadro tiene que salir vacio: \
             si hay tinta, alguien la ha inyectado por omision"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_signature_with_only_text_puts_ink_in_the_box_but_does_not_fill_it() {
        let (_, page) = signed_page(
            "cycle-text-only.pdf",
            &a_config_of("Firmado por: PRUEBAS FNMT", None),
        );

        let dark = dark_pixels_in_the_signature_box(&page);
        let area = box_area();
        assert!(
            dark > 0,
            "el texto del recuadro no ha llegado al PDF: {dark} pixeles oscuros de {area}"
        );
        assert!(
            dark < area / 2,
            "unas letras no pueden ennegrecer medio recuadro: {dark} de {area}. \
             O el texto es enorme, o lo que hay dentro no es texto"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_signature_with_only_a_rubric_fills_the_box() {
        let (_, page) = signed_page(
            "cycle-rubric-only.pdf",
            &a_config_of("", Some(a_black_rubric())),
        );

        let dark = dark_pixels_in_the_signature_box(&page);
        let area = box_area();
        assert!(
            dark > area / 2,
            "el recuadro esta practicamente vacio: {dark} pixeles oscuros de {area}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_signature_with_text_and_rubric_is_the_pdf_of_the_manual_gate() {
        let (path, page) = signed_page(
            MANUAL_GATE_PDF,
            &a_config_of("Firmado por: PRUEBAS FNMT", Some(a_black_rubric())),
        );

        let dark = dark_pixels_in_the_signature_box(&page);
        let area = box_area();
        assert!(
            dark > area / 2,
            "el recuadro esta practicamente vacio: {dark} pixeles oscuros de {area}"
        );

        println!(
            "PDF de la puerta manual del validador oficial: {}",
            path.display()
        );
        assert!(
            path.is_file(),
            "el PDF de la puerta manual tiene que quedar en disco"
        );
    }

    /// La configuración que el caso de uso compone para una orden por modelo, en el recuadro de prueba.
    fn a_config_by_model(content: serde_json::Value, with_rubric: bool) -> SignatureConfig {
        let order: SigningOrder = serde_json::from_value(serde_json::json!({
            "document": "/run/user/1000/doc/1e8b83b9/contrato.pdf",
            "certificate": "FIRMA",
            "placement": null,
            "content": content,
            "withRubric": with_rubric,
            "signedAt": "31/08/26, 12:00:00",
            "rubric": a_black_rubric(),
            "language": "es",
        }))
        .expect("la orden por modelo se acepta");
        let choice = order
            .choice()
            .expect("sin recuadro no hay nada que validar");
        let config = config_for(&choice, &signing_certificate()).expect("sin recuadro cabe");
        SignatureConfig {
            placement: a_config_of("", None).placement,
            ..config
        }
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn the_complete_model_puts_text_and_no_rubric_in_the_box() {
        let config = a_config_by_model(serde_json::json!({ "model": "complete" }), false);
        assert_eq!(config.rubric_image, None);

        let (_, page) = signed_page("cycle-model-complete.pdf", &config);

        let dark = dark_pixels_in_the_signature_box(&page);
        let area = box_area();
        assert!(
            dark > 0 && dark < area / 2,
            "Completa es texto sin rubrica: {dark} pixeles oscuros de {area}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn the_rubric_only_model_fills_the_box_with_the_image() {
        let config = a_config_by_model(serde_json::json!({ "model": "rubricOnly" }), false);
        assert_eq!(config.layer2_text, "");

        let (_, page) = signed_page("cycle-model-rubric-only.pdf", &config);

        let dark = dark_pixels_in_the_signature_box(&page);
        let area = box_area();
        assert!(
            dark > area / 2,
            "Solo rubrica tiene que llenar el recuadro: {dark} pixeles oscuros de {area}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn the_custom_model_with_rubric_signs_the_phrase_beside_the_image() {
        let phrase = serde_json::json!([
            { "text": "Conforme: " },
            { "datum": "signer" },
            { "text": ", " },
            { "datum": "signedAt" },
        ]);
        let config = a_config_by_model(
            serde_json::json!({ "model": "custom", "phrase": phrase }),
            true,
        );
        assert!(config.layer2_text.starts_with("Conforme: "));
        assert!(config.rubric_image.is_some());

        let (_, page) = signed_page("cycle-model-custom.pdf", &config);

        let dark = dark_pixels_in_the_signature_box(&page);
        let area = box_area();
        assert!(
            dark > area / 2,
            "la rubrica de Personalizada no ha llegado al recuadro: {dark} de {area}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_pdf_that_was_already_signed_is_cosigned_and_pdfsig_validates_both() {
        let first = sign(
            &a_one_page_pdf(),
            &a_config_of("Firmado por: PRUEBAS FNMT", None),
        );
        assert!(
            AdmissibleDocument::check(&first)
                .expect("un PDF firmado se puede cofirmar")
                .already_signed(),
            "el PDF ya firmado tiene que reconocerse como tal"
        );

        let base = a_config_of("Firmado por: PRUEBAS FNMT", None);
        let placed = base
            .placement
            .clone()
            .expect("el caso local coloca el recuadro");
        let lower = SignatureConfig {
            placement: Some(Placement {
                rect: PadesRect {
                    lower_left_y: BOX_BOTTOM as i32 - 150,
                    upper_right_y: BOX_TOP as i32 - 150,
                    ..placed.rect
                },
                ..placed
            }),
            ..base
        };
        let second = sign(&first, &lower);

        let report = pdfsig(&write_to_target("cycle-cosigned.pdf", &second));
        let valid = report.matches(VALID).count();
        assert_eq!(
            valid, 2,
            "la cofirma tiene que dejar las DOS firmas validas:\n{report}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_dry_run_paints_the_very_box_the_real_signature_paints() {
        let pdf = a_one_page_pdf();
        let config = a_config_of("rfirma\nla vista previa", Some(a_black_rubric()));

        let signed = sign(&pdf, &config);
        let previewed = dry_run(&pdf, &config);

        let signed_page = rasterise(&write_to_target("signed-against-preview.pdf", &signed));
        let previewed_page = rasterise(&write_to_target("previewed.pdf", &previewed));

        assert_eq!(
            dark_pixels_in_the_signature_box(&previewed_page),
            dark_pixels_in_the_signature_box(&signed_page),
            "la vista previa pinta otra cantidad de tinta que la firma de verdad"
        );
        assert_eq!(
            previewed_page.as_raw(),
            signed_page.as_raw(),
            "la pagina compuesta en seco no se ve igual que la firmada: la ventana estaria enseñando lo que el PDF no va a tener"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_dry_run_composes_a_pdf_without_ever_asking_for_the_pin() {
        let pdf = a_one_page_pdf();

        let previewed = dry_run(&pdf, &a_config_of("rfirma: sin PIN", None));

        assert!(
            previewed.len() > pdf.len(),
            "el PDF compuesto tiene que crecer: {} contra {}",
            previewed.len(),
            pdf.len()
        );
        assert!(
            previewed.starts_with(b"%PDF-"),
            "lo que vuelve de la prefirma en seco tiene que ser un PDF"
        );
    }
}
