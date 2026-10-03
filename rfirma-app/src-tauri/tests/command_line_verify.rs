//! Prueba de grada C de la orden `verify` contra el puente real y las muestras de `testdata/` (ADR-0014).

#[path = "native_cycle/support.rs"]
mod support;

use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, Utc};
use rfirma_lib::desktop::adapters::command_line_ports::{
    DiskFiles, NativeFilter, NativeReader, NativeVerifier,
};
use rfirma_lib::desktop::adapters::handover::SpawnedDesktop;
use rfirma_lib::desktop::adapters::terminal::{ProcessDescriptors, ProcessTerminal, SeenStores};
use rfirma_lib::desktop::application::command_line::{
    attend, CommandLinePorts, Outcome, SUCCEEDED, UNKNOWN_FORMAT,
};
use rfirma_lib::desktop::ports::{
    CommandLineSigning, DocumentSigner, GraphicalPicker, LocalTimeZone, WindowChoice, WindowOffer,
};
use rfirma_lib::identity::domain::certificate::{CertificateRef, TokenCertificate};
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};

use support::{a_cycle_of, a_one_page_pdf};

fn sample(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata")
        .join(relative)
}

struct NeverSigns;

impl GraphicalPicker for NeverSigns {
    fn has_a_display(&self) -> bool {
        panic!("verify no abre la ventana de sede")
    }

    fn chosen(&self, document: &Path, _offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        panic!("verify no elige certificado para {}", document.display())
    }
}

impl DocumentSigner for NeverSigns {
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        panic!("verify no firma {}", request.input.display())
    }

    fn remember(&self, _certificate: &TokenCertificate) {
        panic!("verify no recuerda certificados")
    }

    fn remembered(&self) -> Option<CertificateRef> {
        None
    }
}

/// La hora de Madrid en verano, para que la fecha declarada no dependa de la máquina.
struct SummerInMadrid;

impl LocalTimeZone for SummerInMadrid {
    fn offset_at(&self, _instant: DateTime<Utc>) -> FixedOffset {
        FixedOffset::east_opt(2 * 3600).expect("+02:00 es un desplazamiento")
    }
}

fn verified(path: &Path) -> Outcome {
    attended(&["verify", "-i", &path.display().to_string()])
}

fn attended(words: &[&str]) -> Outcome {
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    attend(
        &arguments,
        &CommandLinePorts {
            stores: &SeenStores::over(Vec::new()),
            terminal: &ProcessTerminal,
            descriptor: &ProcessDescriptors,
            desktop: &SpawnedDesktop,
            filter: &NativeFilter,
            files: &DiskFiles,
            verifier: &NativeVerifier,
            reader: &NativeReader,
            time_zone: &SummerInMadrid,
            signer: &NeverSigns,
            window: &NeverSigns,
        },
    )
}

