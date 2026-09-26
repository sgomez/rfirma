//! Lote remoto (`signBatchJSON` y `signBatch` en XML) con presigner y postsigner, y su rechazo cuando el presigner está caído.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// El fichero congelado con el que contesta uno de los servlets del lote, sin espacios: la sangría
/// la pone el formateador del repositorio y el cliente publicado lo reserializa compacto.
fn the_frozen(fixture: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/site-driver")
        .join(fixture);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("falta la fixture {}: {error}", path.display()));
    without_spaces(&text)
}

fn without_spaces(text: &str) -> String {
    text.split_whitespace().collect()
}

/// La CA local, en el fichero que OpenSSL lee como almacén de confianza: el cliente de los dos
/// servlets es el de producción y valida TLS con el almacén del sistema, así que sin esto un
/// servlet del banco seria un «servlet inalcanzable».
fn the_local_ca_trusted_by_the_batch_client(ca_pem_path: &std::path::Path) {
    std::env::set_var("SSL_CERT_FILE", ca_pem_path);
}

/// El trámite atendiendo el lote remoto: consiente con el certificado de pruebas, lo cierra con el
/// secreto del token y apunta el DER del firmante para contrastarlo con el que recibe la sede.
fn the_batch_errand_of(roots: &Arc<Roots>, signer: &Arc<Mutex<Option<Vec<u8>>>>) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSignTheBatch(consent)) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };
        assert_eq!(consent.signs, 2, "el lote del guion lleva dos documentos");

        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just certs install`")
            });
        let signing_certificate = roots
            .identity
            .chosen(&chosen.id)
            .expect("el certificado consentido deberia seguir en el token");
        *signer
            .lock()
            .expect("nadie envenena el apunte del firmante") =
            Some(signing_certificate.der().to_vec());

        errand::consent(&desk, &chosen.id, live).expect("el lote deberia quedar consentido");
        tokio::task::block_in_place(|| {
            signed_with_the_secret(&desk, live, "")
                .expect("el lote deberia cerrarse con el secreto del token")
        });
    })
}

/// El lote remoto de dos documentos, del `signBatchJSON` del cliente publicado al resultado
/// congelado del postsigner, pasando por los dos servlets que levanta el conductor.
#[expect(clippy::too_many_lines)]
async fn the_remote_batch_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    the_local_ca_trusted_by_the_batch_client(material.ca_pem_file.path());

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, THE_REMOTE_BATCH);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_batch_errand_of(&roots, &signer),
    )
    .await;

    let presign = client.next_event();
    assert_eq!(
        presign.name(),
        "presign",
        "el presigner tenia que recibir el lote antes que nada, y llego {}",
        presign.0
    );
    assert_eq!(presign.field("signs"), "2", "el lote lleva dos documentos");
    assert_eq!(
        presign.field("certs"),
        "1",
        "el lote viaja con la cadena del unico firmante"
    );
    assert_eq!(
        presign.field("algorithm"),
        "SHA256",
        "el algoritmo que declara el lote es el que llega al servlet"
    );

    let postsign = client.next_event();
    assert_eq!(
        postsign.name(),
        "postsign",
        "el postsigner tenia que recibir el tridata firmado, y llego {}",
        postsign.0
    );
    assert_eq!(
        postsign.field("signs"),
        "2",
        "las dos firmas del lote llegan con su PK1"
    );
    assert_eq!(
        postsign.field("pre"),
        "1",
        "solo la firma con NEED_PRE=true conserva su PRE"
    );

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "el lote tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    let result = STANDARD
        .decode(verdict.field("result"))
        .expect("el resultado del lote llega en base64");
    assert_eq!(
        without_spaces(&String::from_utf8(result).expect("el resultado del lote es texto")),
        the_frozen("batch-postsign-result.json"),
        "el cliente publicado recibe el resultado del postsigner tal cual"
    );
    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "con needcert el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_in_json() {
    the_remote_batch_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_also_over_the_third_protocol() {
    the_remote_batch_of(BenchMode::Third).await;
}

/// El lote remoto heredado en XML, del `signBatch` del cliente publicado al resultado congelado
/// del postsigner, pasando por los dos servlets que levanta el conductor.
#[expect(clippy::too_many_lines)]
async fn the_remote_xml_batch_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    the_local_ca_trusted_by_the_batch_client(material.ca_pem_file.path());

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, THE_LEGACY_XML_BATCH);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_batch_errand_of(&roots, &signer),
    )
    .await;

    let presign = client.next_event();
    assert_eq!(
        presign.name(),
        "presign",
        "el presigner tenia que recibir el lote antes que nada, y llego {}",
        presign.0
    );
    assert_eq!(presign.field("signs"), "2", "el lote lleva dos documentos");
    assert_eq!(
        presign.field("certs"),
        "1",
        "el lote viaja con la cadena del unico firmante"
    );
    assert_eq!(
        presign.field("algorithm"),
        "SHA256",
        "el algoritmo que declara el lote es el que llega al servlet"
    );

    let postsign = client.next_event();
    assert_eq!(
        postsign.name(),
        "postsign",
        "el postsigner tenia que recibir el tridata firmado, y llego {}",
        postsign.0
    );
    assert_eq!(
        postsign.field("signs"),
        "2",
        "las dos firmas del lote llegan con su PK1"
    );
    assert_eq!(
        postsign.field("pre"),
        "1",
        "solo la firma con NEED_PRE=true conserva su PRE"
    );

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "el lote tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    let result = STANDARD
        .decode(verdict.field("result"))
        .expect("el resultado del lote llega en base64");
    assert_eq!(
        without_spaces(&String::from_utf8(result).expect("el resultado del lote es texto")),
        the_frozen("batch-xml-postsign-result.xml"),
        "el cliente publicado recibe el resultado del postsigner tal cual"
    );
    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "con needcert el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_in_legacy_xml() {
    the_remote_xml_batch_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_in_legacy_xml_also_over_the_third_protocol() {
    the_remote_xml_batch_of(BenchMode::Third).await;
}

/// El trámite atendiendo el lote remoto cuyo presigner esta caido: consiente y cierra con el
/// secreto del token, y comprueba que el lote se rechaza antes de llegar al postsigner.
fn the_down_presigner_batch_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSignTheBatch(consent)) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };

        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just certs install`")
            });

        errand::consent(&desk, &chosen.id, live).expect("el lote deberia quedar consentido");
        tokio::task::block_in_place(|| {
            let outcome = errand::finish_the_batch(
                &desk,
                &rfirma_lib::identity::domain::protected_secret::ProtectedSecret::from_str(
                    THE_TOKEN_SECRET,
                ),
                live,
            );
            assert!(
                outcome.is_err(),
                "el lote deberia rechazarse con el presigner caido"
            );
        });
    })
}

/// Sin presigner escuchando, el `errorCallback` del cliente publicado tiene que recibir `SAF_26`
/// (`ERROR_CONTACT_BATCH_SERVICE`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_remote_batch_fails_when_the_presigner_is_down() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    the_local_ca_trusted_by_the_batch_client(material.ca_pem_file.path());

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let client = PublishedClient::running_the_script(
        &material,
        BenchMode::Fourth,
        THE_REMOTE_BATCH_WITH_THE_DOWN_PRESIGNER,
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_down_presigner_batch_errand_of(&roots),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "el lote tenia que acabar en el errorCallback, y acabo en {}",
        verdict.name()
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(SafCode::ContactBatchService).on_the_wire(),
        "el presigner caido tenia que contestar ERROR_CONTACT_BATCH_SERVICE"
    );

    channel.close();
}
