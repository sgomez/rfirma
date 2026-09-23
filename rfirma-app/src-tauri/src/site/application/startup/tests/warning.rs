use super::super::*;
use super::fixtures::{a_codec_table, a_launch, World, CREDENTIAL};

fn launching_through_the_warning(world: &Arc<World>, live: &Arc<LiveErrand>, launch: &str) {
    let (world_of_the_launch, live_of_the_launch, url) =
        (Arc::clone(world), Arc::clone(live), launch.to_owned());
    warn_before_launching(
        launch,
        Arc::clone(world) as Arc<dyn SiteWindow>,
        live,
        Box::new(move || {
            attend_site_launch(
                &url,
                &a_codec_table(),
                &|location, duty| world_of_the_launch.transport(location, duty),
                Arc::clone(&world_of_the_launch) as Arc<dyn SiteWindow>,
                &live_of_the_launch,
                LocalCaReach::NotAnObstacle,
            );
        }),
    );
}

#[test]
fn an_old_web_client_warns_before_the_channel_listens() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());

    launching_through_the_warning(
        &world,
        &live,
        &a_launch(&format!("v=4&jvc=0&idsession={CREDENTIAL}")),
    );

    assert_eq!(world.steps(), ["ventana:aviso", "ventana:enseñada"]);
    assert_eq!(live.moment(), Some(Moment::OldWebClient));
    assert!(live.current().is_none());
}

#[test]
fn dismissing_the_warning_opens_the_channel_once_and_the_errand_waits_hidden() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    launching_through_the_warning(
        &world,
        &live,
        &a_launch(&format!("v=4&jvc=0&idsession={CREDENTIAL}")),
    );

    assert!(crate::site::application::errand::dismiss_the_warning(&live));
    assert!(!crate::site::application::errand::dismiss_the_warning(
        &live
    ));

    assert_eq!(
        world.steps(),
        [
            "ventana:aviso",
            "ventana:enseñada",
            "ventana:oculta",
            "canal",
            "ventana:creada:Awaited"
        ]
    );
    assert_eq!(live.moment(), Some(Moment::Waiting));
    assert!(live.current().is_some());
}

#[test]
fn closing_the_window_over_the_warning_opens_the_channel_too() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    launching_through_the_warning(
        &world,
        &live,
        &a_launch(&format!("v=4&jvc=0&idsession={CREDENTIAL}")),
    );

    let after = crate::site::application::errand::answer_before_closing(&live);

    assert_eq!(
        after,
        crate::site::application::errand::WindowAfterClosing::StaysHidden
    );
    assert!(world.steps().contains(&"canal".to_owned()));
    assert!(live.current().is_some(), "el aviso no detiene la operación");
}

#[test]
fn an_old_web_client_warns_before_a_relay_operation_too() {
    let world = Arc::new(World::default());
    let live = Arc::new(LiveErrand::default());
    launching_through_the_warning(
        &world,
        &live,
        "afirma://open?jvc=0&id=123456&stservlet=https://example.com/store&dat=dGVzdA==",
    );
    assert_eq!(world.steps(), ["ventana:aviso", "ventana:enseñada"]);

    crate::site::application::errand::dismiss_the_warning(&live);

    assert_eq!(
        world.steps(),
        [
            "ventana:aviso",
            "ventana:enseñada",
            "ventana:oculta",
            "canal",
            "ventana:creada:Immediate",
            "ventana:enseñada"
        ]
    );
}

#[test]
fn without_a_javascript_version_code_below_one_the_channel_opens_at_once() {
    for jvc in ["&jvc=1", "&jvc=3", "", "&jvc=noesunnumero"] {
        let world = Arc::new(World::default());
        let live = Arc::new(LiveErrand::default());

        launching_through_the_warning(
            &world,
            &live,
            &a_launch(&format!("v=4{jvc}&idsession={CREDENTIAL}")),
        );

        assert_eq!(
            world.steps(),
            ["canal", "ventana:creada:Awaited"],
            "con {jvc:?}"
        );
        assert!(!crate::site::application::errand::dismiss_the_warning(
            &live
        ));
    }
}
