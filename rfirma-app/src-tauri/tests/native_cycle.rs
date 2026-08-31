//! **Grada C** (ADR-0014, TD-02): lo que solo se puede comprobar con
//! `librfirma_crypto.so` delante.
//!
//! Todas las pruebas de este fichero van marcadas `#[ignore]`: el carril rápido
//! del CI **las compila y no las ejecuta**, y el lento las corre con
//! `--include-ignored` (`just test-native`, que exporta `RFIRMA_LIB_DIR`
//! apuntando a la ruta canónica del ADR-0013). Compilarlas no es un detalle: es
//! la mitad de la TD-02, lo único que impide que una prueba que ha dejado de
//! compilar contra la FFI se salte en silencio para siempre.
//!
//! El ADR-0014 llama a este fichero `tests/ciclo_nativo.rs`; se escribe
//! `native_cycle.rs` porque el `CLAUDE.md` del repositorio pide **identificador
//! en inglés y prosa en castellano**, y un nombre de fichero es identificador.
//!
//! Aquí **no se firma un PDF**: eso es la grada C del puente Java, que valida
//! con `pdfsig`. Lo que se comprueba aquí es la frontera —que la librería carga
//! desde donde el ADR-0004 dice, que el JSON del contrato vuelve entero, y que
//! el ciclo de vida de la memoria del ID-11 se sostiene repetido cien mil
//! veces—, que es lo que no se puede probar con dobles.

use std::path::{Path, PathBuf};

use rfirma_lib::ffi::{
    locate, BridgeError, NativeBridge, PostSignRequest, PreSignRequest, LIBRARY_FILE,
};
use rfirma_lib::signing::SessionSeal;

/// Un PDF mínimo, en Base64, que **no** es un PDF válido para firmar.
///
/// Sirve para el camino de error: cruzar la frontera, que Java falle, y que el
/// fallo vuelva como JSON y se libere. No hace falta un PDF de verdad para
/// medir eso, y traerse uno arrastraría el material de prueba del puente.
const NOT_A_PDF_B64: &str = "bm8gc295IHVuIFBERg==";

/// Un certificado que tampoco lo es, por la misma razón.
const NOT_A_CERTIFICATE_B64: &str = "bm8gc295IHVuIGNlcnRpZmljYWRv";

/// Y un PKCS#1 que tampoco.
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

/// La memoria residente del proceso, en bytes. Es la instrumentación con la que
/// se mide la fuga: no hace falta más, porque lo que se busca no es un byte de
/// más sino un JSON entero por llamada.
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

/// La otra mitad del contrato: la postfirma cruza igual que la prefirma y
/// vuelve igual, por JSON y no por un aborto.
///
/// Aquí tampoco se firma un PDF —eso es la grada C del puente Java, que valida
/// con `pdfsig`—: lo que se comprueba es que la segunda entrada existe, que se
/// le pueden pasar sus cinco argumentos y que el fallo vuelve entero.
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

/// El ID-11, medido: si Rust dejase de llamar a `autofirma_free_string`, cada
/// vuelta se quedaría el JSON del puente en el C-heap.
///
/// Se comparan **dos tandas iguales**, no el principio y el final: la primera
/// paga el arranque del isolate, las clases que Java carga la primera vez y el
/// crecimiento natural del montón, y compararla con nada daría un falso
/// positivo. Con una fuga, la segunda tanda crece como la primera; sin ella, se
/// queda plana.
#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_hundred_thousand_round_trips_do_not_leak_the_json_of_the_bridge() {
    // Cien mil, y no diez mil, porque la tolerancia tiene que quedar lejos de
    // lo que mide: lo que se filtraría por vuelta es el JSON de error de Java
    // —unos 100-200 bytes—, así que con diez mil vueltas una fuga real caería
    // en 1-2 MiB, del mismo orden que el ruido que hay que tolerar. Con cien
    // mil son 10-20 MiB contra un margen de 1 MiB, y la prueba deja de
    // depender de cuánto mida el mensaje de excepción del día. Cuesta menos de
    // dos segundos: las veinte mil vueltas de antes tardaban 0,18 s.
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

