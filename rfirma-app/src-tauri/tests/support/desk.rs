//! Trámite de sede montado sobre un `rFirma` de pruebas: consiente con el token PKCS#11 y verifica lo firmado contra los oráculos de la grada C.

use super::*;

/// El módulo PKCS#11 del token de pruebas.
pub fn the_test_module() -> PathBuf {
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

/// Las cinco raíces de un rFirma en marcha que solo ve el token de pruebas y recuerda bajo esa
/// carpeta, para que el caso arranque siempre sin certificado recordado.
pub fn a_running_rfirma(home: &std::path::Path) -> Roots {
    let mut roots = rfirma_lib::roots(Paths::under(home));
    roots.identity.stores = vec![Store::module(the_test_module())];
    roots.signing.prompter = Arc::new(MistypesTheTokenSecretOnce);
    roots
}

/// Doble de pruebas para PortalDialogs configurable programáticamente.
#[derive(Clone, Default)]
pub struct TestPortalDialogs {
    single_file: Arc<Mutex<Option<PathBuf>>>,
    multi_files: Arc<Mutex<Vec<PathBuf>>>,
    save_destination: Arc<Mutex<Option<PathBuf>>>,
}

impl TestPortalDialogs {
    pub fn picking_file(path: impl Into<PathBuf>) -> Self {
        Self {
            single_file: Arc::new(Mutex::new(Some(path.into()))),
            ..Default::default()
        }
    }

    pub fn cancelling_pick() -> Self {
        Self {
            single_file: Arc::new(Mutex::new(None)),
            multi_files: Arc::new(Mutex::new(Vec::new())),
            ..Default::default()
        }
    }

    pub fn picking_files(paths: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            multi_files: Arc::new(Mutex::new(paths.into_iter().map(Into::into).collect())),
            ..Default::default()
        }
    }

    pub fn saving_to(path: impl Into<PathBuf>) -> Self {
        Self {
            save_destination: Arc::new(Mutex::new(Some(path.into()))),
            ..Default::default()
        }
    }

    pub fn cancelling_save() -> Self {
        Self {
            save_destination: Arc::new(Mutex::new(None)),
            ..Default::default()
        }
    }
}

impl PortalDialogs for TestPortalDialogs {
    fn pick_file(&self, _clues: &DialogClues) -> Result<Option<PathBuf>, String> {
        Ok(self.single_file.lock().unwrap().clone())
    }

    fn pick_files(&self, _clues: &DialogClues) -> Result<Vec<PathBuf>, String> {
        Ok(self.multi_files.lock().unwrap().clone())
    }

    fn save_file(&self, _clues: &DialogClues) -> Result<Option<PathBuf>, String> {
        Ok(self.save_destination.lock().unwrap().clone())
    }
}

/// El diálogo del PIN: la primera vez se equivoca y, al avisarle, teclea el del token de pruebas.
pub struct MistypesTheTokenSecretOnce;

impl SecretPrompter for MistypesTheTokenSecretOnce {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        match request.incorrect_secret {
            false => Ok(ProtectedSecret::from_str("0000")),
            true => Ok(ProtectedSecret::from_str(THE_TOKEN_SECRET)),
        }
    }
}

/// La mesa del trámite montada sobre las raíces de un rFirma en marcha.
pub fn the_desk_of(roots: &Roots) -> ErrandDesk<'_, Isolate, Isolate, Neighbours<'_>> {
    ErrandDesk {
        engine: &roots.signing.isolate,
        policies: &roots.signing.isolate,
        validation: &roots.signing.isolate,
        neighbours: Neighbours {
            identity: &roots.identity,
            documents: &roots.documents,
            signing: &roots.signing,
        },
        scratch_dir: roots.site.scratch_dir.clone(),
        scratch: roots.site.scratch.clone(),
        batch: roots.site.batch.clone(),
        triphase: roots.site.triphase.clone(),
    }
}

