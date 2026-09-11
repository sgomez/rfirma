use super::*;
use crate::site::application::tests::InMemoryCaSlots;

use crate::site::domain::channel::{
    ArrivalMode, ChannelDuty, ChannelError, ChannelLocation, Delivery, OpenChannel, Shutdown,
    Situation,
};
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::trust_error::TrustError;
use std::path::Path;
use std::sync::Mutex;

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

#[derive(Default)]
struct World {
    steps: Mutex<Vec<String>>,
    every_port_taken: bool,
    trusted: Mutex<Vec<(std::path::PathBuf, Vec<u8>)>>,
}

impl World {
    fn note(&self, step: &str) {
        self.steps
            .lock()
            .expect("el doble no envenena su cerrojo")
            .push(step.to_owned());
    }

    fn steps(&self) -> Vec<String> {
        self.steps
            .lock()
            .expect("el doble no envenena su cerrojo")
            .clone()
    }

    fn port_of(location: &ChannelLocation) -> u16 {
        match location {
            ChannelLocation::Drawn(ports) | ChannelLocation::Service(ports) => {
                *ports.first().expect("puertos")
            }
            ChannelLocation::Fixed(port) => *port,
            ChannelLocation::Relay(_) => 0,
        }
    }

    fn transport(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        self.note("canal");
        if self.every_port_taken {
            return Err(ChannelError::new(
                Situation::NoDrawnPortIsFree,
                "los tres puertos sorteados estan ocupados",
            ));
        }
        let port = Self::port_of(location);
        if matches!(
            (location, duty),
            (ChannelLocation::Relay(_), ChannelDuty::Serve(_))
        ) {
            return Ok(OpenChannel::with_delivery(
                port,
                Shutdown::of(|| {}),
                Delivery::of(|| {}),
            ));
        }
        Ok(OpenChannel::new(port, Shutdown::of(|| {})))
    }
}

impl SiteWindow for World {
    fn open(&self, content: SiteWindowContent<'_>) {
        self.note(&match content {
            SiteWindowContent::TheErrand(errand) => {
                format!("ventana:creada:{:?}", errand.arrival())
            }
            SiteWindowContent::ADeadEnd(DeadEnd::ChannelNotOpened) => {
                "ventana:sin-puertos".to_owned()
            }
            SiteWindowContent::ADeadEnd(DeadEnd::NoLocalCa) => "ventana:sin-ca".to_owned(),
            SiteWindowContent::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal)) => {
                format!("ventana:rechazo:{}", refusal.code())
            }
        });
    }

    fn show(&self) {
        self.note("ventana:enseñada");
    }

    fn errand_ended(&self, delivered: Acknowledgement) {
        delivered.wait(Duration::from_secs(1));
        self.note("ventana:trámite-terminado");
    }
}

const TRUSTED: u32 = 0x38;

impl TrustStores for World {
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        _nickname: &str,
    ) -> Result<(), TrustError> {
        self.note("confianza");
        self.trusted
            .lock()
            .expect("el doble no envenena su cerrojo")
            .push((profile.to_path_buf(), certificate_der.to_vec()));
        Ok(())
    }

    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError> {
        let installed = self
            .trusted
            .lock()
            .expect("el doble no envenena su cerrojo")
            .iter()
            .any(|(where_, der)| where_ == profile && der == certificate_der);
        Ok(installed.then_some(TRUSTED))
    }
}

fn a_store() -> InMemoryCaSlots {
    InMemoryCaSlots::default()
}

fn a_codec_table() -> crate::site::application::site::CodecTable {
    crate::site::application::site::CodecTable {
        v4: std::sync::Arc::new(crate::site::adapters::codec::V4Codec),
        v3: std::sync::Arc::new(crate::site::adapters::codec_v3::V3Codec),
        v1: std::sync::Arc::new(crate::site::adapters::codec_v1::V1Codec),
        relay: std::sync::Arc::new(|key| {
            std::sync::Arc::new(crate::site::adapters::codec_relay::RelayCodec::new(key))
                as crate::site::application::errand::NegotiatedCodec
        }),
    }
}

fn invoked_with(arguments: &[&str]) -> Invocation {
    let mut command_line = vec!["rfirma".to_owned()];
    command_line.extend(arguments.iter().map(|argument| (*argument).to_string()));
    Invocation {
        command_line,
        folder: PathBuf::from("/tmp"),
    }
}

/// La invocación con la que arranca el proceso; aquí solo importa si trae una llamada de sede.
use crate::desktop::application::invocation::Invocation;

fn a_launch(parameters: &str) -> String {
    format!("afirma://websocket?ports=51001,51002,51003&{parameters}")
}

fn starting_with(world: &Arc<World>, store: &InMemoryCaSlots, invocation: &Invocation) -> Startup {
    let profiles = [PathBuf::from("/perfiles/firefox")];
    let live = LiveErrand::default();
    attend_startup(
        invocation.site_launch(),
        TrustAtStartup {
            store,
            profiles: &profiles,
            stores: &**world,
        },
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(world) as Arc<dyn SiteWindow>,
        &live,
    )
}

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
fn serving_the_retained_refusal_ends_the_wait_and_closes_the_hidden_window() {
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
        [
            "canal",
            "ventana:rechazo:SAF_21",
            "ventana:trámite-terminado"
        ],
        "servir el rechazo cierra la ventana oculta sin esperar el plazo"
    );
}

#[test]
fn the_channel_refusal_wait_expires_and_closes_the_hidden_window_too() {
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
        [
            "canal",
            "ventana:rechazo:SAF_21",
            "ventana:trámite-terminado"
        ],
        "al vencer el plazo sin servirse, la ventana oculta se cierra igualmente"
    );
}

#[test]
fn ending_the_errand_notifies_the_window_it_kept() {
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
        ["canal", "ventana:sin-puertos", "ventana:enseñada"],
        "el desenlace no se pierde: se enseña en la ventana"
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

#[test]
fn a_relay_launch_creates_the_window_and_shows_it_immediately() {
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
        ["canal", "ventana:creada:Immediate", "ventana:enseñada"],
        "relay no tiene canal que esperar: se enseña de inmediato"
    );
}

#[test]
fn an_immediate_arrival_shows_the_window_even_though_the_channel_has_a_port() {
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

    let attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &transport,
        Arc::clone(&world) as Arc<dyn SiteWindow>,
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal", "ventana:creada:Immediate", "ventana:enseñada"],
        "una llegada inmediata enseña la ventana aunque el canal tenga puerto"
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
    assert_eq!(
        world.steps(),
        ["ventana:creada:Immediate", "ventana:enseñada"]
    );
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
    assert_eq!(
        world.steps(),
        ["ventana:creada:Immediate", "ventana:enseñada"]
    );
}

#[test]
fn opening_with_a_dead_end_is_never_suppressed_by_a_delivered_moment() {
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
        Some(Moment::NoChannel(NoChannel::LocalCaMissing)),
        "un callejon sin salida como la falta de CA local siempre se impone sobre la operacion"
    );
    assert_eq!(world.steps(), ["ventana:sin-ca", "ventana:enseñada"]);
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

    // Esperamos a que venza el timeout
    for _ in 0..20 {
        if live.is_revealed() {
            break;
        }
        std::thread::yield_now();
        std::thread::sleep(Duration::from_millis(10));
    }

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
            "ventana:rechazo:SAF_41",
            "subida",
            "ventana:trámite-terminado"
        ],
        "la llegada inmediata dispara la subida y termina el tramite sin enseñar la ventana"
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
