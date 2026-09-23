//! Helpers compartidos por los ficheros de `native_cycle_*.rs`: puente, token y ciclo trifásico (ADR-0001, ADR-0014).

// Cada `native_cycle_*.rs` usa un subconjunto distinto: no todos se usan en todos.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::documents::adapters::rubric;
use rfirma_lib::identity::adapters::folder::RealInstalledFolder;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::application::certificates;
use rfirma_lib::identity::domain::algorithm::SignatureAlgorithm;
use rfirma_lib::identity::domain::certificate::{CertificateRef, TokenCertificate};
use rfirma_lib::signing::adapters::ffi::{locate, NativeBridge};
use rfirma_lib::signing::application::cycle::{self, SigningRequest};
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
use rfirma_lib::signing::domain::{
    AdmissibleDocument, PadesRect, PageSet, Placement, SignatureConfig, Waivers,
};
use rfirma_lib::site::adapters::desk::composed_for;
use rfirma_lib::site::domain::protocol::AskedAlgorithm;

pub(crate) const TOKEN: &str = "rfirma-test";
pub(crate) const PIN: &str = "1234";
/// Contraseña del `.p12` del kit de pruebas.
pub(crate) const KIT_PASSWORD: &str = "1234";
/// El almacén NSS de un `.p12` instalado no pide secreto (ADR-0011).
pub(crate) const NO_SECRET: &str = "";
/// Certificado activo del kit de pruebas.
pub(crate) const ACTIVE: &str = "FNMT-ACTIVO-99999999R";
/// El certificado de curva elíptica, en su propio token.
pub(crate) const ACTIVE_EC: &str = "FNMT-ACTIVO-ECC-99949991H";

/// Dimensiones de página en puntos (72 ppp).
pub(crate) const PAGE_WIDTH: u32 = 595;
pub(crate) const PAGE_HEIGHT: u32 = 842;

pub(crate) const BOX_LEFT: u32 = 72;
pub(crate) const BOX_BOTTOM: u32 = 500;
pub(crate) const BOX_RIGHT: u32 = 272;
pub(crate) const BOX_TOP: u32 = 600;

/// El reto que una sede manda firmar en CAdES: bytes, y ninguno de un PDF.
pub(crate) const CHALLENGE: &[u8] = b"un reto binario de la sede\x00\x01\x02";

pub(crate) fn library() -> PathBuf {
    let executable = std::env::current_exe().expect("debería haber ejecutable");
    let directory = executable.parent().unwrap_or(Path::new(".")).to_path_buf();
    locate(&|name| std::env::var_os(name), &directory).unwrap_or_else(|error| {
        panic!("{error}\n\nejecuta 'just test-native', que exporta RFIRMA_LIB_DIR")
    })
}

pub(crate) fn bridge() -> NativeBridge {
    NativeBridge::open_at(&library()).expect("la librería debería cargarse")
}

pub(crate) fn module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. La grada C necesita SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just certs install",
        module.display()
    );
    module
}

pub(crate) fn signing_certificate() -> TokenCertificate {
    certificate_labelled(ACTIVE)
}

pub(crate) fn certificate_labelled(label: &str) -> TokenCertificate {
    pkcs11::list_certificates(module())
        .expect("no se ha podido listar el token")
        .into_iter()
        .find(|certificate| certificate.reference().label() == label)
        .unwrap_or_else(|| {
            panic!("el token {TOKEN} no tiene {label}. Montalo con: just certs install")
        })
}

pub(crate) fn reference() -> CertificateRef {
    signing_certificate().reference().clone()
}

/// El algoritmo que sale de casar SHA-256 con la clave del certificado de curva elíptica.
pub(crate) fn ecdsa_composed_for_the_ec_certificate(
    certificate: &TokenCertificate,
) -> SignatureAlgorithm {
    let algorithm = composed_for(AskedAlgorithm::Sha256, certificate.key_kind());
    assert_eq!(algorithm, SignatureAlgorithm::Sha256Ecdsa);
    algorithm
}

