use super::super::*;
use super::fixtures::{a_codec_table, a_launch, World, CREDENTIAL};
use crate::site::domain::protocol::SafCode;

#[test]
fn serving_the_retained_refusal_ends_the_wait_and_shows_the_refusal() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch_with_threshold(
        &a_launch(&format!("v=99&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(200),
    );

    assert_eq!(world.steps(), ["canal", "ventana:rechazo:SAF_21"]);

    // El navegador llega y se le sirve el rechazo retenido.
    live.browser_arrived();

    assert_eq!(
        world.steps(),
        ["canal", "ventana:rechazo:SAF_21", "ventana:enseñada"],
        "servir el rechazo lo enseña sin esperar el plazo"
    );
    assert!(
        matches!(live.moment(), Some(Moment::RefusedWithoutChannel(ref refusal)) if refusal.code() == SafCode::UnsupportedProcedure),
        "la ventana enseña el rechazo servido: {:?}",
        live.moment()
    );
}

#[test]
fn the_channel_refusal_wait_expires_and_shows_the_refusal_too() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch_with_threshold(
        &a_launch(&format!("v=99&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
        Duration::from_millis(30),
    );

    for _ in 0..20 {
        if live.is_revealed() {
            break;
        }
        std::thread::yield_now();
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(
        world.steps(),
        ["canal", "ventana:rechazo:SAF_21", "ventana:enseñada"],
        "al vencer el plazo sin servirse, el rechazo se enseña igualmente"
    );
    assert!(
        matches!(live.moment(), Some(Moment::RefusedWithoutChannel(_))),
        "vencer el plazo no lo convierte en «la petición no ha llegado»: {:?}",
        live.moment()
    );
}

#[test]
fn ending_a_service_errand_notifies_the_window_it_kept() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch(
        &format!("afirma://service?ports=51001,51002,51003&v=1&idsession={CREDENTIAL}"),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert_eq!(
        world.steps(),
        ["canal".to_owned(), "ventana:creada:Awaited".to_owned()]
    );

    crate::site::application::errand::replies::declined(&live);

    assert_eq!(
        world.steps(),
        [
            "canal".to_owned(),
            "ventana:creada:Awaited".to_owned(),
            "ventana:trámite-terminado".to_owned(),
        ],
        "al terminar el trámite, la ventana que se guardó al empezar recibe el aviso"
    );
}

#[test]
fn a_websocket_errand_outlives_its_answer_and_closes_its_window_when_the_first_client_leaves() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );
    live.browser_arrived();
    crate::site::application::errand::replies::declined(&live);

    assert_eq!(
        world.steps(),
        [
            "canal".to_owned(),
            "ventana:creada:Awaited".to_owned(),
            "ventana:enseñada".to_owned(),
        ],
        "contestar no termina un trámite de WebSocket"
    );
    assert!(live.current().is_some());

    live.the_first_client_left();

    assert_eq!(
        world.steps(),
        [
            "canal".to_owned(),
            "ventana:creada:Awaited".to_owned(),
            "ventana:enseñada".to_owned(),
            "ventana:cerrada".to_owned(),
        ],
        "irse el primer cliente cierra la ventana, y con ella el proceso"
    );
}

#[test]
fn a_websocket_errand_whose_browser_never_arrived_ends_like_any_other() {
    let world = Arc::new(World::default());
    let live = LiveErrand::default();

    let _attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );
    crate::site::application::errand::answer_before_closing(&live);

    assert!(live.current().is_none());
    assert_eq!(
        world.steps(),
        [
            "canal".to_owned(),
            "ventana:creada:Awaited".to_owned(),
            "ventana:trámite-terminado".to_owned(),
        ],
        "sin navegador, cerrar la ventana termina el trámite y el proceso"
    );
}
