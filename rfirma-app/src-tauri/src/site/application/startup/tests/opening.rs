use super::super::*;
use super::fixtures::{
    a_codec_table, a_launch, a_store, invoked_with, starting_with, World, CREDENTIAL,
};
use crate::site::application::tests::InMemoryCaSlots;
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::protocol::RefusalSituation;

#[test]
fn a_site_launch_attends_the_errand_and_never_shows_the_main_window() {
    let world = Arc::new(World::default());
    let store = a_store();
    store
        .write_serving(&LocalCa::generate().expect("una CA local se genera sin depender de nada"))
        .expect("la ranura de pruebas admite escritura");
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::Serving { .. })
        ),
        "una invocación de sede atiende el trámite y no enseña la principal: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["canal".to_owned(), "ventana:creada:Awaited".to_owned()],
        "un lanzamiento de sede abre el canal y la ventana, sin tocar la CA local"
    );
}

#[test]
fn a_pdf_shows_the_main_window_and_never_reaches_the_transport() {
    let world = Arc::new(World::default());
    let store = a_store();
    let invocation = invoked_with(&["/tmp/contrato.pdf"]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(startup.opening, Opening::TheMainWindow),
        "un documento abre la ventana principal: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["confianza"],
        "sin sede no se ata ningún canal ni se abre ninguna ventana de sede"
    );
}

#[test]
fn starting_with_nothing_shows_the_main_window() {
    let world = Arc::new(World::default());
    let store = a_store();

    let startup = starting_with(&world, &store, &invoked_with(&[]));

    assert!(matches!(startup.opening, Opening::TheMainWindow));
    assert_eq!(world.steps(), ["confianza"]);
}

#[test]
fn a_refused_launch_opens_the_hidden_window_and_arms_the_channel_refusal_wait() {
    let world = Arc::new(World::default());
    let store = a_store();
    let invocation = invoked_with(&[&a_launch(&format!("v=99&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::RefusingOverTheChannel { .. })
        ),
        "el rechazo sale por el socket: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["canal", "ventana:rechazo:SAF_21"],
        "un rechazo por el canal abre la ventana oculta sin enseñarla, y no toca la CA local"
    );
}

#[test]
fn a_site_launch_never_writes_to_the_local_ca_slots_or_trust_stores() {
    let world = Arc::new(World::default());
    let store = InMemoryCaSlots::unwritable_serving(
        LocalCa::generate().expect("una CA local se genera sin depender de ningún almacén"),
    );
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::Serving { .. })
        ),
        "una CA sana atiende el trámite aunque las ranuras no admitan escritura: {:?}",
        startup.opening
    );
    assert!(
        !world.steps().contains(&"confianza".to_owned()),
        "el lanzamiento de sede solo lee la CA local, nunca la instala: {:?}",
        world.steps()
    );
}

#[test]
fn a_site_launch_with_six_days_left_shows_the_local_ca_dead_end() {
    let world = Arc::new(World::default());
    let store = a_store();
    store
        .write_serving(&LocalCa::valid_for_days_for_test(6).expect("la CA de prueba se genera"))
        .expect("la ranura de pruebas admite escritura");
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::Serving { .. })
        ),
        "el canal se abre igual: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["canal", "ventana:sin-ca", "ventana:enseñada"],
        "a seis días de caducar, el callejón se enseña igual que sin CA"
    );
}

#[test]
fn a_site_launch_with_eight_days_left_attends_the_errand() {
    let world = Arc::new(World::default());
    let store = a_store();
    store
        .write_serving(&LocalCa::valid_for_days_for_test(8).expect("la CA de prueba se genera"))
        .expect("la ranura de pruebas admite escritura");
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::Serving { .. })
        ),
        "{:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["canal".to_owned(), "ventana:creada:Awaited".to_owned()],
        "a ocho días de caducar, el trámite se atiende sin callejón"
    );
}