/// Ciclo trifásico CAdES completo contra el token (ADR-0001).
pub(crate) fn sign_cades(data: &[u8], mode: &str) -> Vec<u8> {
    cades_cycle(data, SignatureOperation::Sign, &[("mode", mode)])
}

/// El mismo ciclo, con la operación y los `extraParams` que se le digan.
pub(crate) fn cades_cycle(
    data: &[u8],
    operation: SignatureOperation,
    declared: &[(&str, &str)],
) -> Vec<u8> {
    a_cycle_of(Format::Cades, cycle::ALGORITHM, data, operation, declared)
}

/// El mismo ciclo, para el formato y el algoritmo que la sede haya pedido.
pub(crate) fn a_cycle_of(
    format: Format,
    algorithm: SignatureAlgorithm,
    data: &[u8],
    operation: SignatureOperation,
    declared: &[(&str, &str)],
) -> Vec<u8> {
    a_cycle_signed_by(
        &signing_certificate(),
        PIN,
        format,
        algorithm,
        data,
        operation,
        declared,
    )
}

/// El mismo ciclo, con el certificado que se le diga: el de RSA o el de curva elíptica.
pub(crate) fn a_cycle_signed_by(
    certificate: &TokenCertificate,
    secret: &str,
    format: Format,
    algorithm: SignatureAlgorithm,
    data: &[u8],
    operation: SignatureOperation,
    declared: &[(&str, &str)],
) -> Vec<u8> {
    let bridge = bridge();
    let chain = certificate.chain();
    let reference = certificate.reference().clone();
    let config = SignatureConfig {
        placement: None,
        layer2_text: String::new(),
        rubric_image: None,
        sign_reason: None,
        allow_unregistered_signatures: false,
    };
    let from_the_site: BTreeMap<String, String> = declared
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();

    let cycle = cycle::presign(
        &bridge,
        SigningRequest {
            format,
            algorithm,
            operation,
            document: AdmissibleDocument::check_for(format, data, Waivers::NONE)
                .expect("el puente firma cualquier byte fuera de PAdES"),
            chain: &chain,
            config: &config,
            from_the_site: &from_the_site,
            certificate: &reference,
        },
    )
    .unwrap_or_else(|error| panic!("la prefirma en {format} debería salir: {error}"));

    let signature = cycle
        .sign_on_token(&pkcs11::RealToken, secret)
        .expect("el token debería firmar los atributos");

    cycle
        .postsign(&bridge, signature, &cycle.seal_in_transit())
        .unwrap_or_else(|error| panic!("la postfirma en {format} debería ensamblar: {error}"))
        .into_signed_document()
}

pub(crate) fn write_to_target(name: &str, bytes: &[u8]) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, bytes).expect("deberia poder escribirse el PDF de la prueba");
    path
}

/// El oráculo del original: `rfirma-native-bridge/testbench/validate.sh` (ADR-0014).
pub(crate) fn the_original_validator_accepts(signature: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../rfirma-native-bridge/testbench/validate.sh");
    let output = Command::new(&script)
        .arg(signature)
        .output()
        .unwrap_or_else(|error| panic!("no se ha podido ejecutar {}: {error}", script.display()));
    let verdict = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "el validador del original rechaza {}: {verdict}{}",
        signature.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(verdict.contains("VALID"), "{verdict}");
}

