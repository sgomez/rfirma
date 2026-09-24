//! Servidor intermedio (`setForceWSMode(true)`): la URL larga preprocesada y el destino que solo revela el XML de parámetros.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// El servidor intermedio del banco visto desde rFirma: sirve lo que el cliente publicado subió y
/// guarda la respuesta, sin mirar la dirección del servlet, que en el banco es siempre el mismo.
#[derive(Default)]
struct BenchServlets {
    files: Mutex<std::collections::HashMap<String, String>>,
}

impl BenchServlets {
    fn put(&self, id: &str, data: &str) {
        self.files
            .lock()
            .expect("el candado")
            .insert(id.to_owned(), data.to_owned());
    }

    fn get(&self, id: &str) -> Option<String> {
        self.files.lock().expect("el candado").get(id).cloned()
    }
}

impl Servlets for BenchServlets {
    fn retrieve(&self, _service_url: &str, id: &str) -> Result<String, RelayError> {
        self.get(id).ok_or_else(|| {
            RelayError::new(
                RelaySituation::ServletUnreachable,
                format!("el banco no tiene guardado {id}"),
            )
        })
    }

    fn store(&self, _service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        self.put(id, data);
        Ok(())
    }

    fn wait(&self, _service_url: &str, _id: &str) -> Result<(), RelayError> {
        Ok(())
    }
}

/// El documento que el guion `relay` firma, el mismo que arma el conductor.
fn the_document_too_long_for_the_url() -> Vec<u8> {
    let mut document = b"%PDF-1.7\n".to_vec();
    document.extend(std::iter::repeat_n(b'd', 3000));
    document
}

/// La invocación y lo que el cliente publicado subió al servlet, en el orden en el que los emite.
fn the_relay_launch_of(client: &PublishedClient, servlets: &BenchServlets) -> String {
    let mut launch = None;
    for _ in 0..2 {
        let event = client.next_event();
        match event.name() {
            "stored" => servlets.put(event.field("id"), event.field("dat")),
            "launch" => launch = Some(event.field("url").to_owned()),
            _ => panic!("el banco no esperaba este evento: {}", event.0),
        }
    }
    launch.expect("el cliente publicado tiene que lanzar la aplicacion")
}

#[test]
fn the_published_client_forced_to_the_relay_launches_without_stservlet_and_rfirma_reads_it() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client =
        PublishedClient::running_the_script(&material, BenchMode::Relay, THE_RELAY_SIGNATURE);
    let servlets = Arc::new(BenchServlets::default());

    let launch = the_relay_launch_of(&client, &servlets);
    assert!(
        launch.contains("fileid=") && launch.contains("rtservlet=") && launch.contains("key="),
        "el preproceso de URL larga lanza con 'fileid', 'rtservlet' y 'key': {launch}"
    );
    assert!(
        !launch.contains("stservlet=") && !launch.contains("&id="),
        "la sede en modo servidor intermedio no manda 'stservlet' ni 'id' en la URL: {launch}"
    );

    let request = LaunchRequest::parse(&launch).expect("rfirma deberia leer la invocacion");
    let location = request.location().clone();

    let delivered: Arc<Mutex<Option<(AfirmaUrl, ErrandReply)>>> = Arc::new(Mutex::new(None));
    let inbox_delivered = Arc::clone(&delivered);
    let inbox = Inbox::for_operations(move |url, reply| {
        *inbox_delivered.lock().expect("el candado") = Some((url, reply));
    });
    let background = tokio::runtime::Runtime::new().expect("el runtime de la espera activa");
    let relay = Relay::new(
        Arc::clone(&servlets) as Arc<dyn Servlets + Send + Sync>,
        inbox,
        Arc::new(|_| {}),
        background.handle().clone(),
    );

    let mut channel = relay
        .open(&location, ChannelDuty::Serve(NegotiatedCredential::Absent))
        .expect("el arranque por servidor intermedio deberia abrirse");
    channel
        .take_delivery()
        .expect("una operacion Serve siempre trae entrega")
        .now();

    let (operation, reply) = delivered
        .lock()
        .expect("el candado")
        .take()
        .expect("la operacion deberia haberse entregado");
    let stored_at = operation
        .parameter("id")
        .expect("el XML de parametros trae el 'id'")
        .to_owned();
    let SiteOperation::Sign(signing) =
        read_operation(&operation, &HttpDataSource).expect("lee la operacion")
    else {
        panic!("el guion del banco pide una firma");
    };
    assert_eq!(signing.document(), the_document_too_long_for_the_url());

    reply.answer("la-respuesta-del-tramite".to_owned());
    assert_eq!(
        servlets.get(&stored_at).as_deref(),
        Some("la-respuesta-del-tramite"),
        "la respuesta sube con el 'id' que venia dentro del XML de parametros"
    );
}