/// **El ciclo trifásico entero**, que es lo que el #60 tenía que construir: la
/// prefirma cruza la frontera, el token firma, la postfirma ensambla, y
/// `pdfsig` de poppler dice si eso vale (TD-03, ADR-0014).
///
/// Necesita las **dos** cosas a la vez —`librfirma_crypto.so` y el token
/// SoftHSM—, que es exactamente por lo que estas pruebas viven aquí y no en
/// `pkcs11_token.rs`: aquellas son grada B y corren en el carril rápido, y
/// estas son grada C y corren en `just test-native`.
///
/// # La rúbrica se comprueba **rasterizando**
///
/// `pdftotext` no ve una imagen, así que preguntarle por la rúbrica da un
/// **falso negativo** —y, peor, un falso verde si la prueba se escribe al
/// revés—. Aquí se pinta la página con `pdftoppm` y se cuentan los píxeles
/// oscuros dentro del recuadro, comparándolos con los que había antes de
/// firmar. Es la comprobación que el TD-03 pide, y la única que distingue «la
/// rúbrica está en el PDF» de «el recuadro está vacío».
mod full_cycle {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rfirma_lib::pkcs11::{self, CertificateRef, TokenCertificate};
    use rfirma_lib::rubric;
    use rfirma_lib::signing::{
        cycle, AdmissibleDocument, SessionSeal, SignatureBox, SignatureConfig, SigningRequest,
    };

    use super::bridge;

    const TOKEN: &str = "rfirma-test";
    const PIN: &str = "1234";
    /// El único del kit con clave privada, así que el único con el que se firma.
    const ACTIVE: &str = "FNMT-ACTIVO-99999999R";

    /// La página, en puntos. A 72 ppp un punto es un píxel, y eso es lo que
    /// hace que las coordenadas del recuadro y las del PNG rasterizado sean las
    /// mismas sin conversiones que puedan estar mal las dos.
    const PAGE_WIDTH: u32 = 595;
    const PAGE_HEIGHT: u32 = 842;

