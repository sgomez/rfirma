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