/// El cliente publicado forzado a servidor intermedio cofirma una factura, que rFirma rechaza
/// solo, sin pedir consentimiento; el destino solo se conoce tras leer el XML de parámetros, y el
/// servlet de guardado falso recibe el `SAF_NN` de ese rechazo.
#[test]
fn the_published_client_forced_to_the_relay_uploads_the_saf_of_a_refused_operation() {
    use rfirma_lib::site::adapters::codec::V4Codec;
    use rfirma_lib::site::adapters::codec_relay::RelayCodec;
    use rfirma_lib::site::adapters::codec_v1::V1Codec;
    use rfirma_lib::site::adapters::codec_v3::V3Codec;
    use rfirma_lib::site::application::errand::LiveErrand;
    use rfirma_lib::site::application::site::{attend_launch, Attendance, CodecTable};
    use rfirma_lib::site::domain::protocol::Refusal;

    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_the_script(
        &material,
        BenchMode::Relay,
        THE_RELAY_REFUSED_OPERATION,
    );
    let servlets = Arc::new(BenchServlets::default());

    let launch = the_relay_launch_of(&client, &servlets);
    assert!(
        launch.contains("fileid=") && launch.contains("rtservlet="),
        "el preproceso de URL larga lanza con 'fileid' y 'rtservlet': {launch}"
    );
    assert!(
        !launch.contains("stservlet=") && !launch.contains("&id="),
        "el destino solo esta dentro del XML de parametros: {launch}"
    );

    let stored_at: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let inbox_stored_at = Arc::clone(&stored_at);
    let inbox = Inbox::for_operations(move |url, reply: ErrandReply| {
        *inbox_stored_at.lock().expect("el candado") =
            Some(url.parameter("id").expect("el xml trae 'id'").to_owned());
        let refusal = read_operation(&url, &HttpDataSource)
            .err()
            .unwrap_or_else(|| Refusal::params("el guion esperaba un rechazo del protocolo"));
        reply.answer(refusal.answer().on_the_wire());
    });
    let background = tokio::runtime::Runtime::new().expect("el runtime de la espera activa");
    let relay = Relay::new(
        Arc::clone(&servlets) as Arc<dyn Servlets + Send + Sync>,
        inbox,
        Arc::new(|_| {}),
        background.handle().clone(),
    );

    let codecs = CodecTable {
        v4: Arc::new(V4Codec),
        v3: Arc::new(V3Codec),
        v1: Arc::new(|version| Arc::new(V1Codec::new(version)) as NegotiatedCodec),
        relay: Arc::new(|key, version| Arc::new(RelayCodec::new(key, version)) as NegotiatedCodec),
    };

    let attendance = attend_launch(
        &launch,
        &codecs,
        &|location, duty| relay.open(location, duty),
        &LiveErrand::default(),
    );

    let Attendance::Serving { mut channel, .. } = attendance else {
        panic!(
            "el destino ya se conocia en el xml de parametros: deberia servir para contestar por \
             el: {attendance:?}"
        );
    };
    channel
        .take_delivery()
        .expect("la llegada del servidor intermedio es inmediata")
        .now();

    let stored_at = stored_at
        .lock()
        .expect("el candado")
        .clone()
        .expect("la operacion deberia haberse leido y rechazado");
    assert_eq!(
        servlets.get(&stored_at),
        Some(WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire()),
        "el servlet de guardado deberia recibir el SAF_NN del rechazo con el 'id' del xml"
    );
}
