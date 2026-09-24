//! Cliente de canal para probar el canal local de punta a punta (ADR-0005).

#[path = "channel_client/support.rs"]
mod support;
use support::*;

use rfirma_lib::site::adapters::channel::{bind_first_free, THE_PORT_OF_THE_THIRD_PROTOCOL};
use rfirma_lib::site::adapters::codec::V4Codec;
use rfirma_lib::site::application::errand::LiveErrand;
use rfirma_lib::site::application::site::{Attendance, CodecTable};
use rfirma_lib::site::application::startup::{attend_site_launch, LocalCaReach};
use rfirma_lib::site::domain::channel::ChannelLocation;
use rfirma_lib::site::domain::protocol::LaunchRequest;

#[tokio::test]
async fn the_channel_answers_the_echo_with_ok_over_tls() {
    let canal = AChannel::serving_the_echo().await;

    let mut client = canal.a_client().await;

    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
    assert!(
        client.is_still_open().await,
        "contestado el eco, la sede manda la operacion por el mismo canal"
    );
}

#[tokio::test]
async fn a_client_that_speaks_in_the_clear_never_reaches_the_protocol() {
    let canal = AChannel::serving_the_echo().await;
    let port = canal.port();

    let request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .expect("la URL deberia ser una peticion");
    let in_the_clear = tokio::time::timeout(PATIENCE, tokio_tungstenite::connect_async(request))
        .await
        .expect("el intento deberia terminar");

    assert!(
        in_the_clear.is_err(),
        "un `ws://` que llegue al protocolo seria una segunda puerta sin cifrar"
    );
}

#[tokio::test]
async fn a_client_that_does_not_trust_the_local_ca_is_turned_away_at_the_handshake() {
    let canal = AChannel::serving_the_echo().await;

    let without_the_ca = ChannelClient::try_connect(canal.port(), None).await;

    assert!(
        without_the_ca.is_err(),
        "sin la CA local en el almacen, el navegador no abre el canal"
    );
}

#[tokio::test]
async fn an_echo_with_another_credential_is_refused_and_the_channel_keeps_answering() {
    let canal = AChannel::serving_the_echo().await;
    let mut client = canal.a_client().await;

    let answer = client.echo("otraPaginaDelEquipo0").await;

    assert_eq!(
        answer,
        Some("SAF_46: Id de sesion invalido; el parametro que falla es 'idsession'".to_owned())
    );
    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
}

#[tokio::test]
async fn a_launch_with_a_malformed_credential_is_refused_over_the_socket() {
    let refusal = LaunchRequest::parse("afirma://websocket?ports=54001&v=4&idsession=abc-def")
        .expect_err("el idsession no vale");
    assert_eq!(refusal.code(), SafCode::Params);

    let canal = AChannel::serving(ChannelDuty::Refuse(refusal.answer())).await;
    let mut client = canal.a_client().await;

    let answer = client.echo(CREDENTIAL).await;

    assert_eq!(
        answer,
        Some(
            "SAF_03: Error en los parametros de entrada; el parametro que falla es 'idsession'"
                .to_owned()
        )
    );
    assert!(
        !client.is_still_open().await,
        "ese canal no expone ninguna capacidad: contesta y cierra"
    );
}

#[tokio::test]
async fn the_channel_ends_up_on_one_of_the_ports_the_site_drew() {
    let drawn = {
        let mut ports = Vec::new();
        for _ in 0..3 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("puerto efimero");
            ports.push(listener.local_addr().expect("atado").port());
        }
        ports
    };

    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let certificate =
        LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");
    let listener = bind_first_free(&ChannelLocation::Drawn(drawn.clone()))
        .expect("alguno de los tres deberia estar libre");

    let channel = serve(
        listener,
        &certificate,
        ChannelDuty::Serve(NegotiatedCredential::Required(
            ChannelCredential::parse(CREDENTIAL).expect("credencial"),
        )),
        no_operations(),
    )
    .await
    .expect("el canal deberia levantarse");

    assert!(
        drawn.contains(&channel.port()),
        "el canal quedo en {}, que la sede no sorteo",
        channel.port()
    );

    let mut client = ChannelClient::connect(
        channel.port(),
        Some(&ca.certificate_pem().expect("la CA local en PEM")),
    )
    .await;
    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
}

#[tokio::test]
async fn the_channel_answers_on_both_loopbacks() {
    if std::net::TcpListener::bind("[::1]:0").is_err() {
        return;
    }
    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let certificate =
        LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("puerto efimero");
        listener.local_addr().expect("atado").port()
    };
    let listener = bind_first_free(&ChannelLocation::Drawn(vec![port])).expect("estaba libre");
    let channel = serve(
        listener,
        &certificate,
        ChannelDuty::Serve(NegotiatedCredential::Required(
            ChannelCredential::parse(CREDENTIAL).expect("credencial"),
        )),
        no_operations(),
    )
    .await
    .expect("el canal deberia levantarse");
    let ca_pem = ca.certificate_pem().expect("la CA local en PEM");

    for address in ["127.0.0.1", "::1"] {
        let mut client =
            ChannelClient::try_connect_to_the_address(address, channel.port(), &ca_pem)
                .await
                .unwrap_or_else(|error| panic!("por {address} deberia conectar: {error}"));
        assert_eq!(
            client.echo(CREDENTIAL).await,
            Some("OK".to_owned()),
            "por {address}"
        );
    }
}

