//! Prueba de grada C de que el puente no pierde el JSON que devuelve, sola en su binario porque mide la residente de todo el proceso.

use std::path::{Path, PathBuf};

use rfirma_lib::signing::adapters::ffi::{locate, NativeBridge};
use rfirma_lib::signing::domain::bridge::{
    Format, PreSignRequest, SignatureOperation, XadesVariant,
};

/// Un PDF mínimo en Base64 no válido para firmar.
const NOT_A_PDF_B64: &str = "bm8gc295IHVuIFBERg==";

/// Un certificado sintético no válido.
const NOT_A_CERTIFICATE_B64: &str = "bm8gc295IHVuIGNlcnRpZmljYWRv";

fn bridge() -> NativeBridge {
    let executable = std::env::current_exe().expect("debería haber ejecutable");
    let directory = executable.parent().unwrap_or(Path::new(".")).to_path_buf();
    let library: PathBuf =
        locate(&|name| std::env::var_os(name), &directory).unwrap_or_else(|error| {
            panic!("{error}\n\nejecuta 'just test-native', que exporta RFIRMA_LIB_DIR")
        });
    NativeBridge::open_at(&library).expect("la librería debería cargarse")
}

fn presign_of_something_invalid_in(bridge: &NativeBridge, format: Format) {
    let _ = bridge.presign(PreSignRequest {
        format,
        operation: SignatureOperation::Sign,
        document_b64: NOT_A_PDF_B64,
        algorithm: "SHA256withRSA",
        certificate_chain_b64: NOT_A_CERTIFICATE_B64,
        extra_params: "signaturePage=1\n",
    });
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

fn a_hundred_thousand_round_trips_do_not_leak(bridge: &NativeBridge, format: Format) {
    const BATCH: usize = 100_000;
    const TOLERANCE: u64 = 1024 * 1024;

    for _ in 0..BATCH {
        presign_of_something_invalid_in(bridge, format);
    }
    let after_first_batch = resident_bytes();
    for _ in 0..BATCH {
        presign_of_something_invalid_in(bridge, format);
    }
    let after_second_batch = resident_bytes();

    let growth = after_second_batch.saturating_sub(after_first_batch);
    assert!(
        growth < TOLERANCE,
        "la segunda tanda de {BATCH} vueltas en {format} ha crecido {growth} bytes: \
         alguien ha dejado de llamar a autofirma_free_string"
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_hundred_thousand_round_trips_per_format_do_not_leak_the_json_of_the_bridge() {
    let bridge = bridge();

    for format in [
        Format::Pades,
        Format::Cades,
        Format::Xades(XadesVariant::Enveloping),
    ] {
        a_hundred_thousand_round_trips_do_not_leak(&bridge, format);
    }
}
