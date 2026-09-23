use super::super::*;
use super::fixtures::{a_codec_table, a_launch, World, CREDENTIAL};
use crate::site::domain::channel::{ChannelDuty, ChannelLocation, Delivery, OpenChannel, Shutdown};

#[test]
fn a_relay_launch_opens_its_window_hidden_and_leaves_showing_it_to_the_operation() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();
    let relay_url = "afirma://open?id=123456&stservlet=https://example.com/store&dat=dGVzdA==";

    let attendance = attend_site_launch(
        relay_url,
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Immediate"],
        "relay no tiene canal que esperar, pero tampoco nada que decir hasta que llegue la operación"
    );
}

#[test]
fn an_immediate_arrival_arms_no_backing_timeout_even_though_the_channel_has_a_port() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let transport = |_location: &ChannelLocation, _duty: ChannelDuty| {
        world.note("canal");
        Ok(OpenChannel::with_delivery(
            54001,
            Shutdown::of(|| {}),
            Delivery::of(|| {}),
        ))
    };

    let attendance = attend_site_launch_with_threshold(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(20),
    );
    std::thread::sleep(Duration::from_millis(60));

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Immediate"],
        "una llegada inmediata no espera a ningún navegador aunque el canal tenga puerto"
    );
}

#[test]
fn immediate_delivery_of_the_operation_preserves_its_moment_when_opening_the_window() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    let relay_url = "afirma://open?id=123456&stservlet=https://example.com/store&dat=dGVzdA==";

    let delivered_moment = Moment::AskingForConsent {
        certificates: Vec::new(),
    };

    let live_for_delivery = Arc::clone(&live);
    let moment_to_deliver = delivered_moment.clone();
    let transport = move |_location: &ChannelLocation, _duty: ChannelDuty| {
        let live = Arc::clone(&live_for_delivery);
        let moment = moment_to_deliver.clone();
        let delivery = Delivery::of(move || {
            live.note(moment);
        });
        Ok(OpenChannel::with_delivery(0, Shutdown::of(|| {}), delivery))
    };

    let attendance = attend_site_launch(
        relay_url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        live.moment(),
        Some(delivered_moment),
        "la apertura de la ventana no debe pisar el momento dejado por la entrega de la operacion"
    );
}

#[test]
fn a_relay_launch_with_fileid_and_stservlet_in_url_preserves_the_delivered_moment() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    let relay_url = "afirma://sign?id=TX1&fileid=FID1&rtservlet=https://example.com/retrieve&key=12345678&stservlet=https://example.com/store";

    let delivered_moment = Moment::AskingToSign {
        document: "doc.pdf".to_owned(),
        format: crate::signing::domain::bridge::Format::Pades,
        round: crate::site::domain::protocol::SignatureRound::First,
        certificates: Vec::new(),
        unregistered_signatures: false,
        already_chosen: None,
    };

    let live_for_delivery = Arc::clone(&live);
    let moment_to_deliver = delivered_moment.clone();
    let transport = move |location: &ChannelLocation, _duty: ChannelDuty| {
        assert!(matches!(
            location,
            ChannelLocation::Relay(crate::site::domain::protocol::RelayChannelInfo {
                request: crate::site::domain::protocol::RelayRequest::DataByFileId { .. },
                ..
            })
        ));
        let live = Arc::clone(&live_for_delivery);
        let moment = moment_to_deliver.clone();
        let delivery = Delivery::of(move || {
            live.note(moment);
        });
        Ok(OpenChannel::with_delivery(0, Shutdown::of(|| {}), delivery))
    };

    let attendance = attend_site_launch(
        relay_url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        live.moment(),
        Some(delivered_moment),
        "el arranque con stservlet en la URL conserva el momento entregado"
    );
    assert_eq!(world.steps(), ["ventana:creada:Immediate"]);
}

