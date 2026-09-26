//! Prueba de grada C de lo que cuesta en residente la primera firma XAdES, sola en su binario porque mide todo el proceso.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::signing::adapters::ffi::{locate, NativeBridge};
use rfirma_lib::signing::application::cycle::{self, SigningRequest};
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation, XadesVariant};
use rfirma_lib::signing::domain::{AdmissibleDocument, SignatureConfig, Waivers};

const PIN: &str = "1234";
/// Certificado activo del kit de pruebas.
const ACTIVE: &str = "FNMT-ACTIVO-99999999R";

const A_REFERENCE_XML: &[u8] = include_bytes!("../../../testdata/reference/document.xml");

fn bridge() -> NativeBridge {
    let executable = std::env::current_exe().expect("debería haber ejecutable");
    let directory = executable.parent().unwrap_or(Path::new(".")).to_path_buf();
    let library: PathBuf =
        locate(&|name| std::env::var_os(name), &directory).unwrap_or_else(|error| {
            panic!("{error}\n\nejecuta 'just test-native', que exporta RFIRMA_LIB_DIR")
        });
    NativeBridge::open_at(&library).expect("la librería debería cargarse")
}

fn signing_certificate() -> TokenCertificate {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    pkcs11::list_certificates(module)
        .expect("no se ha podido listar el token")
        .into_iter()
        .find(|certificate| certificate.reference().label() == ACTIVE)
        .unwrap_or_else(|| panic!("el token no tiene {ACTIVE}. Montalo con: just certs install"))
}

fn sign_xades(bridge: &NativeBridge, certificate: &TokenCertificate) {
    let format = Format::Xades(XadesVariant::Enveloping);
    let chain = certificate.chain();
    let config = SignatureConfig {
        placement: None,
        layer2_text: String::new(),
        rubric_image: None,
        allow_unregistered_signatures: false,
    };
    let cycle = cycle::presign(
        bridge,
        SigningRequest {
            format,
            algorithm: cycle::ALGORITHM,
            operation: SignatureOperation::Sign,
            document: AdmissibleDocument::check_for(format, A_REFERENCE_XML, Waivers::NONE)
                .expect("un XML es firmable en XAdES"),
            chain: &chain,
            config: &config,
            from_the_site: &BTreeMap::new(),
            certificate: certificate.reference(),
        },
    )
    .unwrap_or_else(|error| panic!("la prefirma XAdES debería salir: {error}"));
    let signature = cycle
        .sign_on_token(&pkcs11::RealToken, &ProtectedSecret::from_str(PIN))
        .expect("el token debería firmar los atributos");
    cycle
        .postsign(bridge, signature, &cycle.seal_in_transit())
        .unwrap_or_else(|error| panic!("la postfirma XAdES debería ensamblar: {error}"));
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

/// JAXP y xmlsec arrancan perezosos dentro de la imagen: la primera firma
/// XAdES es la que los levanta, y aquí se mide cuánto cuesta.
#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn the_first_xades_signature_does_not_blow_the_resident_memory_up() {
    const CEILING: u64 = 128 * 1024 * 1024;
    let bridge = bridge();
    let certificate = signing_certificate();

    let before = resident_bytes();
    sign_xades(&bridge, &certificate);
    let growth = resident_bytes().saturating_sub(before);

    assert!(
        growth < CEILING,
        "la primera firma XAdES ha añadido {growth} bytes de residente"
    );
}