    /// Dónde cae el recuadro. Lejos del texto de la página, para que lo que se
    /// cuente dentro sea la rúbrica y no una letra.
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
        pkcs11::list_certificates(&module())
            .expect("no se ha podido listar el token")
            .into_iter()
            .find(|certificate| certificate.reference().label() == ACTIVE)
            .unwrap_or_else(|| {
                panic!("el token {TOKEN} no tiene {ACTIVE}. Montalo con: just token")
            })
    }

    fn reference() -> CertificateRef {
        CertificateRef::new(module(), TOKEN, ACTIVE)
    }

    /// Un PDF de una página, escrito a mano.
    ///
    /// Se genera en vez de guardarse como material de prueba porque las
    /// **posiciones** de la tabla de referencias cruzadas se calculan aquí: un
    /// fichero pegado con los offsets a mano se rompe callado en cuanto alguien
    /// le cambia una letra al texto.
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
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }

    /// Una rúbrica **negra y maciza**, en el JPEG sin perfil ICC que el puente
    /// exige (ADR-0012).
    ///
    /// Negra a propósito: lo que la prueba cuenta luego son píxeles oscuros
    /// dentro del recuadro, y una rúbrica con medios tonos convertiría el
    /// umbral en una discusión sobre la compresión JPEG.
    fn a_black_rubric() -> String {
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(200, 100, image::Rgb([0, 0, 0])))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");

        rubric::normalize(&png)
            .expect("una rubrica negra es normalizable")
            .to_base64()
    }

    fn a_config(rubric: Option<String>) -> SignatureConfig {
        SignatureConfig {
            signature_box: SignatureBox {
                page: 1,
                lower_left_x: BOX_LEFT as i32,
                lower_left_y: BOX_BOTTOM as i32,
                upper_right_x: BOX_RIGHT as i32,
                upper_right_y: BOX_TOP as i32,
            },
            layer2_text: "Firmado por: PRUEBAS FNMT".to_owned(),
            rubric_image: rubric,
            sign_reason: None,
        }
    }

    /// El ciclo entero, en el orden del ADR-0001, y con la fase 2 en el token.
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

    /// `pdfsig` de poppler: **la puerta automática de validez** del ADR-0014.
    ///
    /// Lo que se comprueba es que la firma es criptográficamente válida, no que
    /// el certificado encadene: sin la CA de la FNMT en el almacén, `pdfsig` la
    /// dará siempre por no verificada, y afirmar lo contrario sería una prueba
    /// que miente. El validador oficial sigue siendo una puerta manual.
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

    /// La primera página, pintada a 72 ppp: un punto del PDF, un píxel.
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
        image::open(&page)
            .unwrap_or_else(|error| panic!("no se ha podido leer {page}: {error}"))
            .to_luma8()
    }

    /// Cuántos píxeles oscuros hay dentro del recuadro de la firma.
    ///
    /// El eje Y del PDF crece hacia arriba y el de la imagen hacia abajo, así
    /// que la fila se cuenta desde el alto de la página. Equivocarse aquí daría
    /// una prueba que mira una franja en blanco y no se entera.
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

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn the_whole_cycle_signs_a_pdf_and_pdfsig_validates_it() {
        let pdf = a_one_page_pdf();

        let signed = sign(&pdf, &a_config(None));

        assert!(
            signed.len() > pdf.len(),
            "el PDF firmado tiene que crecer: {} contra {}",
            signed.len(),
            pdf.len()
        );
        let report = pdfsig(&write_to_target("cycle-without-rubric.pdf", &signed));
        assert!(
            report.contains("Signature Validation: Signature is Valid."),
            "pdfsig no da la firma por valida:\n{report}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn the_rubric_is_there_when_the_page_is_rasterised() {
        // `pdftotext` no ve una imagen: preguntarle por la rubrica daria un
        // falso negativo, y por eso el TD-03 manda rasterizar.
        let pdf = a_one_page_pdf();
        let before = rasterise(&write_to_target("cycle-before-rubric.pdf", &pdf));
        assert_eq!(
            dark_pixels_in_the_signature_box(&before),
            0,
            "el recuadro tiene que estar vacio antes de firmar, o la prueba no mide nada"
        );

        let signed = sign(&pdf, &a_config(Some(a_black_rubric())));

        let page = rasterise(&write_to_target("cycle-with-rubric.pdf", &signed));
        let dark = dark_pixels_in_the_signature_box(&page);
        let area = (BOX_RIGHT - BOX_LEFT) * (BOX_TOP - BOX_BOTTOM);
        assert!(
            dark > area / 2,
            "el recuadro esta practicamente vacio: {dark} pixeles oscuros de {area}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_pdf_that_was_already_signed_is_cosigned_and_pdfsig_validates_both() {
        let first = sign(&a_one_page_pdf(), &a_config(None));
        assert!(
            AdmissibleDocument::check(&first)
                .expect("un PDF firmado se puede cofirmar")
                .already_signed(),
            "el PDF ya firmado tiene que reconocerse como tal"
        );

        // El segundo recuadro va más abajo: dos firmas visibles en el mismo
        // sitio se taparían, y lo que se quiere ver es que las dos están.
        let lower = SignatureConfig {
            signature_box: SignatureBox {
                lower_left_y: BOX_BOTTOM as i32 - 150,
                upper_right_y: BOX_TOP as i32 - 150,
                ..a_config(None).signature_box
            },
            ..a_config(None)
        };
        let second = sign(&first, &lower);

        let report = pdfsig(&write_to_target("cycle-cosigned.pdf", &second));
        let valid = report
            .matches("Signature Validation: Signature is Valid.")
            .count();
        assert_eq!(
            valid, 2,
            "la cofirma tiene que dejar las DOS firmas validas:\n{report}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_seal_altered_between_the_presign_and_the_postsign_makes_the_postsign_fail() {
        // La invariante del ADR-0016, medida de punta a punta: sin ella la
        // postfirma **completa sin error** y el PDF sale con `Digest Mismatch`.
        // Un fallo visible es lo unico que separa eso de una firma invalida que
        // nadie sabe por que lo es.
        let bridge = bridge();
        let pdf = a_one_page_pdf();
        let certificate = signing_certificate();
        let chain = vec![certificate.der().to_vec()];
        let reference = reference();
        let config = a_config(None);

        let cycle = cycle::presign(
            &bridge,
            SigningRequest {
                document: AdmissibleDocument::check(&pdf).expect("es admisible"),
                chain: &chain,
                config: &config,
                certificate: &reference,
            },
        )
        .expect("la prefirma deberia salir");
        let signature = cycle.sign_on_token(PIN).expect("el token deberia firmar");

        // El sello se altera **sin leerlo**: se le pega un byte al final, que es
        // todo lo que hace falta para que deje de ser el mismo.
        let tampered = SessionSeal::from_bridge(format!(
            "{}\u{0}",
            cycle.seal_in_transit().as_bridge_payload()
        ));

        let outcome = cycle.postsign(&bridge, &signature, &tampered);

        assert!(
            matches!(outcome, Err(cycle::CycleError::Seal(_))),
            "la postfirma tenia que abortar por el sello, y ha contestado {outcome:?}"
        );
    }
}
