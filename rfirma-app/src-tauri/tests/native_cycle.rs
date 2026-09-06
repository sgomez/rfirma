//! Pruebas de integración de grada C con la biblioteca nativa y el ciclo completo (ADR-0014).

use std::path::{Path, PathBuf};

use rfirma_lib::signing::adapters::ffi::{
    locate, BridgeError, NativeBridge, PostSignRequest, PreSignRequest, LIBRARY_FILE,
};
use rfirma_lib::signing::domain::SessionSeal;

/// Un PDF mínimo en Base64 no válido para firmar.
const NOT_A_PDF_B64: &str = "bm8gc295IHVuIFBERg==";

/// Un certificado sintético no válido.
const NOT_A_CERTIFICATE_B64: &str = "bm8gc295IHVuIGNlcnRpZmljYWRv";

/// Una firma sintética no válida.
const NOT_A_SIGNATURE_B64: &str = "bm8gc295IHVuYSBmaXJtYQ==";

fn library() -> PathBuf {
    let executable = std::env::current_exe().expect("debería haber ejecutable");
    let directory = executable.parent().unwrap_or(Path::new(".")).to_path_buf();
    locate(&|name| std::env::var_os(name), &directory).unwrap_or_else(|error| {
        panic!("{error}\n\nejecuta 'just test-native', que exporta RFIRMA_LIB_DIR")
    })
}

fn bridge() -> NativeBridge {
    NativeBridge::open_at(&library()).expect("la librería debería cargarse")
}

fn presign_of_something_invalid(bridge: &NativeBridge) -> Result<(), BridgeError> {
    bridge
        .presign(PreSignRequest {
            pdf_b64: NOT_A_PDF_B64,
            algorithm: "SHA256withRSA",
            certificate_chain_b64: NOT_A_CERTIFICATE_B64,
            extra_params: "signaturePage=1\n",
        })
        .map(|_| ())
}

fn postsign_of_something_invalid(bridge: &NativeBridge) -> Result<(), BridgeError> {
    let stamp = SessionSeal::from_bridge("bm8gc295IHVuIHNlbGxv");
    bridge
        .postsign(PostSignRequest {
            pdf_b64: NOT_A_PDF_B64,
            certificate_chain_b64: NOT_A_CERTIFICATE_B64,
            stamp: &stamp,
            session: "<xml/>",
            pkcs1_b64: NOT_A_SIGNATURE_B64,
        })
        .map(|_| ())
}

