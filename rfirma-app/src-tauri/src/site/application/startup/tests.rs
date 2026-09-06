use super::*;

use crate::site::adapters::channel::{ChannelDuty, ChannelError, OpenChannel, Shutdown, Situation};
use crate::site::domain::trust_error::TrustError;
use std::path::Path;
use std::sync::Mutex;

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";
const PORTS: [u16; 3] = [51001, 51002, 51003];

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

    fn transport(&self, ports: &[u16], _duty: ChannelDuty) -> Result<OpenChannel, ChannelError> {
        self.note("canal");
        if self.every_port_taken {
            return Err(ChannelError::new(
                Situation::NoDrawnPortIsFree,
                "los tres puertos sorteados estan ocupados",
            ));
        }
        let port = *ports.first().expect("la sede sorteó puertos");
        Ok(OpenChannel::new(port, Shutdown::of(|| {})))
    }

    fn window(&self, content: SiteWindowContent<'_>) {
        self.note(&match content {
            SiteWindowContent::TheErrand(errand) => format!("ventana:{}", errand.port()),
            SiteWindowContent::ADeadEnd(DeadEnd::ChannelNotOpened) => {
                "ventana:sin-puertos".to_owned()
            }
            SiteWindowContent::ADeadEnd(DeadEnd::NoLocalCa) => "ventana:sin-ca".to_owned(),
            SiteWindowContent::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal)) => {
                format!("ventana:rechazo:{}", refusal.code())
            }
        });
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

fn a_store() -> (tempfile::TempDir, LocalCaStore) {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = LocalCaStore::of(&crate::desktop::adapters::paths::Paths::under(
        directory.path(),
    ));
    (directory, store)
}

fn invoked_with(arguments: &[&str]) -> Invocation {
    let mut command_line = vec!["rfirma".to_owned()];
    command_line.extend(arguments.iter().map(|argument| (*argument).to_string()));
    Invocation {
        command_line,
        folder: PathBuf::from("/tmp"),
    }
}

fn a_launch(parameters: &str) -> String {
    format!("afirma://websocket?ports=51001,51002,51003&{parameters}")
}

fn starting_with(world: &World, store: &LocalCaStore, invocation: &Invocation) -> Startup {
    let profiles = [PathBuf::from("/perfiles/firefox")];
    let live = LiveErrand::default();
    attend_startup(
        invocation,
        TrustAtStartup {
            store,
            profiles: &profiles,
            stores: world,
        },
        &|ports, duty| world.transport(ports, duty),
        &|content| world.window(content),
        &live,
    )
}

#[test]
fn a_site_launch_attends_the_errand_and_never_shows_the_main_window() {
    let world = World::default();
    let (_directory, store) = a_store();
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
        [
            "confianza".to_owned(),
            "canal".to_owned(),
            format!("ventana:{}", PORTS[0])
        ],
        "primero la CA local, luego el canal y sólo entonces la ventana de sede"
    );
}

#[test]
fn a_pdf_shows_the_main_window_and_never_reaches_the_transport() {
    let world = World::default();
    let (_directory, store) = a_store();
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
    let world = World::default();
    let (_directory, store) = a_store();

    let startup = starting_with(&world, &store, &invoked_with(&[]));

    assert!(matches!(startup.opening, Opening::TheMainWindow));
    assert_eq!(world.steps(), ["confianza"]);
}

#[test]
fn a_refused_launch_opens_no_site_window() {
    let world = World::default();
    let (_directory, store) = a_store();
    let invocation = invoked_with(&[&a_launch(&format!("v=3&idsession={CREDENTIAL}"))]);

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
        ["confianza", "canal"],
        "un rechazo no abre ventana de sede"
    );
}

#[test]
fn unwritable_local_ca_material_is_said_but_does_not_stop_the_errand() {
    let world = World::default();
    let store = LocalCaStore::of(&crate::desktop::adapters::paths::Paths::under(Path::new(
        "/proc/rfirma-no-se-puede-escribir",
    )));
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let startup = starting_with(&world, &store, &invocation);

    assert!(
        startup
            .said
            .iter()
            .any(|line| line.contains("no se puede refrescar la CA local")),
        "lo que no se puede escribir se dice: {:?}",
        startup.said
    );
    assert!(
        matches!(
            startup.opening,
            Opening::TheSiteErrand(Attendance::Serving { .. })
        ),
        "y el trámite se atiende igual: {:?}",
        startup.opening
    );
}

#[test]
fn a_second_launch_with_a_live_errand_gets_no_window_of_its_own() {
    let world = World::default();
    let live = LiveErrand::default();
    assert!(
        live.begin(Errand::of(
            crate::site::domain::protocol::ChannelCredential::parse(CREDENTIAL)
                .expect("la credencial es buena"),
            PORTS[0],
            std::sync::Arc::new(crate::site::adapters::codec::V4Codec),
        )),
        "el primero se queda con la plaza"
    );

    let attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &|ports, duty| world.transport(ports, duty),
        &|content| world.window(content),
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
fn a_second_invocation_never_touches_the_trust_stores() {
    let world = World::default();
    let live = LiveErrand::default();

    let attendance = attend_site_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &|ports, duty| world.transport(ports, duty),
        &|content| world.window(content),
        &live,
        LocalCaReach::NotAnObstacle,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        world.steps(),
        ["canal".to_owned(), format!("ventana:{}", PORTS[0])],
        "ni un almacén se abre en la segunda invocación"
    );
}

#[test]
fn every_port_taken_shows_the_dead_end_in_the_site_window() {
    let world = World {
        every_port_taken: true,
        ..World::default()
    };
    let (_directory, store) = a_store();
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
        ["confianza", "canal", "ventana:sin-puertos"],
        "el desenlace no se pierde: se enseña en la ventana"
    );
}

#[test]
fn a_launch_without_ports_shows_its_refusal_in_the_window() {
    let world = World::default();
    let (_directory, store) = a_store();
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
        ["confianza", "ventana:rechazo:SAF_03"],
        "sin puertos no se intenta abrir ningun socket"
    );
}

#[test]
fn a_local_ca_that_reached_no_store_is_the_dead_end_the_window_shows() {
    let world = World::default();
    let (_directory, store) = a_store();
    let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

    let live = LiveErrand::default();
    let startup = attend_startup(
        &invocation,
        TrustAtStartup {
            store: &store,
            profiles: &[],
            stores: &world,
        },
        &|ports, duty| world.transport(ports, duty),
        &|content| world.window(content),
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
        ["canal", "ventana:sin-ca"],
        "lo que se enseña es el callejon, no la espera"
    );
    assert!(
        startup.said.iter().any(|line| line.contains("canal local")),
        "y se dice por stderr: {:?}",
        startup.said
    );
}
