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
//! # Por qué este fichero es la excepción vertical al corte horizontal
//!
//! El resto del spec #46 está cortado **por módulo**: la rúbrica, el token, la
//! memoria, las coordenadas, la frontera FFI. Este fichero no. Necesita el
//! puente Java, la FFI, PKCS#11, el PDF y `pdfsig` **a la vez**, así que no
//! pertenece a ningún módulo del corte, y por eso tiene un sub-issue propio
//! (#61) en vez de repartirse entre los demás (TD-09, ADR-0014).
//!
//! **El corte no se rompió por descuido.** Se rompió porque sin dueño explícito
//! cada módulo habría supuesto que esta prueba la escribía otro, y no la habría
//! escrito nadie. Lo que se pierde al partirla es exactamente lo único que
//! demuestra que la versión sirve: que un PDF firmado por rFirma es un PDF
//! válido. Todo lo demás se puede probar con dobles; esto no.
//!
//! # Las dos mitades del fichero
//!
//! Arriba, **la frontera sola**: que la librería carga desde donde el ADR-0004
//! dice, que el JSON del contrato vuelve entero en vez de abortar el proceso, y
//! que el ciclo de vida de la memoria del ID-11 se sostiene repetido cien mil
//! veces. Abajo, en [`full_cycle`], **el ciclo trifásico entero** contra el
//! token, con `pdfsig` de poppler delante.
//!
//! # El PDF de la puerta manual
//!
//! El validador oficial es una **puerta manual de release**, no de CI: VALIDe
//! es red, web y sin API estable (TD-04, ADR-0014). El PDF que se le lleva es
//! el que produce
//! [`full_cycle::a_signature_with_text_and_rubric_is_the_pdf_of_the_manual_gate`],
//! el caso máximo —recuadro con texto **y** rúbrica—, y se deposita como
//! `manual-gate.pdf` dentro del `CARGO_TARGET_TMPDIR` de la prueba
//! (`rfirma-app/src-tauri/target/tmp/manual-gate.pdf` con el diseño de
//! directorios de cargo de hoy). La propia prueba imprime la ruta absoluta, y
//! el carril lento del CI lo sube como artefacto `pdf-puerta-manual` para que
//! quien etiquete una `v*` no tenga que reproducir el entorno para conseguirlo.
//! Un check en verde **no** demuestra lo que promete el criterio de terminado
//! del hito; este fichero produce el PDF, y una persona cierra la puerta.

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

