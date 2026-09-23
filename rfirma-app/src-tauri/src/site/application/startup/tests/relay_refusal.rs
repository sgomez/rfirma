use super::super::*;
use super::fixtures::{a_codec_table, World, CREDENTIAL};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, Delivery, OpenChannel, Shutdown,
};

#[test]
fn a_relay_refusal_with_a_known_destination_fires_its_delivery_and_ends_the_errand_unseen() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();
    let world_for_transport = Arc::clone(&world);

    let transport =
        move |location: &ChannelLocation, duty: ChannelDuty| -> Result<OpenChannel, ChannelError> {
            world_for_transport.note("canal");
            let ChannelDuty::Refuse(_) = duty else {
                panic!("esta invocacion siempre se rechaza en negociacion: {duty:?}");
            };
            assert!(matches!(location, ChannelLocation::Relay(_)));
            let uploaded = Arc::clone(&world_for_transport);
            Ok(OpenChannel::with_delivery(
                0,
                Shutdown::of(|| {}),
                Delivery::of(move || uploaded.note("subida")),
            ))
        };

    let url = "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=https://relay.\
               example/store&id=tx1&ver=99";

    let attendance = attend_site_launch_with_threshold(
        url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(200),
    );

    assert!(
        matches!(attendance, Attendance::RefusingOverTheChannel { .. }),
        "el destino ya se conocia en la url: se rechaza por el canal"
    );
    assert_eq!(
        world.steps(),
        [
            "canal",
            "ventana:rechazo:SAF_21",
            "subida",
            "ventana:trámite-terminado"
        ],
        "la llegada inmediata dispara la subida y termina el tramite sin enseñar la ventana"
    );
}

#[test]
fn a_relay_refusal_whose_upload_fails_shows_the_window_and_does_not_end_the_errand() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();
    let world_for_transport = Arc::clone(&world);

    let transport =
        move |_: &ChannelLocation, _: ChannelDuty| -> Result<OpenChannel, ChannelError> {
            world_for_transport.note("canal");
            let uploaded = Arc::clone(&world_for_transport);
            Ok(OpenChannel::with_delivery(
                0,
                Shutdown::of(|| {}),
                Delivery::fallible(move || {
                    uploaded.note("subida-fallida");
                    false
                }),
            ))
        };

    let url = "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=https://relay.\
               example/store&id=tx1&ver=99";

    attend_site_launch_with_threshold(
        url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(200),
    );

    assert_eq!(
        world.steps(),
        [
            "canal",
            "ventana:rechazo:SAF_21",
            "subida-fallida",
            "ventana:enseñada"
        ],
        "la sede no se entero del rechazo: lo ve la persona, y el tramite no termina solo"
    );
}

#[test]
fn a_relay_refusal_by_an_errand_in_flight_still_fires_its_delivery_unseen() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();
    assert!(
        live.begin(Errand::of(
            crate::site::domain::protocol::NegotiatedCredential::Required(
                crate::site::domain::protocol::ChannelCredential::parse(CREDENTIAL)
                    .expect("la credencial es buena"),
            ),
            ArrivalMode::Awaited,
            std::sync::Arc::new(crate::site::adapters::codec::V4Codec),
        )),
        "el primero se queda con la plaza"
    );

    let world_for_transport = Arc::clone(&world);
    let transport =
        move |location: &ChannelLocation, duty: ChannelDuty| -> Result<OpenChannel, ChannelError> {
            world_for_transport.note("canal");
            match duty {
                ChannelDuty::Serve(_) => Ok(OpenChannel::new(0, Shutdown::of(|| {}))),
                ChannelDuty::Refuse(_) => {
                    assert!(matches!(location, ChannelLocation::Relay(_)));
                    let uploaded = Arc::clone(&world_for_transport);
                    Ok(OpenChannel::with_delivery(
                        0,
                        Shutdown::of(|| {}),
                        Delivery::of(move || uploaded.note("subida")),
                    ))
                }
            }
        };

    let relay_url = "afirma://sign?id=TX1&fileid=FID1&rtservlet=https://example.com/retrieve&key=12345678&stservlet=https://example.com/store";

    let attendance = attend_site_launch_with_threshold(
        relay_url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(200),
    );

    assert!(
        matches!(attendance, Attendance::RefusingOverTheChannel { .. }),
        "el trámite en curso se rechaza por su propio canal, con el destino que ya traía la URL: \
         {attendance:?}"
    );
    assert_eq!(
        world.steps(),
        ["canal", "canal", "subida"],
        "la subida se dispara aunque ya haya un trámite vivo, sin ventana ni fin de trámite \
         propios"
    );
}
