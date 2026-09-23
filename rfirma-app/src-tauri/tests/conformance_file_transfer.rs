//! Transferencia de ficheros con el portal: `saveDataToFile`, `getFileNameContentBase64`, `getMultiFileNameContentBase64` y `signAndSaveToFile`.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// Guarda datos en disco con `saveDataToFile` y verifica la respuesta `SAVE_OK` y el fichero escrito.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_saves_data_to_file() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let target_dir = tempfile::tempdir().expect("directorio de guardado");
    let save_path = target_dir.path().join("saved_challenge.bin");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::saving_to(&save_path));
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_SAVE);
    let channel = the_errand_channel(&client, &material, &roots, the_save_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "saveDataToFile tenía que acabar en el successCallback, y acabó en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    assert_eq!(
        verdict.field("data"),
        "SAVE_OK",
        "el successCallback recibe SAVE_OK"
    );
    assert_eq!(
        std::fs::read(&save_path).expect("el fichero guardado debe existir"),
        std::fs::read(the_challenge_path()).expect("el reto debe leerse"),
        "el fichero guardado en disco coincide con los datos enviados"
    );
    channel.close();
}

/// Cancela el diálogo de guardado en `saveDataToFile` y verifica la excepción de cancelación.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_cancels_saving_data_to_file() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::cancelling_save());
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_SAVE);
    let channel = the_errand_channel(&client, &material, &roots, the_save_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "la cancelación de saveDataToFile tenía que acabar en el errorCallback"
    );
    assert_eq!(
        verdict.field("type"),
        "es.gob.afirma.core.AOCancelledOperationException",
        "el errorCallback recibe la excepción de cancelación"
    );
    channel.close();
}

/// Carga un único fichero con `getFileNameContentBase64` y verifica nombre y contenido en base64.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_loads_single_file() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let load_dir = tempfile::tempdir().expect("directorio de carga");
    let file_path = load_dir.path().join("documento.bin");
    let content = b"contenido de prueba para carga simple";
    std::fs::write(&file_path, content).expect("debe escribirse el fichero");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::picking_file(&file_path));
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_LOAD);
    let channel = the_errand_channel(&client, &material, &roots, the_load_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "getFileNameContentBase64 tenía que acabar en el successCallback, y acabó en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    assert_eq!(verdict.field("filename"), "documento.bin");
    assert_eq!(verdict.field("data"), STANDARD.encode(content));
    channel.close();
}

/// Cancela el diálogo de carga en `getFileNameContentBase64` y verifica la excepción de cancelación.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_cancels_loading_file() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::cancelling_pick());
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_LOAD);
    let channel = the_errand_channel(&client, &material, &roots, the_load_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "la cancelación de getFileNameContentBase64 tenía que acabar en el errorCallback"
    );
    assert_eq!(
        verdict.field("type"),
        "es.gob.afirma.core.AOCancelledOperationException"
    );
    channel.close();
}

/// Carga múltiples ficheros con `getMultiFileNameContentBase64` y verifica nombres y contenidos.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_loads_multiple_files() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let load_dir = tempfile::tempdir().expect("directorio de carga");
    let file1 = load_dir.path().join("doc1.bin");
    let file2 = load_dir.path().join("doc2.bin");
    let content1 = b"primer fichero";
    let content2 = b"segundo fichero";
    std::fs::write(&file1, content1).expect("debe escribirse doc1");
    std::fs::write(&file2, content2).expect("debe escribirse doc2");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::picking_files([&file1, &file2]));
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_MULTI_LOAD);
    let channel = the_errand_channel(&client, &material, &roots, the_load_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "getMultiFileNameContentBase64 tenía que acabar en el successCallback, y acabó en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    assert_eq!(verdict.field("filenames"), "doc1.bin|doc2.bin");
    assert_eq!(
        verdict.field("data"),
        format!(
            "{}|{}",
            STANDARD.encode(content1),
            STANDARD.encode(content2)
        )
    );
    channel.close();
}

/// Cancela el diálogo de carga múltiple en `getMultiFileNameContentBase64` y verifica la excepción de cancelación.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_cancels_loading_multiple_files() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::cancelling_pick());
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_MULTI_LOAD);
    let channel = the_errand_channel(&client, &material, &roots, the_load_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "la cancelación de getMultiFileNameContentBase64 tenía que acabar en el errorCallback"
    );
    assert_eq!(
        verdict.field("type"),
        "es.gob.afirma.core.AOCancelledOperationException"
    );
    channel.close();
}

/// Firma y guarda en disco con `signAndSaveToFile` verificando la firma guardada y los datos devueltos.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_and_saves_to_file() {
    if !the_bench_can_be_mounted() {
        return;
    }
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let target_dir = tempfile::tempdir().expect("directorio de guardado");
    let save_path = target_dir.path().join("challenge-signed.csig");

    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    let portal = Arc::new(TestPortalDialogs::saving_to(&save_path));
    roots.documents.portal = portal.clone();
    roots.site.portal = portal;
    let roots = Arc::new(roots);

    let signer = Arc::new(Mutex::new(None));
    let client =
        PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_SIGN_AND_SAVE);
    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_sign_and_save_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "signAndSaveToFile tenía que acabar en el successCallback, y acabó en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let cms = STANDARD
        .decode(verdict.field("result"))
        .expect("el CMS de signandsave llega en base64");
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

    let saved_bytes =
        std::fs::read(&save_path).expect("el fichero firmado debe haberse guardado en disco");
    assert_eq!(
        saved_bytes, cms,
        "el contenido guardado en disco coincide con la firma devuelta"
    );

    channel.close();
}