#[test]
fn a_relay_launch_with_fileid_and_parameters_xml_preserves_the_delivered_moment() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    let relay_url = "afirma://sign?fileid=FID2&rtservlet=https://example.com/retrieve&key=12345678";

    let delivered_moment = Moment::AskingToSign {
        document: "doc.pdf".to_owned(),
        format: crate::signing::domain::bridge::Format::Pades,
        round: crate::site::domain::protocol::SignatureRound::First,
        certificates: Vec::new(),
        unregistered_signatures: false,
        already_chosen: None,
    };

    let live_for_delivery = Arc::clone(&live);
    let moment_to_deliver = delivered_moment.clone();
    let transport = move |location: &ChannelLocation, _duty: ChannelDuty| {
        assert!(matches!(
            location,
            ChannelLocation::Relay(crate::site::domain::protocol::RelayChannelInfo {
                request: crate::site::domain::protocol::RelayRequest::ParametersByFileId { .. },
                ..
            })
        ));
        let live = Arc::clone(&live_for_delivery);
        let moment = moment_to_deliver.clone();
        let delivery = Delivery::of(move || {
            live.note(moment);
        });
        Ok(OpenChannel::with_delivery(0, Shutdown::of(|| {}), delivery))
    };

    let attendance = attend_site_launch(
        relay_url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        live.moment(),
        Some(delivered_moment),
        "el arranque con parametros por fileid conserva el momento entregado"
    );
    assert_eq!(world.steps(), ["ventana:creada:Immediate"]);
}

#[test]
fn a_relay_launch_is_attended_even_without_a_local_ca() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    let relay_url = "afirma://open?id=123456&stservlet=https://example.com/store&dat=dGVzdA==";

    let delivered_moment = Moment::AskingForConsent {
        certificates: Vec::new(),
    };

    let live_for_delivery = Arc::clone(&live);
    let moment_to_deliver = delivered_moment.clone();
    let transport = move |_location: &ChannelLocation, _duty: ChannelDuty| {
        let live = Arc::clone(&live_for_delivery);
        let moment = moment_to_deliver.clone();
        let delivery = Delivery::of(move || {
            live.note(moment);
        });
        Ok(OpenChannel::with_delivery(0, Shutdown::of(|| {}), delivery))
    };

    let attendance = attend_site_launch(
        relay_url,
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::Nowhere,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        live.moment(),
        Some(delivered_moment),
        "el servidor intermedio no pasa por el canal local: la falta de su CA no tapa la operación"
    );
    assert_eq!(world.steps(), ["ventana:creada:Immediate"]);
}

#[test]
fn the_backing_timeout_expires_and_reveals_the_unreachable_window() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch_with_threshold(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(40),
    );

    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Awaited"],
        "recién arrancado no está enseñada"
    );

    world.wait_until_shown();

    assert!(
        live.is_revealed(),
        "el temporizador de respaldo debió revelar la ventana"
    );
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Awaited", "ventana:enseñada"],
        "el timeout de respaldo enseña la ventana"
    );
    assert_eq!(
        live.moment(),
        Some(Moment::Unreachable),
        "el momento anotado es Unreachable"
    );
}

#[test]
fn browser_arrival_before_timeout_reveals_and_disarms_the_backup_timer() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch_with_threshold(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(100),
    );

    assert_eq!(world.steps(), ["canal", "ventana:creada:Awaited"]);

    // Llega el navegador
    live.browser_arrived();

    assert!(live.is_revealed());
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Awaited", "ventana:enseñada"]
    );
    assert_eq!(live.moment(), Some(Moment::Waiting));

    // Esperamos a que pase el tiempo del timeout original
    std::thread::sleep(Duration::from_millis(150));

    // Comprobamos que no se volvió a enseñar ni cambió a Unreachable
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Awaited", "ventana:enseñada"],
        "no hay segunda llamada a show"
    );
    assert_eq!(live.moment(), Some(Moment::Waiting));
}