/// El trámite atendiendo la operación del canal, consintiendo con el certificado de pruebas cuando
/// se lo pide, y llevando la cuenta de las veces que lo ha pedido.
pub fn the_errand_of(roots: &Arc<Roots>, consents: &Arc<AtomicUsize>) -> SiteOperations {
    let roots = Arc::clone(roots);
    let consents = Arc::clone(consents);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingForConsent { certificates, .. }) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };

        consents.fetch_add(1, Ordering::SeqCst);
        let chosen = certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just certs install`")
            });
        errand::consent(&desk, &chosen.id, live).expect("el consentimiento deberia entregarse");
    })
}

/// El códec con el que rFirma contesta a una invocación de esa versión.
pub fn the_codec_of(roots: &Roots, launch: &LaunchRequest) -> NegotiatedCodec {
    match launch.version() {
        THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS => Arc::clone(&roots.site.codecs.v4),
        3 => Arc::clone(&roots.site.codecs.v3),
        other => panic!("el guion no habla la version {other}"),
    }
}

/// Abre el canal donde la sede invocó y arranca el trámite que atenderá su operación.
pub async fn the_errand_channel(
    client: &PublishedClient,
    material: &ChannelMaterial,
    roots: &Arc<Roots>,
    operations: SiteOperations,
) -> OpenChannel {
    let url = client.the_launch_url();
    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la invocacion deberia atenderse");

    let location = client.the_channel_location(&launch);
    let channel = the_channel_at(
        &location,
        material,
        ChannelDuty::Serve(launch.credential().clone()),
        operations,
    )
    .await;
    assert!(
        roots.site.errand.begin(
            Errand::of(
                launch.credential().clone(),
                channel.arrival_mode(),
                the_codec_of(roots, &launch),
            )
            .with_tenure(location.tenure())
        ),
        "la invocacion anterior deberia haber cerrado su tramite"
    );
    channel
}

/// El certificado que el `successCallback` del cliente publicado recibió.
pub fn the_certificate_of(event: &Event, step: &str) -> String {
    assert_eq!(
        event.name(),
        "success",
        "la seleccion '{step}' tenia que acabar en el successCallback, y acabo en {}: {}",
        event.name(),
        event.field("message")
    );
    assert_eq!(event.field("step"), step, "las selecciones llegan en orden");
    event.field("data").to_owned()
}
/// El trámite atendiendo `sign`: consiente con el certificado de pruebas, firma en el token con
/// el secreto y apunta el DER del firmante para contrastarlo con el que recibe la sede.
pub fn the_sign_errand_of(
    roots: &Arc<Roots>,
    signer: &Arc<Mutex<Option<Vec<u8>>>>,
) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSign(consent)) = errand::attend(&desk, url, answering, live)
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
        let signing_certificate = roots
            .identity
            .chosen(&chosen.id)
            .expect("el certificado consentido deberia seguir en el token");
        *signer
            .lock()
            .expect("nadie envenena el apunte del firmante") =
            Some(signing_certificate.der().to_vec());

        errand::consent(&desk, &chosen.id, live).expect("la prefirma deberia consentirse");
        tokio::task::block_in_place(|| {
            sign_on_token(
                &roots.identity.signer(),
                &roots.signing.session,
                THE_TOKEN_SECRET,
            )
            .expect("la firma en el token deberia completarse");
            errand::finish(&desk, live).expect("la postfirma deberia completarse");
        });
    })
}

/// El trámite de una firma que consiente, abre el secreto por la única puerta del PIN y entrega, salvo que el puente la rechace.
pub fn the_errand_that_signs_unless_refused(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSign(consent)) = errand::attend(&desk, url, answering, live)
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

        if errand::consent(&desk, &chosen.id, live).is_err() {
            return;
        }
        tokio::task::block_in_place(|| {
            if signed_with_the_secret(&desk, live, THE_TOKEN_SECRET).is_ok() {
                if let Some(ErrandStep::Saving(consent)) =
                    errand::finish(&desk, live).expect("la firma deberia entregarse")
                {
                    saved_through_the_portal(&roots, &desk, &consent, live);
                }
            }
        });
    })
}

/// Los eventos y las condiciones que mide la sede de un guion de firma en v4, hasta su desenlace.
pub async fn the_events_of_a_signing_script(script: &str) -> Vec<Event> {
    the_events_of_a_script_saving_to(script, None).await
}

/// Lo mismo, con el diálogo de guardado del portal eligiendo esa ruta si el trámite guarda.
pub async fn the_events_of_a_script_saving_to(
    script: &str,
    saving_to: Option<&Path>,
) -> Vec<Event> {
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let mut roots = tokio::task::block_in_place(|| a_running_rfirma(home.path()));
    if let Some(path) = saving_to {
        roots.site.portal = Arc::new(TestPortalDialogs::saving_to(path));
    }
    let roots = Arc::new(roots);
    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, script);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_errand_that_signs_unless_refused(&roots),
    )
    .await;

    let mut events = Vec::new();
    loop {
        let event = client.next_event_or_condition();
        let settled = matches!(event.name(), "success" | "error");
        events.push(event);
        if settled {
            break;
        }
    }
    channel.close();
    events
}

/// El trámite atendiendo `saveDataToFile`: abre el diálogo del portal y escribe en disco o cancela.
pub fn the_save_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::Saving(consent)) = errand::attend(&desk, url, answering, live) else {
            return;
        };
        saved_through_the_portal(&roots, &desk, &consent, live);
    })
}

/// Abre el diálogo de guardado del portal y escribe lo que el trámite guarda, o declina si se cancela.
fn saved_through_the_portal(
    roots: &Roots,
    desk: &ErrandDesk<'_, Isolate, Isolate, Neighbours<'_>>,
    consent: &errand::SavingConsent,
    live: &errand::LiveErrand,
) {
    let clues = DialogClues {
        title: consent.title.clone(),
        filename: consent.filename.clone(),
        extensions: consent.extensions.clone(),
        description: consent.description.clone(),
        starting_folder: consent.starting_folder.as_ref().map(PathBuf::from),
    };
    let chosen = roots
        .site
        .portal
        .save_file(&clues)
        .expect("el diálogo del portal para guardar no falla");
    match chosen {
        Some(path) => {
            errand::saved(
                desk.scratch.as_ref(),
                &path,
                &consent.data,
                consent.signer_der.as_deref(),
                live,
            );
        }
        None => {
            errand::decline(live);
        }
    }
}

/// El trámite atendiendo `load` o `multiload`: abre el selector del portal y entrega o cancela.
pub fn the_load_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::Loading(consent)) = errand::attend(&desk, url, answering, live) else {
            return;
        };

        let clues = DialogClues {
            title: consent.title.clone(),
            filename: consent.filename.clone(),
            extensions: consent.extensions.clone(),
            description: consent.description.clone(),
            starting_folder: consent.starting_folder.as_ref().map(PathBuf::from),
        };
        let chosen = if consent.multiple {
            roots
                .site
                .portal
                .pick_files(&clues)
                .expect("el diálogo del portal para cargar no falla")
        } else {
            roots
                .site
                .portal
                .pick_file(&clues)
                .expect("el diálogo del portal para cargar no falla")
                .into_iter()
                .collect()
        };
        if chosen.is_empty() {
            errand::decline(live);
        } else {
            let named: Vec<(String, PathBuf)> = chosen
                .into_iter()
                .map(|path| {
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned());
                    (name, path)
                })
                .collect();
            errand::document_chosen(&desk, &named, live);
        }
    })
}

/// El trámite atendiendo `signandsave`: consiente, firma en el token, y guarda en el portal.
pub fn the_sign_and_save_errand_of(
    roots: &Arc<Roots>,
    signer: &Arc<Mutex<Option<Vec<u8>>>>,
) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSign(consent)) = errand::attend(&desk, url, answering, live)
        else {
            return;
        };

        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| panic!("el token de pruebas no ofreció {THE_TEST_CERTIFICATE}"));
        let signing_certificate = roots
            .identity
            .chosen(&chosen.id)
            .expect("el certificado consentido debería seguir en el token");
        *signer
            .lock()
            .expect("nadie envenena el apunte del firmante") =
            Some(signing_certificate.der().to_vec());

        errand::consent(&desk, &chosen.id, live).expect("la prefirma debería consentirse");
        let saving_step = tokio::task::block_in_place(|| {
            sign_on_token(
                &roots.identity.signer(),
                &roots.signing.session,
                THE_TOKEN_SECRET,
            )
            .expect("la firma en el token debería completarse");
            errand::finish(&desk, live).expect("la postfirma debería completarse")
        });

        let Some(ErrandStep::Saving(saving_consent)) = saving_step else {
            panic!("signandsave tenía que desembocar en ErrandStep::Saving");
        };
        saved_through_the_portal(&roots, &desk, &saving_consent, live);
    })
}

/// El trámite atendiendo una operación que el protocolo rechaza sin pedir consentimiento: la
/// URL se decodifica y la respuesta sale por el canal en el mismo `attend`.
pub fn the_refusing_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;
        let answering = ErrandReply::of(move |text| reply.answer(text));
        errand::attend(&desk, url, answering, live);
    })
}
/// Comprueba que `signature` es el PKCS#1 en SHA-256 de `data`, sin nada alrededor, con la clave del certificado.
pub fn verified_as_a_bare_pkcs1(signature: &[u8], data: &[u8], certificate_der: &[u8]) {
    let key = openssl::x509::X509::from_der(certificate_der)
        .and_then(|certificate| certificate.public_key())
        .expect("el firmante es un certificado X.509 con clave publica");
    let mut verifier = openssl::sign::Verifier::new(openssl::hash::MessageDigest::sha256(), &key)
        .expect("openssl verifica PKCS#1 en SHA-256");
    verifier.update(data).expect("openssl lee los datos");
    assert!(
        verifier.verify(signature).unwrap_or(false),
        "lo que volvio no es el PKCS#1 de los datos con la clave del certificado"
    );
}

/// Comprueba el CMS detached con `openssl cms -verify`, contra el `content` que firmó.
pub fn verified_by_openssl(cms: &[u8], content: &Path) {
    let cms_file = a_der_file(cms);
    let output = Command::new("openssl")
        .args(["cms", "-verify", "-noverify", "-inform", "DER", "-in"])
        .arg(cms_file.path())
        .arg("-binary")
        .arg("-content")
        .arg(content)
        .args(["-out", "/dev/null"])
        .output()
        .expect("falta openssl para el banco de conformidad");
    assert!(
        output.status.success(),
        "openssl cms -verify ha fallado:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba `path` con el oráculo de la grada C (`rfirma-native-bridge/testbench/validate.sh`, #526).
pub fn validated_by_the_reference_tool_at(path: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../rfirma-native-bridge/testbench/validate.sh");
    let output = Command::new(&script)
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("no se ha podido ejecutar {}: {error}", script.display()));
    assert!(
        output.status.success(),
        "validate.sh ha fallado:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba el CMS con el oráculo de la grada C (`rfirma-native-bridge/testbench/validate.sh`, #526).
pub fn validated_by_the_reference_tool(cms: &[u8]) {
    let cms_file = a_der_file(cms);
    validated_by_the_reference_tool_at(cms_file.path());
}

/// Comprueba que el fichero en `path` está bien formado con `xmllint --noout`.
pub fn well_formed_according_to_xmllint(path: &Path) {
    let output = Command::new("xmllint")
        .arg("--noout")
        .arg(path)
        .output()
        .expect("falta xmllint para el banco de conformidad");
    assert!(
        output.status.success(),
        "xmllint ha rechazado el XML:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba que el contenedor nombra la firma CAdES que lleva dentro, en vez de dejar que un
/// ZIP cualquiera pase el banco.
pub fn carries_the_asic_s_binary_signature(container: &[u8]) {
    const ENTRY: &[u8] = b"META-INF/signature.p7s";

    assert!(
        container.windows(ENTRY.len()).any(|window| window == ENTRY),
        "al contenedor le falta {}",
        String::from_utf8_lossy(ENTRY)
    );
}

/// Comprueba que `xml` trae un `ds:Signature` del espacio de nombres XMLDSig, en vez de dejar
/// que un XML simplemente bien formado pase el banco sin firma.
pub fn carries_a_xmldsig_signature(xml: &[u8]) {
    let xml = String::from_utf8_lossy(xml);
    assert!(
        xml.contains("http://www.w3.org/2000/09/xmldsig#") && xml.contains(":Signature"),
        "el XML no trae un elemento Signature del espacio de nombres XMLDSig:\n{xml}"
    );
}