#[tokio::test]
async fn once_closed_the_channel_accepts_no_new_conversations() {
    let canal = AChannel::serving_the_echo().await;
    let port = canal.port();
    let ca_pem = canal.ca_pem.clone();
    canal.channel.close();

    let mut refused = false;
    for _ in 0..40 {
        if ChannelClient::try_connect(port, Some(&ca_pem))
            .await
            .is_err()
        {
            refused = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(refused, "el canal cerrado seguia aceptando conversaciones");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_site_launch_ends_with_the_echo_answered_over_the_open_channel() {
    let free = {
        let mut ports = Vec::new();
        for _ in 0..2 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("puerto efimero");
            ports.push(listener.local_addr().expect("atado").port());
        }
        ports
    };
    let drawn = [THE_PORT_OF_THE_THIRD_PROTOCOL, free[0], free[1]];

    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let ca_pem = ca.certificate_pem().expect("la CA local en PEM");
    let certificate =
        LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");

    let windows = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let live = LiveErrand::default();
    let url = format!(
        "afirma://websocket?ports={},{},{}&v=4&idsession={CREDENTIAL}",
        drawn[0], drawn[1], drawn[2]
    );

    struct WindowCounter(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl rfirma_lib::site::application::startup::SiteWindow for WindowCounter {
        fn open(&self, _content: rfirma_lib::site::application::startup::SiteWindowContent<'_>) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        fn show(&self) {}
        fn hide(&self) {}
        fn close(&self) {}
        fn errand_ended(&self, _delivered: rfirma_lib::site::ports::Acknowledgement) {}
    }

    let attendance = attend_site_launch(
        &url,
        &CodecTable {
            v4: std::sync::Arc::new(V4Codec),
            v3: std::sync::Arc::new(rfirma_lib::site::adapters::codec_v3::V3Codec),
            v1: std::sync::Arc::new(|version| {
                std::sync::Arc::new(rfirma_lib::site::adapters::codec_v1::V1Codec::new(version))
                    as rfirma_lib::site::application::errand::NegotiatedCodec
            }),
            relay: std::sync::Arc::new(|key, version| {
                std::sync::Arc::new(rfirma_lib::site::adapters::codec_relay::RelayCodec::new(
                    key, version,
                )) as rfirma_lib::site::application::errand::NegotiatedCodec
            }),
        },
        &|ports, duty| {
            let listener = bind_first_free(ports)?;
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(serve(
                    listener,
                    &certificate,
                    duty,
                    no_operations(),
                ))
            })
        },
        std::sync::Arc::new(WindowCounter(std::sync::Arc::clone(&windows))),
        &live,
        LocalCaReach::NotAnObstacle,
    );

    let Attendance::Serving { channel, .. } = &attendance else {
        panic!("la invocacion era buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        free[0],
        "el canal queda en el primero libre que sorteo la sede, y nunca en el 63117"
    );
    assert_eq!(
        windows.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "con trámite hay una ventana de sede, y una sola (ID-334)"
    );

    let mut client = ChannelClient::connect(channel.port(), Some(&ca_pem)).await;

    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
}

/// Más que el máximo de 240s que el original le daba a un lote (`setConnectionLostTimeout`).
const LONGER_THAN_THE_ORIGINAL_BATCH_ALLOWANCE: Duration = Duration::from_secs(245);

/// Mide si el canal `wss` sigue vivo tras una conexión callada larga (issue #499).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "duerme 245s de verdad: mide el timeout real del canal wss (issue #499)"]
async fn a_silent_wss_connection_survives_longer_than_the_original_batch_allowance() {
    if std::env::var_os("RFIRMA_MEASURE_IDLE_TIMEOUT").is_none() {
        eprintln!(
            "omitida sin RFIRMA_MEASURE_IDLE_TIMEOUT=1: just test-native la compila y la \
             ejecuta con --include-ignored, pero no le añade los 245s de esta prueba"
        );
        return;
    }

    let canal = AChannel::serving_the_echo().await;
    let mut client = canal.a_client().await;

    tokio::time::sleep(LONGER_THAN_THE_ORIGINAL_BATCH_ALLOWANCE).await;

    assert_eq!(
        client.echo(CREDENTIAL).await,
        Some("OK".to_owned()),
        "el eco deberia contestarse tras el silencio, sin timeout propio de por medio"
    );
}
