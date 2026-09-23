//! `sign()` del cliente publicado sobre cada formato (CAdES, ASiC-S, XAdES, FacturaE) y sus variantes de invocación.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// Un `sign()` del cliente publicado del reto binario del banco de referencia, verificado con
/// `openssl cms -verify` y con el oráculo de la grada C. El puente entrega el CAdES detached en
/// ambos guiones, con y sin `mode=explicit`, así que el reto original hace falta en los dos.
async fn the_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, script);

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
        "'{script}' tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let cms = STANDARD
        .decode(verdict.field("result"))
        .expect("el CMS de sign llega en base64");
    verified_by_openssl(&cms, &the_challenge_path());
    validated_by_the_reference_tool(&cms);

    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_cades_explicit() {
    the_sign_of(BenchMode::Fourth, THE_SIGN_CADES_EXPLICIT).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_gzipped_binary_challenge() {
    the_sign_of(BenchMode::Fourth, THE_SIGN_GZIP).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_cades_explicit_also_over_the_third_protocol(
) {
    the_sign_of(BenchMode::Third, THE_SIGN_CADES_EXPLICIT).await;
}

/// Dos trámites de sede a la vez en el mismo proceso, cada uno con su propia terna de `ports=`
/// (ID-06): no hay techo de trámites simultáneos ni estado global que los estorbe. No toma
/// `ONE_AT_A_TIME`, porque eso solo lo necesitan los casos de lote con servlets.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn two_published_clients_sign_at_once_in_the_same_process() {
    if !the_bench_can_be_mounted() {
        return;
    }

    tokio::join!(
        the_sign_of(BenchMode::Fourth, THE_SIGN_CADES_EXPLICIT),
        the_sign_of(BenchMode::Fourth, THE_SIGN_CADES_EXPLICIT),
    );
}

/// Un `sign()` del cliente publicado con `format=CAdES-ASiC-S`: lo que vuelve no es un CMS sino
/// el contenedor ZIP, con la firma CAdES dentro, y el oráculo de la grada C lo valida.
async fn the_asic_s_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, script);

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
        "'{script}' tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let container = STANDARD
        .decode(verdict.field("result"))
        .expect("el contenedor de sign llega en base64");
    assert_eq!(
        &container[..4],
        b"PK\x03\x04",
        "un ASiC-S es un ZIP y empieza por su firma de fichero local"
    );
    carries_the_asic_s_binary_signature(&container);
    validated_by_the_reference_tool_at(an_asic_s_file(&container).path());

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_into_an_asic_s_container() {
    the_asic_s_sign_of(BenchMode::Fourth, THE_SIGN_CADES_ASIC_S).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_format_auto() {
    the_sign_of(BenchMode::Fourth, THE_SIGN_AUTO).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_format_auto_also_over_the_third_protocol(
) {
    the_sign_of(BenchMode::Third, THE_SIGN_AUTO).await;
}

/// Un `sign()` del cliente publicado del XML de referencia, con `ds:Signature` bien formado
/// (`xmllint`) y aceptado por el oráculo de la grada C. `format=XAdES` y `format=auto` sobre XML
/// resuelven la misma variante Enveloping, así que comparten guion de verificación.
async fn the_xades_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, script);

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
        "'{script}' tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let xml = STANDARD
        .decode(verdict.field("result"))
        .expect("el XML de sign llega en base64");
    carries_a_xmldsig_signature(&xml);
    let xml_file = an_xml_file(&xml);
    well_formed_according_to_xmllint(xml_file.path());
    validated_by_the_reference_tool_at(xml_file.path());

    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_xades() {
    the_xades_sign_of(BenchMode::Fourth, THE_SIGN_XADES).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_xades_also_over_the_third_protocol() {
    the_xades_sign_of(BenchMode::Third, THE_SIGN_XADES).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_format_auto() {
    the_xades_sign_of(BenchMode::Fourth, THE_SIGN_XADES_AUTO).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_format_auto_also_over_the_third_protocol()
{
    the_xades_sign_of(BenchMode::Third, THE_SIGN_XADES_AUTO).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_invoice_with_facturae() {
    the_xades_sign_of(BenchMode::Fourth, THE_SIGN_FACTURAE).await;
}

/// Cofirmar una factura no se admite: el `errorCallback` del cliente publicado tiene que
/// recibir `SAF_04` (`ERROR_UNSUPPORTED_OPERATION`), sin que el trámite llegue a pedir
/// consentimiento.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn cosigning_an_invoice_with_facturae_is_refused() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let client =
        PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_COSIGN_FACTURAE);

    let channel =
        the_errand_channel(&client, &material, &roots, the_refusing_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "la cofirma de una factura tenia que acabar en el errorCallback, y acabo en {}",
        verdict.name()
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire(),
        "la cofirma de una factura tenia que contestar ERROR_UNSUPPORTED_OPERATION"
    );

    channel.close();
}

/// Un `sign()` con `format=XAdES` cuyas condiciones, medidas por la sede, salen todas conformes.
async fn the_envelope_of(script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script(script).await;

    let verdict = events.last().expect("hay desenlace");
    assert_eq!(verdict.name(), "success", "{}", verdict.field("message"));
    let conditions: Vec<(&str, &str)> = events
        .iter()
        .filter(|event| event.name() == "condition")
        .map(|event| (event.field("verdict"), event.field("observation")))
        .collect();
    assert!(!conditions.is_empty(), "la sede mide la envoltura");
    assert!(
        conditions
            .iter()
            .all(|(verdict, _)| *verdict == "compliant"),
        "la sede midio: {conditions:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_generic_xades_signs_externally_detached_when_its_params_ask_for_it() {
    the_envelope_of("signxadesexternallydetached").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_age_policy_turns_a_xades_enveloping_into_detached() {
    the_envelope_of("signxadesagepolicy").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_xades_enveloped_over_data_that_is_no_xml_is_rejected_with_saf_29() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script("signxadesenvelopedoveranonxml").await;

    let verdict = events.last().expect("hay desenlace");
    assert_eq!(
        verdict.name(),
        "error",
        "tenia que acabar en el errorCallback"
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(SafCode::InvalidXml).on_the_wire()
    );
}