/// Valida un CMS con openssl; `content` solo lo lleva la firma explícita (ADR-0014).
pub(crate) fn openssl_cms_verify(signature: &Path, content: Option<&Path>) -> Vec<u8> {
    let recovered = signature.with_extension("recuperado.bin");
    let output = run_openssl_cms_verify(signature, content, &recovered);
    assert!(
        output.status.success(),
        "openssl no acepta la firma {}: {}",
        signature.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read(&recovered).expect("openssl deja el contenido recuperado")
}

/// Sin `-content` una firma detached no tiene contenido que verificar.
pub(crate) fn openssl_cms_finds_no_content_in(signature: &Path) {
    let discarded = signature.with_extension("descartado.bin");
    let output = run_openssl_cms_verify(signature, None, &discarded);
    assert!(
        !output.status.success(),
        "openssl verifica {} sin -content: la firma lleva el contenido dentro \
         y no es explícita",
        signature.display()
    );
}

fn run_openssl_cms_verify(
    signature: &Path,
    content: Option<&Path>,
    recovered: &Path,
) -> std::process::Output {
    let mut command = Command::new("openssl");
    command
        .arg("cms")
        .arg("-verify")
        .arg("-inform")
        .arg("DER")
        .arg("-noverify")
        // Sin esto openssl canoniza los saltos del contenido y el reto deja
        // de casar con el messageDigest que selló la prefirma.
        .arg("-binary")
        .arg("-in")
        .arg(signature)
        .arg("-out")
        .arg(recovered);
    if let Some(content) = content {
        command.arg("-content").arg(content);
    }
    command.output().unwrap_or_else(|error| {
        panic!(
            "falta openssl: es la puerta de validez de CAdES en la grada C (ADR-0014).\n  \
             sudo apt install -y openssl\n  {error}"
        )
    })
}

/// Los certificados que lleva dentro un CMS, tal y como los enumera openssl (ADR-0014).
pub(crate) fn openssl_prints_the_certificates_of(signature: &Path) -> String {
    let output = Command::new("openssl")
        .arg("pkcs7")
        .arg("-inform")
        .arg("DER")
        .arg("-in")
        .arg(signature)
        .arg("-print_certs")
        .arg("-noout")
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "falta openssl: es la puerta de validez de CAdES en la grada C \
                 (ADR-0014).\n  sudo apt install -y openssl\n  {error}"
            )
        });
    assert!(
        output.status.success(),
        "openssl no ha podido leer los certificados de {}: {}",
        signature.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// El `.p12` del kit, instalado en su propio almacén NSS con la cadena que traía dentro.
pub(crate) fn an_installed_certificate(installed: &Path) -> TokenCertificate {
    let p12 = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/fnmt/active-rsa.p12");
    let bytes = std::fs::read(&p12).expect("el .p12 del kit deberia leerse");
    certificates::install_pkcs12(
        &pkcs11::RealToken,
        &RealInstalledFolder,
        installed,
        &bytes,
        KIT_PASSWORD,
    )
    .expect("el .p12 del kit deberia instalarse");

    let softoken = pkcs11::stores::softoken().expect(
        "falta libsoftokn3.so. El almacen del .p12 lo necesita:\n  \
         sudo apt install -y libnss3",
    );
    certificates::certificates_with_their_chains(
        &pkcs11::RealToken,
        &pkcs11::stores::installed_stores(&softoken, installed),
    )
    .expect("el almacen del .p12 deberia listarse")
    .into_iter()
    .next()
    .expect("el .p12 trae el certificado de persona")
}

/// El CMS que un PDF firmado lleva dentro, tal y como lo vuelca pdfsig (ADR-0014).
pub(crate) fn the_cms_inside(pdf: &Path) -> PathBuf {
    let dumped = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pades-con-cadena.volcado");
    let _ = std::fs::remove_dir_all(&dumped);
    std::fs::create_dir_all(&dumped).expect("deberia poder crearse el directorio del volcado");

    let output = Command::new("pdfsig")
        .arg("-dump")
        .arg(pdf)
        .current_dir(&dumped)
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

    std::fs::read_dir(&dumped)
        .expect("el directorio del volcado deberia leerse")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .next()
        .unwrap_or_else(|| panic!("pdfsig -dump no ha dejado ningun CMS:\n{report}"))
}

/// Genera un PDF sintético de una página.
pub(crate) fn a_one_page_pdf() -> Vec<u8> {
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
pub(crate) fn a_black_rubric() -> String {
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(200, 100, image::Rgb([0, 0, 0])))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("el PNG de prueba deberia codificarse");

    rubric::normalize(&png)
        .expect("una rubrica negra es normalizable")
        .to_base64()
}

/// Configuración de firma para el recuadro de prueba.
pub(crate) fn a_config_of(text: &str, rubric: Option<String>) -> SignatureConfig {
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