#[test]
fn attending_a_site_launch_with_a_live_errand_gets_no_window_of_its_own() {
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

    let attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(
        matches!(attendance, Attendance::RefusingOverTheChannel { .. }),
        "el que llega se entera por su propio canal: {attendance:?}"
    );
    assert!(
        !world.steps().iter().any(|step| step.starts_with("ventana")),
        "no hay segunda ventana de sede: {:?}",
        world.steps()
    );
}

#[test]
fn attending_a_site_launch_directly_never_touches_the_trust_stores() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal".to_owned(), "ventana:creada:Awaited".to_owned()],
        "ni un almacén se abre en la segunda invocación"
    );
}

#[test]
fn every_port_taken_shows_the_dead_end_in_the_site_window() {
    let world = Arc::new(World {
        every_port_taken: true,
        ..World::default()
    });
    let store = a_store();
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::ChannelNotOpened(_))
        ),
        "no se ha podido abrir el canal: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["canal", "ventana:rechazo:SAF_45", "ventana:enseñada"],
        "el desenlace no se pierde: se enseña en la ventana"
    );
}

#[test]
fn every_port_taken_tells_the_person_the_ports_are_taken_instead_of_blaming_the_browser() {
    let world = Arc::new(World {
        every_port_taken: true,
        ..World::default()
    });
    let live = LiveErrand::default();

    let _ = attend_site_launch(
        &format!("afirma://service?ports=63131,63132,63133&v=1&idsession={CREDENTIAL}"),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    let Some(Moment::RefusedWithoutChannel(refusal)) = live.moment() else {
        panic!(
            "la ventana dice que no pudo abrir el canal: {:?}",
            live.moment()
        );
    };
    assert_eq!(refusal.situation(), RefusalSituation::PortsTaken);
    assert!(
        refusal.detail().contains("ocupados"),
        "el detalle es el del fallo al ligar: {}",
        refusal.detail()
    );
}

#[test]
fn a_launch_without_ports_shows_its_refusal_in_the_window() {
    let world = Arc::new(World::default());
    let store = a_store();
    let invocation = invoked_with(&[&format!("afirma://websocket?v=4&idsession={CREDENTIAL}")]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::RefusingInTheWindow(_))
        ),
        "sin puertos el rechazo es de la ventana: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["ventana:rechazo:SAF_03", "ventana:enseñada"],
        "sin puertos no se intenta abrir ningun socket ni tocar la CA local"
    );
}

#[test]
fn a_local_ca_that_reached_no_store_is_the_dead_end_the_window_shows() {
    let world = Arc::new(World::default());
    let store = a_store();
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let live = LiveErrand::default();
    let startup = attend_startup(
        invocation.site_launch(),
        TrustAtStartup {
            store: &store,
            profiles: &[],
            stores: &*world,
        },
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
    );

    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::Serving { .. })
        ),
        "el canal se abre igual: {:?}",
        startup.opening
    );
    assert_eq!(
        world.steps(),
        ["canal", "ventana:sin-ca", "ventana:enseñada"],
        "lo que se enseña es el callejon, no la espera"
    );
    assert!(
        startup.said.is_empty(),
        "un lanzamiento de sede solo lee la CA local, sin narrar nada: {:?}",
        startup.said
    );
}

#[test]
fn a_websocket_v3_launch_creates_the_window_hidden_and_does_not_show_it() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let attendance = attend_site_launch(
        &a_launch(&format!("v=3&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Awaited"],
        "crea la ventana oculta y no la muestra al arrancar"
    );
}

#[test]
fn a_service_launch_creates_the_window_hidden_and_does_not_show_it() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();
    let service_url =
        format!("afirma://service?ports=51001,51002,51003&v=1&idsession={CREDENTIAL}");

    let attendance = attend_site_launch(
        &service_url,
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Awaited"],
        "service crea la ventana oculta y no la muestra al arrancar"
    );
}