/// Memoria residente del proceso en bytes.
fn resident_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("debería haber /proc");
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .expect("statm trae al menos dos campos")
        .parse()
        .expect("es un número");
    pages * 4096
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_library_loads_from_where_the_adr_says_and_creates_its_isolate() {
    let bridge = bridge();

    assert!(bridge.path().ends_with(LIBRARY_FILE));
    assert!(bridge.path().is_file());
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_bridge_failure_comes_back_as_json_and_not_as_a_crash() {
    let bridge = bridge();

    let error = presign_of_something_invalid(&bridge).expect_err("eso no es un PDF firmable");

    match error {
        BridgeError::Failed(detail) => assert!(
            detail.contains("Exception") || detail.contains("Error"),
            "el detalle crudo de Java tiene que llegar entero: {detail}"
        ),
        other => panic!("se esperaba un fallo del puente, no {other}"),
    }
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_postsign_crosses_the_border_and_comes_back_as_json_too() {
    let bridge = bridge();

    let error = postsign_of_something_invalid(&bridge).expect_err("eso no es una postfirma");

    match error {
        BridgeError::Failed(detail) => assert!(
            detail.contains("Exception") || detail.contains("Error"),
            "el detalle crudo de Java tiene que llegar entero: {detail}"
        ),
        other => panic!("se esperaba un fallo del puente, no {other}"),
    }
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_hundred_thousand_round_trips_do_not_leak_the_json_of_the_bridge() {
    const BATCH: usize = 100_000;
    const TOLERANCE: u64 = 1024 * 1024;

    let bridge = bridge();

    for _ in 0..BATCH {
        let _ = presign_of_something_invalid(&bridge);
    }
    let after_first_batch = resident_bytes();
    for _ in 0..BATCH {
        let _ = presign_of_something_invalid(&bridge);
    }
    let after_second_batch = resident_bytes();

    let growth = after_second_batch.saturating_sub(after_first_batch);
    assert!(
        growth < TOLERANCE,
        "la segunda tanda de {BATCH} vueltas ha crecido {growth} bytes: \
         alguien ha dejado de llamar a autofirma_free_string"
    );
}

/// Ciclo trifásico completo contra el token y validación con pdfsig (ADR-0001, ADR-0014).
mod full_cycle {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use base64::Engine;
    use rfirma_lib::documents::adapters::rubric;
    use rfirma_lib::identity::adapters::pkcs11::{self, CertificateRef, TokenCertificate};
    use rfirma_lib::signing::adapters::ffi::{
        BridgeError, ExpandRequest, FilterRequest, NativeBridge,
    };
    use rfirma_lib::signing::application::cycle::{self, SigningRequest};
    use rfirma_lib::signing::application::filtering;
    use rfirma_lib::signing::domain::{
        AdmissibleDocument, PadesRect, PageSet, Placement, SessionSeal, SignatureConfig,
    };
    use rfirma_lib::site::domain::protocol::site_filter;

    use super::bridge;

    const TOKEN: &str = "rfirma-test";
    const PIN: &str = "1234";
    /// Certificado activo del kit de pruebas.
    const ACTIVE: &str = "FNMT-ACTIVO-99999999R";

    /// Dimensiones de página en puntos (72 ppp).
    const PAGE_WIDTH: u32 = 595;
    const PAGE_HEIGHT: u32 = 842;

    const BOX_LEFT: u32 = 72;
    const BOX_BOTTOM: u32 = 500;
    const BOX_RIGHT: u32 = 272;
    const BOX_TOP: u32 = 600;

    fn module() -> PathBuf {
        let module = PathBuf::from(
            std::env::var("RFIRMA_PKCS11_MODULE")
                .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
        );
        assert!(
            module.is_file(),
            "falta el modulo PKCS#11 en {}. La grada C necesita SoftHSM:\n  \
             sudo apt install -y softhsm2 opensc\n  just token",
            module.display()
        );
        module
    }

    fn signing_certificate() -> TokenCertificate {
        pkcs11::list_certificates(module())
            .expect("no se ha podido listar el token")
            .into_iter()
            .find(|certificate| certificate.reference().label() == ACTIVE)
            .unwrap_or_else(|| {
                panic!("el token {TOKEN} no tiene {ACTIVE}. Montalo con: just token")
            })
    }

    fn reference() -> CertificateRef {
        signing_certificate().reference().clone()
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_filter_engine_survives_inside_the_native_image() {
        let bridge = bridge();
        let certificate = signing_certificate();
        let der = base64::engine::general_purpose::STANDARD.encode(certificate.der());

        let inside = bridge
            .filter_certificates(FilterRequest {
                filter_properties: "filters=subject.contains:EIDAS CERTIFICADO PRUEBAS\n",
                certificates_b64: &der,
            })
            .expect("el motor tiene que contestar desde dentro de la imagen");
        assert_eq!(inside, vec![0]);

        let outside = bridge
            .filter_certificates(FilterRequest {
                filter_properties: "filters=subject.contains:NO ESTA EN EL SUBJECT\n",
                certificates_b64: &der,
            })
            .expect("el motor tiene que contestar desde dentro de la imagen");
        assert!(
            outside.is_empty(),
            "la sede lo excluye y el listado sale vacio, no completo"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_policy_expander_survives_inside_the_native_image() {
        let bridge = bridge();

        let expanded = bridge
            .expand_extra_params(ExpandRequest {
                extra_params: "expPolicy=FirmaAGE\n",
                format: "PAdES",
            })
            .expect("el expansor tiene que contestar desde dentro de la imagen");

        assert!(
            !expanded.contains("expPolicy="),
            "la clave expandible se consume: {expanded}"
        );
        assert!(
            expanded.contains("policyIdentifier=urn:oid:"),
            "y el identificador sale del policy.properties de afirma-core: {expanded}"
        );
        assert!(
            expanded.contains("signatureSubFilter=ETSI.CAdES.detached"),
            "que es ademas el mismo subfiltro que rFirma envia siempre: {expanded}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn a_policy_that_does_not_fit_the_format_comes_back_named() {
        let bridge = bridge();

        let refused = bridge
            .expand_extra_params(ExpandRequest {
                extra_params: "expPolicy=PoliticaQueNoExiste\n",
                format: "PAdES",
            })
            .expect_err("esa politica no se puede aplicar");

        assert!(
            matches!(refused, BridgeError::IncompatiblePolicy(_)),
            "tenia que llegar con nombre propio: {refused:?}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_use_case_bounds_the_listing_through_the_real_bridge() {
        let bridge = bridge();
        let listing = vec![signing_certificate()];
        let filter = site_filter(&[(
            "filters".to_owned(),
            "subject.contains:EIDAS CERTIFICADO PRUEBAS".to_owned(),
        )])
        .expect("el criterio esta en la lista blanca");

        let kept = filtering::keep_what_the_site_accepts(&bridge, &filter, listing)
            .expect("el motor contesta");

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].reference().label(), ACTIVE);
    }

    /// Genera un PDF sintético de una página.
    fn a_one_page_pdf() -> Vec<u8> {
        let content = format!(
            "BT /F1 24 Tf 72 {} Td (rfirma: ciclo trifasico) Tj ET\n",
            PAGE_HEIGHT - 92
        );
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
                 /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            ),
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            ),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        ];

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
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n::EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }

    /// Rúbrica de prueba en JPEG normalizado (ADR-0012).
    fn a_black_rubric() -> String {
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            200,
            100,
            image::Rgb([0, 0, 0]),
        ))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("el PNG de prueba deberia codificarse");

        rubric::normalize(&png)
            .expect("una rubrica negra es normalizable")
            .to_base64()
    }

    /// Configuración de firma para el recuadro de prueba.
    fn a_config_of(text: &str, rubric: Option<String>) -> SignatureConfig {
        SignatureConfig {
            placement: Some(Placement {
                rect: PadesRect {
                    lower_left_x: BOX_LEFT as i32,
                    lower_left_y: BOX_BOTTOM as i32,
                    upper_right_x: BOX_RIGHT as i32,
                    upper_right_y: BOX_TOP as i32,
                },
                pages: PageSet::only_page(1),
            }),
            layer2_text: text.to_owned(),
            rubric_image: rubric,
            sign_reason: None,
            allow_unregistered_signatures: false,
        }
    }

    /// Ciclo trifásico completo contra el token (ADR-0001).
    fn sign(pdf: &[u8], config: &SignatureConfig) -> Vec<u8> {
        let bridge = bridge();
        let certificate = signing_certificate();
        let chain = vec![certificate.der().to_vec()];
        let reference = reference();

        let cycle = cycle::presign(
            &bridge,
            SigningRequest {
                document: AdmissibleDocument::check(pdf).expect("el PDF generado es admisible"),
                chain: &chain,
                config,
                from_the_site: &cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )
        .expect("la prefirma deberia salir");

        let signature = cycle
            .sign_on_token(PIN)
            .expect("el token deberia firmar los atributos");

        cycle
            .postsign(&bridge, &signature, &cycle.seal_in_transit())
            .expect("la postfirma deberia ensamblar el PDF")
    }

    fn write_to_target(name: &str, bytes: &[u8]) -> PathBuf {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        std::fs::write(&path, bytes).expect("deberia poder escribirse el PDF de la prueba");
        path
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
                &cycle::TokenSignature::invented(),
                &cycle.seal_in_transit(),
            )
            .expect("la postfirma deberia componer el PDF con el PK1 inventado")
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

    /// Decodifica el bloque de texto del sello (ADR-0016).
    fn inside_the_seal(seal: &SessionSeal) -> String {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(seal.as_bridge_payload())
            .expect("el sello del puente viene en Base64");
        String::from_utf8(raw).expect("el bloque del sello es UTF-8")
    }

    /// Re-codifica las líneas del sello preservando el salto final.
    fn re_encoded(lines: &[String]) -> String {
        base64::engine::general_purpose::STANDARD.encode(format!("{}\n", lines.join("\n")))
    }

    /// Altera el primer campo del sello que empiece por prefijo.
    fn seal_with_field_altered(seal: &SessionSeal, prefix: &str) -> SessionSeal {
        let block = inside_the_seal(seal);

        let untouched: Vec<String> = block.lines().map(str::to_owned).collect();
        assert_eq!(
            re_encoded(&untouched),
            seal.as_bridge_payload(),
            "reconstruir el sello sin mutar nada tiene que dar el mismo payload byte a \
             byte, o esta prueba pasaria por la reconstruccion y no por el campo \
             '{prefix}':\n{block}"
        );

        let mut altered = false;
        let lines: Vec<String> = block
            .lines()
            .map(|line| {
                if !altered && line.starts_with(prefix) {
                    altered = true;
                    format!("{line}-alterado")
                } else {
                    line.to_owned()
                }
            })
            .collect();
        assert!(
            altered,
            "el sello del puente ya no lleva ningun campo '{prefix}'. \
             Sin ese campo la invariante del ADR-0016 no esta sellada, \
             y esta prueba habria pasado sin comprobar nada:\n{block}"
        );
        SessionSeal::from_bridge(re_encoded(&lines))
    }

    /// Prepara un ciclo firmado en token listo para postfirma.
    fn a_cycle_ready_to_postsign() -> (NativeBridge, cycle::OpenCycle, cycle::TokenSignature) {
        let bridge = bridge();
        let pdf = a_one_page_pdf();
        let certificate = signing_certificate();
        let chain = vec![certificate.der().to_vec()];
        let reference = reference();
        let config = a_config_of("Firmado por: PRUEBAS FNMT", None);

        let cycle = cycle::presign(
            &bridge,
            SigningRequest {
                document: AdmissibleDocument::check(&pdf).expect("es admisible"),
                chain: &chain,
                config: &config,
                from_the_site: &cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )
        .expect("la prefirma deberia salir");
        let signature = cycle.sign_on_token(PIN).expect("el token deberia firmar");

        (bridge, cycle, signature)
    }

    /// Comprueba que postsign rechaza un sello con un campo alterado (ADR-0016).
    fn postsign_refuses_a_seal_altered_in(prefix: &str) {
        let (bridge, cycle, signature) = a_cycle_ready_to_postsign();

        let tampered = seal_with_field_altered(&cycle.seal_in_transit(), prefix);

        let outcome = cycle
            .postsign(&bridge, &signature, &tampered)
            .map(|pdf| format!("un PDF de {} bytes", pdf.len()));

        assert!(
            matches!(outcome, Err(cycle::CycleError::Seal(_))),
            "con el campo '{prefix}' alterado la postfirma tenia que abortar por el \
             sello, y ha contestado {outcome:?}. Un Ok aqui es un PDF con Digest \
             Mismatch que nadie sabe por que no vale"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn an_altered_signing_instant_makes_the_postsign_fail() {
        postsign_refuses_a_seal_altered_in("TIME=");
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn an_altered_time_zone_makes_the_postsign_fail() {
        postsign_refuses_a_seal_altered_in("TZ=");
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn altered_effective_extra_params_make_the_postsign_fail() {
        postsign_refuses_a_seal_altered_in("P.");
    }
}
