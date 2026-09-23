//! `checkSignatures=true` de `sign(format=PAdES)`: sigue hasta firmar si la firma previa se sostiene, y la rechaza si no.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// Una ronda del banco que firma `pdf` con `format=PAdES` y devuelve el PDF firmado, para medir
/// después sobre él la comprobación de las firmas previas.
async fn the_pdf_signed_by_the_bench(pdf: &Path) -> Vec<u8> {
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script_over_the_pdf(
        &material,
        BenchMode::Fourth,
        THE_SIGN_PADES,
        pdf,
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_sign_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "la primera firma PAdES tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    let signed = STANDARD
        .decode(verdict.field("result"))
        .expect("el PDF firmado llega en base64");

    channel.close();
    signed
}

/// Lo que el cliente publicado recibe al firmar `pdf` con `checkSignatures=true` en las
/// `properties`.
async fn checking_the_signatures_of(pdf: &Path) -> Event {
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script_over_the_pdf(
        &material,
        BenchMode::Fourth,
        THE_SIGN_PADES_CHECKING_SIGNATURES,
        pdf,
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_sign_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    channel.close();
    verdict
}

/// La version del encabezado entra en el `/ByteRange`: el resumen de la firma deja de cuadrar.
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

/// `checkSignatures=true` medido por el cable entero contra la libreria nativa: sobre el PDF
/// firmado sin tocar el tramite sigue hasta la firma, y sobre el mismo PDF alterado el
/// `errorCallback` del cliente publicado recibe `SAF_39` (`ERROR_INVALID_SIGNATURE`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_is_refused_a_document_whose_previous_signature_does_not_hold() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let pdf = a_temp_file(".pdf", &a_one_page_pdf());
    let signed = the_pdf_signed_by_the_bench(pdf.path()).await;

    let signed_file = a_temp_file(".pdf", &signed);
    let held = checking_the_signatures_of(signed_file.path()).await;
    assert_eq!(
        held.name(),
        "success",
        "la firma previa se sostiene, asi que el tramite tenia que seguir hasta firmar, y acabo \
         en {}: {}",
        held.name(),
        held.field("message")
    );

    let altered_file = a_temp_file(".pdf", &with_the_signed_bytes_altered(&signed));
    let broken = checking_the_signatures_of(altered_file.path()).await;
    assert_eq!(
        broken.name(),
        "error",
        "el PDF alterado tenia que acabar en el errorCallback, y acabo en {}",
        broken.name()
    );
    assert_eq!(
        broken.field("message"),
        WireAnswer::refused(SafCode::InvalidSignature).on_the_wire(),
        "una firma previa que ya no cuadra tenia que contestar ERROR_INVALID_SIGNATURE"
    );
}