fn printed_lines(outcome: &Outcome) -> Vec<String> {
    assert_eq!(
        outcome.exit_code, SUCCEEDED,
        "verify sale con 0 sea cual sea la firma: {:?}",
        outcome.stderr
    );
    String::from_utf8(outcome.stdout.clone())
        .expect("la salida es UTF-8")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// La firma CAdES implícita de referencia con un byte de su contenido cambiado.
fn a_tampered_cades() -> PathBuf {
    let challenge = std::fs::read(sample("reference/challenge.bin")).expect("el reto se lee");
    let mut signature =
        std::fs::read(sample("reference/cades-implicit.p7s")).expect("la firma se lee");
    let at = signature
        .windows(challenge.len())
        .position(|window| window == challenge)
        .expect("la firma implícita lleva el reto dentro");
    signature[at] ^= 0xff;
    let tampered = Path::new(env!("CARGO_TARGET_TMPDIR")).join("verify-tampered.p7s");
    std::fs::write(&tampered, signature).expect("la firma manipulada se escribe");
    tampered
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_valid_signature_prints_firma_valida() {
    for valid in [
        "reference/cades-implicit.p7s",
        "reference/xades-enveloping.xml",
        "reference/facturae.xsig",
    ] {
        let lines = printed_lines(&verified(&sample(valid)));

        assert_eq!(lines, ["Firma valida"], "{valid}");
    }
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_pades_signed_with_the_token_prints_firma_valida() {
    let signed = a_cycle_of(
        Format::Pades,
        ALGORITHM,
        &a_one_page_pdf(),
        SignatureOperation::Sign,
        &[],
    );
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("verify-signed.pdf");
    std::fs::write(&path, signed).expect("el PDF firmado se escribe");

    assert_eq!(printed_lines(&verified(&path)), ["Firma valida"]);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_tampered_signature_prints_that_it_does_not_hold_and_ends_with_zero() {
    let lines = printed_lines(&verified(&a_tampered_cades()));

    assert!(
        lines.iter().all(|line| line.starts_with("Firma no valida")),
        "{lines:?}"
    );
    assert!(!lines.is_empty());
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_signature_with_an_expired_certificate_is_not_valid() {
    let lines = printed_lines(&verified(&sample(
        "previous-signatures/pades-long-term-expired.pdf",
    )));

    assert!(
        lines.contains(&"Firma no valida: existe un certificado de firma caducado".to_owned()),
        "{lines:?}"
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_file_without_signatures_prints_that_it_has_none() {
    let lines = printed_lines(&verified(&sample("reference/document.xml")));

    assert_eq!(
        lines,
        ["Firma no valida: no se encuentra la firma dentro del documento"]
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_file_of_no_signature_format_prints_its_result() {
    let lines = printed_lines(&verified(&sample("reference/challenge.bin")));

    assert_eq!(lines, [UNKNOWN_FORMAT]);
}

fn a_pdf_signed_twice_with_the_token() -> PathBuf {
    let signed = a_cycle_of(
        Format::Pades,
        ALGORITHM,
        &a_one_page_pdf(),
        SignatureOperation::Sign,
        &[],
    );
    let twice = a_cycle_of(
        Format::Pades,
        ALGORITHM,
        &signed,
        SignatureOperation::Sign,
        &[],
    );
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("verify-signed-twice.pdf");
    std::fs::write(&path, twice).expect("el PDF firmado dos veces se escribe");
    path
}

/// Las líneas con la fecha declarada, que es la de una firma recién hecha, comprobada y con un marcador.
fn with_the_declared_time_checked(lines: Vec<String>, since: DateTime<Utc>) -> Vec<String> {
    const DECLARED: &str = "  Fecha declarada:   ";
    lines
        .into_iter()
        .map(|line| match line.strip_prefix(DECLARED) {
            None => line,
            Some(declared) => {
                let instant = DateTime::parse_from_str(declared, "%Y-%m-%d %H:%M:%S %:z")
                    .unwrap_or_else(|error| panic!("«{declared}» no es una fecha local: {error}"));
                assert_eq!(instant.offset().local_minus_utc(), 2 * 3600, "{declared}");
                assert!(
                    instant >= since - chrono::Duration::seconds(1) && instant <= Utc::now(),
                    "{declared} no es el instante de la firma"
                );
                format!("{DECLARED}<instante de la firma>")
            }
        })
        .collect()
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn verbose_prints_the_validity_the_format_and_a_sheet_per_signature_of_a_pdf() {
    let since = Utc::now();
    let path = a_pdf_signed_twice_with_the_token();

    let outcome = attended(&["verify", "-i", &path.display().to_string(), "--verbose"]);

    assert!(outcome.stderr.is_empty(), "{:?}", outcome.stderr);
    assert_eq!(
        with_the_declared_time_checked(printed_lines(&outcome), since),
        [
            "Firma valida",
            "",
            "Formato: PAdES",
            "",
            "Firma 1",
            "  Firmante:          EIDAS CERTIFICADO PRUEBAS (99999999R)",
            "  Emisor:            AC FNMT Usuarios",
            "  Fecha declarada:   <instante de la firma>",
            "",
            "Firma 2",
            "  Firmante:          EIDAS CERTIFICADO PRUEBAS (99999999R)",
            "  Emisor:            AC FNMT Usuarios",
            "  Fecha declarada:   <instante de la firma>",
        ]
    );
}

fn a_cades_cosigned_with_the_token() -> PathBuf {
    let challenge = std::fs::read(sample("reference/challenge.bin")).expect("el reto se lee");
    let signed = a_cycle_of(
        Format::Cades,
        ALGORITHM,
        &challenge,
        SignatureOperation::Sign,
        &[("mode", "implicit")],
    );
    let cosigned = a_cycle_of(
        Format::Cades,
        ALGORITHM,
        &signed,
        SignatureOperation::Cosign,
        &[("mode", "implicit")],
    );
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("verify-cosigned.csig");
    std::fs::write(&path, cosigned).expect("el CAdES cofirmado se escribe");
    path
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn verbose_prints_the_format_and_a_sheet_per_cosignature_of_a_cades() {
    let since = Utc::now();
    let path = a_cades_cosigned_with_the_token();

    let outcome = attended(&["verify", "-i", &path.display().to_string(), "-v"]);

    assert!(outcome.stderr.is_empty(), "{:?}", outcome.stderr);
    assert_eq!(
        with_the_declared_time_checked(printed_lines(&outcome), since),
        [
            "Firma valida",
            "Firma valida",
            "",
            "Formato: CAdES",
            "",
            "Firma 1",
            "  Firmante:          EIDAS CERTIFICADO PRUEBAS (99999999R)",
            "  Emisor:            AC FNMT Usuarios",
            "  Fecha declarada:   <instante de la firma>",
            "",
            "Firma 2",
            "  Firmante:          EIDAS CERTIFICADO PRUEBAS (99999999R)",
            "  Emisor:            AC FNMT Usuarios",
            "  Fecha declarada:   <instante de la firma>",
        ]
    );
}