/// **El ciclo trifásico entero**: la prefirma cruza la frontera, el token
/// firma, la postfirma ensambla, y `pdfsig` de poppler dice si eso vale
/// (TD-03, ADR-0014).
///
/// Necesita las **dos** cosas a la vez —`librfirma_crypto.so` y el token
/// SoftHSM—, que es exactamente por lo que estas pruebas viven aquí y no en
/// `pkcs11_token.rs`: aquellas son grada B y corren en el carril rápido, y
/// estas son grada C y corren en `just test-native`.
///
/// Dentro hay tres grupos: los **cuatro casos de firma visible** que la
/// librería nativa soporta —ni texto ni rúbrica, texto solo, rúbrica sola, y
/// las dos—, la **cofirma** sobre un PDF ya firmado, y las **tres invariantes
/// del sello** del ADR-0016, probadas como fallo esperado.
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

    use base64::Engine;
    use rfirma_lib::app::cycle::{self, SigningRequest};
    use rfirma_lib::app::filtering;
    use rfirma_lib::ffi::{BridgeError, ExpandRequest, FilterRequest, NativeBridge};
    use rfirma_lib::pkcs11::{self, CertificateRef, TokenCertificate};
    use rfirma_lib::protocol::site_filter;
    use rfirma_lib::rubric;
    use rfirma_lib::signing::{
        AdmissibleDocument, PadesRect, PageSet, Placement, SessionSeal, SignatureConfig,
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

    /// **La cuarta entrada, contra la librería de verdad** (ID-252, TD-56).
    ///
    /// Las pruebas de grada A del motor viven en el puente Java, donde se
    /// ejecuta sobre la JVM. Lo que sólo se puede comprobar aquí es que ese
    /// motor **sigue estando dentro de la imagen nativa**: `native-image`
    /// descarta lo que no alcanza, así que un `CertFilterManager` que se
    /// quedara fuera no daría error de compilación —daría una respuesta vacía o
    /// una excepción en tiempo de ejecución, con la sede delante—.
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

    /// **La quinta entrada, contra la librería de verdad** (ID-266).
    ///
    /// Por lo mismo que la del motor de filtros, y con un modo de fallo peor:
    /// `AdESPolicyPropertiesManager` lee `policy.properties` por
    /// `ResourceBundle`, que `native-image` **no alcanza solo**. Sin el
    /// `-H:IncludeResourceBundles=policy` de `native-image.properties` la
    /// expansión no revienta: devuelve las claves vacías, la firma se hace, y
    /// lo que sale no lleva la política que la sede declaró.
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

    /// Una política que no se puede aplicar llega **con nombre propio** hasta
    /// este lado, y no colapsada en «la firma no ha salido» (ID-266).
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

    /// Y el camino entero, con el puente de verdad haciendo de motor: los
    /// criterios de rFirma primero y la expresión de la sede después (ID-258).
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

    /// La configuracion de firma de un caso, con el mismo recuadro siempre.
    ///
    /// El texto y la rubrica van sueltos porque son **los dos ejes de los
    /// cuatro casos**: cada prueba de abajo fija uno y otro, y el recuadro se
    /// queda quieto para que las cuatro midan la misma region de la pagina.
    fn a_config_of(text: &str, rubric: Option<String>) -> SignatureConfig {
        SignatureConfig {
            placement: Placement {
                rect: PadesRect {
                    lower_left_x: BOX_LEFT as i32,
                    lower_left_y: BOX_BOTTOM as i32,
                    upper_right_x: BOX_RIGHT as i32,
                    upper_right_y: BOX_TOP as i32,
                },
                pages: PageSet::only_page(1),
            },
            layer2_text: text.to_owned(),
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
        let image = image::open(&page)
            .unwrap_or_else(|error| panic!("no se ha podido leer {page}: {error}"))
            .to_luma8();

        // A 72 ppp un punto del PDF es un píxel. Si algún día no lo fuera, los
        // recortes de `dark_pixels_in_the_signature_box` mirarían una franja
        // más pequeña que el recuadro y el caso vacío pasaría por la vía
        // trivial. Mejor un fallo ruidoso aquí que un recorte callado allí.
        assert_eq!(
            image.dimensions(),
            (PAGE_WIDTH, PAGE_HEIGHT),
            "pdftoppm ha rasterizado a otra escala: los recortes del recuadro dejarian \
             de medir el recuadro entero"
        );

        image
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

    /// Lo que `pdfsig` escribe cuando la firma es criptográficamente válida.
    const VALID: &str = "Signature Validation: Signature is Valid.";

    /// El PDF que va a la **puerta manual del validador oficial** (TD-04).
    ///
    /// Es el caso máximo —recuadro con texto **y** rúbrica—, porque es el que
    /// ejercita todo lo que la versión promete a la vez; si el validador
    /// oficial acepta este, los otros tres son subconjuntos suyos.
    const MANUAL_GATE_PDF: &str = "manual-gate.pdf";

    /// Firma, pasa `pdfsig` y devuelve la primera página pintada.
    ///
    /// Los cuatro casos de firma visible comparten esto porque lo que los
    /// distingue es **solo** lo que hay dentro del recuadro: la validez
    /// criptográfica se exige igual en los cuatro, y una prueba que la
    /// comprobara en unos sí y en otros no dejaría el agujero justo donde la
    /// firma visible cambia el PDF.
    fn signed_page(name: &str, config: &SignatureConfig) -> (PathBuf, image::GrayImage) {
        let pdf = a_one_page_pdf();

        // El recuadro tiene que estar vacío ANTES de firmar, o contar píxeles
        // oscuros después no mide nada.
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

    /// **La prefirma en seco** (ID-136): el mismo ciclo, con un `PK1` inventado
    /// y **sin PIN**.
    ///
    /// Que aquí no aparezca `sign_on_token` ni la constante `PIN` no es
    /// casualidad: es la mitad de la decisión que se puede leer. La otra mitad
    /// —que el sello pintado sea el mismo— la mide
    /// `the_dry_run_paints_the_very_box_the_real_signature_paints`.
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

    /// El área del recuadro en píxeles, que es contra lo que se compara la
    /// tinta que haya dentro.
    fn box_area() -> u32 {
        (BOX_RIGHT - BOX_LEFT) * (BOX_TOP - BOX_BOTTOM)
    }

    // ---------------------------------------------------------------------
    // Los cuatro casos de firma visible
    // ---------------------------------------------------------------------
    //
    // Son cuatro y no dos porque el texto y la rúbrica son dos ajustes
    // independientes (`layer2Text` y `signatureRubricImage`, ID-18) y la
    // librería nativa soporta las cuatro combinaciones. Los que se saltan sin
    // querer son siempre los mismos: el vacío, que es el que descubre que
    // `PdfSessionManager` inyecta su texto por omisión en cuanto falta la
    // clave, y el de rúbrica sola, que es el que descubre que el texto no
    // estaba pintando la rúbrica por él.
    //
    // Los cuatro se miden **rasterizando** (TD-03), y los cuatro miran el mismo
    // recuadro, así que entre ellos se distinguen: vacío, poca tinta, lleno,
    // lleno.

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_signature_with_neither_text_nor_rubric_leaves_the_box_empty() {
        // `layer2Text` viaja **vacío pero presente** a propósito (ID-18): si la
        // clave faltara y tampoco hubiera rúbrica, `PdfSessionManager` metería
        // su texto por omisión —castellano fijo y con comodines dentro— y este
        // caso dejaría de ser el caso vacío sin que nadie se enterase.
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
        // El techo es la otra mitad de la comprobación: unas letras dejan
        // bastante menos tinta que una rúbrica maciza, y sin este límite el
        // caso de texto y el de rúbrica pasarían con la misma medida.
        assert!(
            dark < area / 2,
            "unas letras no pueden ennegrecer medio recuadro: {dark} de {area}. \
             O el texto es enorme, o lo que hay dentro no es texto"
        );
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn a_signature_with_only_a_rubric_fills_the_box() {
        // `pdftotext` no ve una imagen: preguntarle por la rúbrica daría un
        // falso negativo, y por eso el TD-03 manda rasterizar. Este caso es el
        // que lo demuestra sin ayuda del texto —aquí no hay ni una letra que
        // pudiera estar dando el verde por ella—.
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

        // Se imprime la ruta, y no solo se escribe el fichero: la puerta del
        // TD-04 la ejecuta una persona, y una persona necesita saber de dónde
        // sacar el PDF. `cargo test -- --nocapture` (o el registro del carril
        // lento) lo dice.
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

        // El segundo recuadro va más abajo: dos firmas visibles en el mismo
        // sitio se taparían, y lo que se quiere ver es que las dos están.
        let base = a_config_of("Firmado por: PRUEBAS FNMT", None);
        let lower = SignatureConfig {
            placement: Placement {
                rect: PadesRect {
                    lower_left_y: BOX_BOTTOM as i32 - 150,
                    upper_right_y: BOX_TOP as i32 - 150,
                    ..base.placement.rect
                },
                ..base.placement.clone()
            },
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

    // ---------------------------------------------------------------------
    // Las tres invariantes del sello, como fallo esperado
    // ---------------------------------------------------------------------
    //
    // El ADR-0016 y el ID-17 dicen que la postfirma exige recibir de la
    // prefirma **lo mismo en tres cosas a la vez**: los `extraParams`
    // efectivos, el instante de firma y la zona horaria. Las tres se prueban
    // **como fallo esperado** y no se evitan: lo que las tres protegen es un
    // fallo que no se ve —la postfirma completa sin error y el PDF sale con
    // `Digest Mismatch`—, así que una prueba que se limitara a no alterarlas no
    // distinguiría el código con guarda del código sin ella.
    //
    // Con una salvedad que conviene no leer de más: las tres acaban en la misma
    // comparación del **bloque entero**, así que ninguna aísla causalidad por
    // campo. Lo que distingue a una de otra es su `assert!(altered)`, es decir,
    // «el puente sigue sellando `TIME` / `TZ` / `P.*`». Mientras `OpenCycle` no
    // exponga la sesión —y el ADR-0016 es justo lo que lo impide—, esta es la
    // cota máxima.
    //
    // Las tres son de grada C y no de grada A aunque `session_seal.rs` ya
    // compare bytes en el carril rápido, porque **lo que se altera aquí es el
    // sello de verdad**: el que acaba de componer `SessionStamp.encode` al otro
    // lado de la FFI, con los nombres de campo que el puente escribe hoy. Una
    // prueba de grada A los inventa, y seguiría verde el día que el puente
    // dejara de sellar la zona horaria.
    //
    // Y no las guarda Java: la postfirma toma del sello la zona horaria y los
    // `extraParams`, así que un sello alterado en esos dos campos le parece
    // bien y el PDF sale inválido. `SessionStamp.matchesSessionTime` cubre el
    // instante, y nada más. La comparación de bytes de Rust, **antes** de
    // cruzar, es la única guarda que tienen los otros dos.

    /// **Lo que la vista previa pinta es el sello, no un dibujo parecido**
    /// (ID-136).
    ///
    /// El sondeo del #115 midió que los bytes visibles del PDF compuesto con un
    /// `PK1` inventado son idénticos a los del firmado de verdad. Esto es esa
    /// medida convertida en puerta: se rasterizan las dos páginas a 72 ppp y se
    /// comparan **píxel a píxel**. Si algún día dejaran de coincidir, la
    /// ventana estaría enseñando dentro del recuadro algo que el PDF firmado no
    /// va a tener, y el hito entero cambia de forma.
    ///
    /// Se compara la página entera y no solo el recuadro a propósito: lo que se
    /// afirma es que la vista previa no altera nada de lo que se ve, ni dentro
    /// ni fuera.
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

    /// **La prefirma en seco no pide PIN y no escribe donde cae el firmado.**
    ///
    /// Que no pida PIN se ve en que este recorrido compone el PDF con el token
    /// **cerrado**: `dry_run` no llama a `sign_on_token` y aquí no hay
    /// contraseña ninguna. Que no toque el disco de destino se ve en que lo que
    /// vuelve son bytes: el único fichero que aparece es el que escribe la
    /// prueba para poder mirarlo.
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

    /// El bloque de texto que hay dentro del sello.
    ///
    /// **rFirma no lee el sello nunca** —el ADR-0016 lo prohíbe, y por eso
    /// `SessionSeal` no tiene ni un `get`—, pero la prueba sí, y a propósito:
    /// es lo único que permite alterar *una* invariante sin tocar las otras
    /// dos. Que este conocimiento viva aquí, en un fichero de pruebas, y no en
    /// el código de producción es justo la línea que el ADR traza.
    fn inside_the_seal(seal: &SessionSeal) -> String {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(seal.as_bridge_payload())
            .expect("el sello del puente viene en Base64");
        String::from_utf8(raw).expect("el bloque del sello es UTF-8")
    }

    /// Vuelve a componer el payload del sello a partir de sus líneas.
    ///
    /// El `\n` final no es un detalle de estilo: `SessionStamp.encode` cierra
    /// **cada** campo con un salto de línea, incluido el último, así que el
    /// bloque decodificado termina en `'\n'`. `str::lines()` no lo devuelve y
    /// `join("\n")` no lo repone. Sin reponerlo aquí, el sello reconstruido ya
    /// diferiría del original **sin haber mutado nada**, y como
    /// `SessionSeal::verify_unchanged` compara la cadena entera, las tres
    /// invariantes pasarían por el salto de línea en vez de por el campo: una
    /// mutación inerte las dejaría igual de verdes.
    fn re_encoded(lines: &[String]) -> String {
        base64::engine::general_purpose::STANDARD.encode(format!("{}\n", lines.join("\n")))
    }

    /// Devuelve el sello con el valor del **primer** campo cuya clave empiece
    /// por `prefix` cambiado, y **falla si no hay ninguno**.
    ///
    /// Ese `assert` es lo que separa esta prueba de una que no prueba nada: sin
    /// él, el día que el puente dejara de sellar el campo, la prueba alteraría
    /// un sello inexistente, el sello saldría idéntico... y el fallo esperado
    /// no llegaría.
    ///
    /// El segundo `assert` ancla la otra mitad de esa afirmación: reconstruir
    /// el sello **sin mutar nada** tiene que dar el payload original byte a
    /// byte. Con los dos, la única diferencia entre el sello que abrió el ciclo
    /// y el que llega a la postfirma es el campo que la prueba dice alterar.
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

    /// Abre un ciclo, firma en el token y devuelve las dos mitades que la
    /// postfirma necesita. Lo comparten las tres invariantes.
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

    /// Altera un campo del sello real y comprueba que la postfirma **aborta**
    /// en vez de devolver un PDF.
    fn postsign_refuses_a_seal_altered_in(prefix: &str) {
        let (bridge, cycle, signature) = a_cycle_ready_to_postsign();

        let tampered = seal_with_field_altered(&cycle.seal_in_transit(), prefix);

        // Del PDF solo se enseña el tamaño: si la postfirma devuelve uno, el
        // `Debug` de un `Vec<u8>` son medio megabyte de numeros en el registro
        // de la prueba, y lo que hace falta saber es que ha devuelto algo.
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
        // El `TIME` que la prefirma dejó en el `TriphaseData`. Entra dentro de
        // los atributos firmados, así que un segundo de diferencia invalida.
        postsign_refuses_a_seal_altered_in("TIME=");
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn an_altered_time_zone_makes_the_postsign_fail() {
        // El tercero, el que nadie esperaba (#23): el desfase de la zona entra
        // dentro del rango firmado, así que el mismo instante en otra zona
        // produce otros bytes. Y aquí Java no ayuda —toma la zona del propio
        // sello—, así que esta comparación es la única guarda que hay.
        postsign_refuses_a_seal_altered_in("TZ=");
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so y el token (just test-native)"]
    fn altered_effective_extra_params_make_the_postsign_fail() {
        // Los `extraParams` **efectivos**, no los enviados: Java muta el
        // `Properties` que recibe y `PAdESTriPhaseSigner:174` no lo clona, así
        // que el puente relee el objeto después de la prefirma y sella eso.
        // Van prefijados con `P.` dentro del bloque.
        postsign_refuses_a_seal_altered_in("P.");
    }
}
