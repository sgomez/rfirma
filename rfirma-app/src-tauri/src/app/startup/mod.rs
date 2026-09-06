//! Gestión del flujo de arranque de la aplicación y atención de invocaciones de sede (ADR-0005).

pub mod channel;
pub mod repair;

use std::path::PathBuf;

use crate::tls::LocalCaStore;
use crate::trust::{Moment as TrustMoment, TrustStores};

use crate::protocol::Refusal;

use super::errand::{Errand, LiveErrand, Moment, NoChannel};
use super::invocation::Invocation;
use super::site::{self, Attendance, ChannelTransport};
use super::trust;

pub use channel::{hold_the_channel, HeldChannel};
pub use repair::{repair_the_local_ca, LocalCaTrust};

/// Función para abrir y notificar el contenido de la ventana de sede.
pub type SiteWindowOpener<'a> = &'a dyn Fn(SiteWindowContent<'_>);

/// Contenido inicial que debe mostrar la ventana de sede.
#[derive(Debug)]
pub enum SiteWindowContent<'a> {
    /// Trámite activo en servicio.
    TheErrand(&'a Errand),
    /// Trámite bloqueado por una condición irrecuperable.
    ADeadEnd(DeadEnd),
}

/// Situaciones de bloqueo que impiden continuar el trámite con la sede.
#[derive(Debug)]
pub enum DeadEnd {
    /// No se pudo abrir el canal local en los puertos solicitados.
    ChannelNotOpened,
    /// La CA local no está registrada en ningún almacén NSS de confianza (ADR-0005).
    NoLocalCa,
    /// Rechazo de la invocación sin canal disponible para comunicarlo.
    RefusedWithoutChannel(Refusal),
}

/// Estado de disponibilidad de la CA local en los almacenes del sistema (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalCaReach {
    /// La CA local no está presente en ningún almacén revisado.
    Nowhere,
    /// La CA local está presente o no se ha verificado en esta fase.
    NotAnObstacle,
}

/// Parámetros de verificación y almacenes NSS disponibles al arrancar (ADR-0005).
#[derive(Clone, Copy)]
pub struct TrustAtStartup<'a> {
    /// Almacén de la CA local.
    pub store: &'a LocalCaStore,
    /// Rutas de perfiles NSS detectados.
    pub profiles: &'a [PathBuf],
    /// Interfaz de acceso a los almacenes de confianza.
    pub stores: &'a dyn TrustStores,
}

/// Resultado del proceso de arranque de la aplicación.
#[derive(Debug)]
pub struct Startup {
    /// Mensajes informativos sobre el estado de la CA local.
    pub said: Vec<String>,
    /// Ventana seleccionada para abrir en el arranque.
    pub opening: Opening,
}

/// Tipo de ventana que debe abrirse tras evaluar la invocación.
#[derive(Debug)]
pub enum Opening {
    /// Abre la ventana principal para uso local o documento directo.
    TheMainWindow,
    /// Atiende el trámite de sede sin mostrar la ventana principal.
    TheSiteErrand(Attendance),
}

/// Atiende la invocación inicial gestionando la CA local y la ventana correspondiente.
pub fn attend_startup(
    invocation: &Invocation,
    trust: TrustAtStartup<'_>,
    transport: ChannelTransport<'_>,
    window: SiteWindowOpener<'_>,
    live: &LiveErrand,
) -> Startup {
    let (said, local_ca) = refresh_the_local_ca(trust);

    let Some(url) = invocation.site_launch() else {
        return Startup {
            said,
            opening: Opening::TheMainWindow,
        };
    };

    Startup {
        said,
        opening: Opening::TheSiteErrand(attend_site_launch(url, transport, window, live, local_ca)),
    }
}

/// Atiende una invocación de sede y abre la ventana asociada según el resultado.
pub fn attend_site_launch(
    url: &str,
    transport: ChannelTransport<'_>,
    window: SiteWindowOpener<'_>,
    live: &LiveErrand,
    local_ca: LocalCaReach,
) -> Attendance {
    let attendance = site::attend_launch(url, transport, live);

    match &attendance {
        Attendance::Serving { errand, .. } => match local_ca {
            LocalCaReach::Nowhere => open(
                live,
                window,
                SiteWindowContent::ADeadEnd(DeadEnd::NoLocalCa),
            ),
            LocalCaReach::NotAnObstacle => open(live, window, SiteWindowContent::TheErrand(errand)),
        },
        Attendance::ChannelNotOpened(_) => {
            open(
                live,
                window,
                SiteWindowContent::ADeadEnd(DeadEnd::ChannelNotOpened),
            );
        }
        Attendance::RefusingInTheWindow(refusal) => {
            open(
                live,
                window,
                SiteWindowContent::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal.clone())),
            );
        }
        Attendance::RefusingOverTheChannel { .. } => {}
    }

    attendance
}

fn open(live: &LiveErrand, window: SiteWindowOpener<'_>, content: SiteWindowContent<'_>) {
    live.note(content.moment());
    window(content);
}

impl SiteWindowContent<'_> {
    /// Devuelve el momento correspondiente para la ventana de sede.
    pub fn moment(&self) -> Moment {
        match self {
            Self::TheErrand(_) => Moment::Waiting,
            Self::ADeadEnd(DeadEnd::ChannelNotOpened) => {
                Moment::NoChannel(NoChannel::ChannelNotOpened)
            }
            Self::ADeadEnd(DeadEnd::NoLocalCa) => Moment::NoChannel(NoChannel::LocalCaMissing),
            Self::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal)) => {
                Moment::RefusedWithoutChannel(refusal.clone())
            }
        }
    }
}

fn refresh_the_local_ca(trust: TrustAtStartup<'_>) -> (Vec<String>, LocalCaReach) {
    match trust::refresh_local_ca_trust(
        trust.store,
        trust.profiles,
        trust.stores,
        TrustMoment::Startup,
    ) {
        Ok(outcome) => {
            let reach = if outcome.nowhere() {
                LocalCaReach::Nowhere
            } else {
                LocalCaReach::NotAnObstacle
            };
            (
                trust::narrate_startup_outcome(outcome, trust.profiles),
                reach,
            )
        }
        Err(error) => (
            vec![format!(
                "rfirma: no se puede refrescar la CA local ({error}); el arranque sigue sin ella"
            )],
            LocalCaReach::NotAnObstacle,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::channel::{ChannelDuty, ChannelError, OpenChannel, Shutdown, Situation};
    use crate::trust::TrustError;
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

        fn transport(
            &self,
            ports: &[u16],
            _duty: ChannelDuty,
        ) -> Result<OpenChannel, ChannelError> {
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

        fn trust_of(
            &self,
            profile: &Path,
            certificate_der: &[u8],
        ) -> Result<Option<u32>, TrustError> {
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
        let store = LocalCaStore::of(&crate::paths::Paths::under(directory.path()));
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
        let store = LocalCaStore::of(&crate::paths::Paths::under(Path::new(
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
                crate::protocol::ChannelCredential::parse(CREDENTIAL)
                    .expect("la credencial es buena"),
                PORTS[0],
                std::sync::Arc::new(crate::app::codec::V4Codec),
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
}
