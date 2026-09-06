//! **Qué se abre cuando arranca la aplicación** (ID-324, ID-328…ID-329, TD-70).
//!
//! Es la costura del #390: hasta ahora una invocación de sede caía por el
//! camino del documento —se abría la ventana principal entera y la sede se
//! quedaba esperando para siempre—, porque nadie miraba
//! [`Invocation::site_launch`] al montar la aplicación.
//!
//! Aquí se deciden tres cosas y ninguna más:
//!
//! 1. **Primero la CA local** (ID-329): se refresca antes de atender nada, y
//!    nunca a mitad de un trámite. Sin ella ningún navegador intenta siquiera
//!    abrir el canal.
//! 2. **Si la invocación es de sede, se atiende el trámite** y la ventana
//!    principal **no se enseña** (ID-324, ID-328). Si no lo es, la ventana
//!    principal se enseña y aquí no pasa nada más: el documento que traiga se
//!    recoge por donde siempre ([`super::invocation::invoked_document`]).
//! 3. **La ventana de sede se crea sólo cuando hay trámite** (ID-334): con un
//!    rechazo —o con otro trámite ya vivo— no se abre ninguna.
//!
//! # Tres puertos, y por eso esto se prueba sin Tauri
//!
//! El caso de uso no crea ventanas, no ata sockets y no abre almacenes NSS:
//! recibe [`ChannelTransport`] (ID-326), [`TrustStores`] (ID-329) y
//! [`SiteWindowOpener`] (ID-333), y con eso el arranque de Tauri queda como un
//! adaptador sin decisión dentro. Es el **único seam nuevo** de la spec
//! (TD-70), y es lo que deja la conducta del #390 probada entera en grada A,
//! sin Tauri, sin navegador y sin ventana (TD-71).

use std::path::PathBuf;

use crate::channel::OpenChannel;
use crate::tls::LocalCaStore;
use crate::trust::{Moment, TrustStores};

use crate::protocol::Refusal;

use super::errand::{Errand, LiveErrand};
use super::invocation::Invocation;
use super::site::{self, Attendance, ChannelTransport};
use super::trust;

/// **El abridor de la ventana de sede** (ID-333, ID-334, ID-341): crea la
/// ventana y le publica lo que ha pasado.
///
/// No devuelve nada: si la ventana no se puede crear no hay decisión que tomar
/// aquí, y quien la crea es quien lo cuenta.
pub type SiteWindowOpener<'a> = &'a dyn Fn(SiteWindowContent<'_>);

/// Con qué se abre la ventana de sede.
///
/// **Una invocación de sede acaba siempre en algo que se enseña** (ID-341): o
/// el trámite, o el callejón sin salida en el que quedó. Lo que no puede pasar
/// es que no quede nada, que es el síntoma del #390 —una ventana que no aparece
/// y una web esperando—.
#[derive(Debug)]
pub enum SiteWindowContent<'a> {
    /// El trámite que se quedó con la plaza (ID-280): el canal está en pie y la
    /// petición de la sede llegará por él.
    TheErrand(&'a Errand),
    /// El trámite no puede seguir, y esto es por qué.
    ADeadEnd(DeadEnd),
}

/// Un camino en el que el trámite no puede seguir (ID-341).
///
/// Los tres tienen en común que **no hay socket por el que decirlo**: o no se
/// ha podido atar ninguno, o la sede no sorteó ninguno, o el navegador no va a
/// llegar a intentarlo. Por eso se dicen en la ventana y no por el cable.
#[derive(Debug)]
pub enum DeadEnd {
    /// Todos los puertos que sorteó la sede estaban ocupados (ID-215).
    NoPortLeft,
    /// La CA local no ha quedado en ningún almacén NSS (ID-329): ninguna sede
    /// va a poder abrir el canal, aunque el canal esté en pie.
    NoLocalCa,
    /// El rechazo no tiene por dónde salir: sin `ports` en la URL, o con todos
    /// ocupados. La ventana enseña su situación y su detalle (ID-291).
    RefusedWithoutChannel(Refusal),
}

/// **Si la CA local ha llegado a algún almacén NSS**, y sólo cuando se ha
/// mirado (ID-224, ID-329).
///
/// A mitad de un trámite no se abre ni un perfil, así que la respuesta ahí no
/// es «no está»: es que nadie lo ha medido, y contarle a la persona un fallo
/// que nadie ha medido es peor que callarse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalCaReach {
    /// Se ha mirado y no está en ninguno.
    Nowhere,
    /// O está en alguno, o no se ha mirado.
    NotAnObstacle,
}

/// **Los almacenes de confianza del arranque** (ID-329): dónde está guardada la
/// CA local, qué perfiles NSS hay y quién sabe escribir en ellos.
///
/// Viajan juntos porque las tres cosas son la misma pregunta —«¿va a poder la
/// sede abrir el canal con este navegador?»— y ninguna significa nada sin las
/// otras dos.
#[derive(Clone, Copy)]
pub struct TrustAtStartup<'a> {
    /// Las dos ranuras de la CA local: la que sirve y la del solape.
    pub store: &'a LocalCaStore,
    /// Los perfiles NSS que se intentan recorrer.
    pub profiles: &'a [PathBuf],
    /// Quién registra de verdad en un perfil.
    pub stores: &'a dyn TrustStores,
}

/// En qué queda el arranque.
#[derive(Debug)]
pub struct Startup {
    /// Lo que hay que decir por `stderr` sobre la CA local (ID-329). Vacío es
    /// lo normal: sólo se habla cuando algo ha cambiado o algo falta.
    pub said: Vec<String>,
    /// Qué ventana se abre.
    pub opening: Opening,
}

/// Qué ventana abre esta invocación.
#[derive(Debug)]
pub enum Opening {
    /// Arranque normal —a secas o con un documento—: **se enseña la ventana
    /// principal**, y el documento que traiga se recoge por donde siempre.
    TheMainWindow,
    /// Invocación de sede: **la ventana principal no se enseña** (ID-328) y
    /// esto es en qué quedó el trámite. La ventana de sede, si la hay, ya se
    /// ha abierto por su puerto.
    TheSiteErrand(Attendance),
}

/// **Caso de uso.** Atiende la invocación con la que arrancó este proceso.
///
/// El orden no es negociable (ID-329): **primero la CA local y después el
/// trámite**. Un almacén que no se deja escribir no impide atender: se cuenta
/// y se dice (ID-03).
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

/// **Caso de uso.** Atiende una invocación de sede y, si le queda el trámite,
/// abre su ventana.
///
/// Es lo que comparten el arranque (ID-324) y la segunda invocación sobre la
/// aplicación ya abierta (ID-327): la segunda no refresca la CA local —eso es
/// del arranque y nunca de mitad de un trámite (ID-224, ID-329)— y por lo demás
/// hace exactamente lo mismo, con el mismo transporte y el mismo trámite vivo.
///
/// **La ventana se abre sólo cuando hay trámite** (ID-334), y quién se queda
/// con la plaza lo decide [`LiveErrand::begin`] y nadie más (ID-280): lo que se
/// le publica a la ventana es **el trámite que viene dentro del `Serving`**,
/// que es exactamente el que quedó apuntado. Preguntárselo después a
/// [`LiveErrand::current`] dejaba dos rendijas —un `Serving` sin ventana si
/// `current()` volviera vacío, y publicar un trámite distinto del que se quedó
/// la plaza— que por aquí no existen.
pub fn attend_site_launch(
    url: &str,
    transport: ChannelTransport<'_>,
    window: SiteWindowOpener<'_>,
    live: &LiveErrand,
    local_ca: LocalCaReach,
) -> Attendance {
    let attendance = site::attend_launch(url, transport, live);

    match &attendance {
        // **Con la CA local en ninguna parte el canal está en pie y no sirve de
        // nada** (ID-329, ID-341): el navegador ni llega a intentarlo, así que
        // lo que hay delante es el callejón y no la espera.
        Attendance::Serving { errand, .. } => match local_ca {
            LocalCaReach::Nowhere => window(SiteWindowContent::ADeadEnd(DeadEnd::NoLocalCa)),
            LocalCaReach::NotAnObstacle => window(SiteWindowContent::TheErrand(errand)),
        },
        // Los dos callejones del ID-341. No hay socket por el que decirlo, y
        // que no se diga en ninguna parte es el #390.
        Attendance::ChannelNotOpened(_) => {
            window(SiteWindowContent::ADeadEnd(DeadEnd::NoPortLeft));
        }
        Attendance::RefusingInTheWindow(refusal) => {
            window(SiteWindowContent::ADeadEnd(DeadEnd::RefusedWithoutChannel(
                refusal.clone(),
            )));
        }
        // **Un rechazo con socket no abre ventana** (ID-334): la sede recibe su
        // código por donde preguntó, y ahí se acaba.
        Attendance::RefusingOverTheChannel { .. } => {}
    }

    attendance
}

/// Deja la CA local de confianza donde se pueda, y devuelve lo que hay que
/// decir (ID-329).
///
/// El material que no se puede leer ni escribir tampoco interrumpe el arranque:
/// se dice y se sigue, porque la ventana principal no depende de la CA local
/// para abrirse.
fn refresh_the_local_ca(trust: TrustAtStartup<'_>) -> (Vec<String>, LocalCaReach) {
    match trust::refresh_local_ca_trust(trust.store, trust.profiles, trust.stores, Moment::Startup)
    {
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
        // Un material que no se puede ni leer no dice que la CA no esté en
        // ningún almacén: dice que no se ha podido mirar (ID-224).
        Err(error) => (
            vec![format!(
                "rfirma: no se puede refrescar la CA local ({error}); el arranque sigue sin ella"
            )],
            LocalCaReach::NotAnObstacle,
        ),
    }
}

/// **El canal abierto, sostenido mientras haga falta.**
///
/// No es una decisión, es una consecuencia: soltar un [`OpenChannel`] suelta
/// con él su asa de apagado —el emisor del `oneshot` que espera el servidor—, y
/// la tarea que acepta conexiones termina. Sin alguien que lo guarde, el canal
/// que acaba de abrirse se cierra en cuanto el arranque devuelve, y la sede se
/// queda esperando exactamente igual que en el #390.
///
/// Vive en el estado de Tauri, como el trámite (ID-325). Cuando el asa de
/// respuesta entre en [`LiveErrand`] (ID-321) éste es el sitio del que saldrá.
///
/// # Dos ranuras, y la razón es el ID-280
///
/// Un canal de rechazo (`SAF_45` y cualquier otro del ID-248) **no puede
/// compartir ranura con el del trámite**: cuando llega una segunda invocación
/// con un trámite ya vivo, [`site::attend_launch`] abre un canal nuevo sólo
/// para decir el código, y meterlo donde estaba el del trámite cerraría
/// justamente el canal que está sirviendo al primero —el que llega dejaría
/// fuera al que estaba, que es lo contrario del criterio (ID-279, ID-280) y el
/// síntoma mismo del #390—.
///
/// Así que el que sirve y el que rechaza se guardan aparte: `hold` es del
/// trámite y `hold_a_refusal` del rechazo, y ninguno toca la ranura del otro.
#[derive(Default)]
pub struct HeldChannel {
    /// El canal del trámite que se quedó con la plaza.
    serving: std::sync::Mutex<Option<OpenChannel>>,
    /// El canal abierto sólo para contestar un rechazo por el socket (ID-248).
    refusing: std::sync::Mutex<Option<OpenChannel>>,
}

impl HeldChannel {
    /// Se queda con el canal **del trámite**. El que hubiera sirviendo **se
    /// cierra**: sólo hay un trámite a la vez (ID-280), y si hay uno nuevo
    /// sirviendo es que el anterior terminó y ya no tiene quien lo conteste.
    pub fn hold(&self, channel: OpenChannel) {
        if let Some(previous) = super::lock(&self.serving).replace(channel) {
            previous.close();
        }
    }

    /// Sostiene el canal de un **rechazo** mientras contesta (ID-248).
    ///
    /// Vive lo justo para decir su código: no se le suelta en el acto porque
    /// soltarlo apaga el servidor antes de que la sede llegue a conectarse, y
    /// no se cierra a mano porque nadie sabe aquí cuándo ha contestado. Lo
    /// cierra el rechazo siguiente, y si no llega ninguno, el fin del proceso.
    ///
    /// **Nunca toca el canal del trámite vivo**: un rechazo es exactamente el
    /// caso en el que el anterior sí tiene quien lo conteste.
    pub fn hold_a_refusal(&self, channel: OpenChannel) {
        if let Some(previous) = super::lock(&self.refusing).replace(channel) {
            previous.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;
    use std::sync::Mutex;

    use crate::channel::{ChannelDuty, ChannelError, Shutdown, Situation};
    use crate::trust::TrustError;

    /// La credencial que sortea la sede: veinte alfanuméricos.
    const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

    /// Los puertos que sortea la sede en las pruebas.
    const PORTS: [u16; 3] = [51001, 51002, 51003];

    /// **Grada A**: el mundo entero doblado, y **en el orden en que se le
    /// habla**.
    ///
    /// Los tres puertos apuntan en la misma lista porque lo que el ID-329 fija
    /// es una secuencia —refrescar y luego atender—, y una secuencia no se
    /// comprueba con tres dobles que no se conocen entre sí.
    #[derive(Default)]
    struct World {
        steps: Mutex<Vec<String>>,
        /// Todos los puertos que sorteó la sede están ocupados: el transporte
        /// no ata ni uno.
        every_port_taken: bool,
        /// Las CA locales que han quedado registradas, por perfil.
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

        /// El transporte: apunta que se le pidió el canal y lo abre en el
        /// primero de los puertos, como haría el de producción.
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

        /// El abridor de ventana: apunta con qué se le abre —el puerto del
        /// trámite, o el callejón sin salida que se enseña—.
        fn window(&self, content: SiteWindowContent<'_>) {
            self.note(&match content {
                SiteWindowContent::TheErrand(errand) => format!("ventana:{}", errand.port()),
                SiteWindowContent::ADeadEnd(DeadEnd::NoPortLeft) => {
                    "ventana:sin-puertos".to_owned()
                }
                SiteWindowContent::ADeadEnd(DeadEnd::NoLocalCa) => "ventana:sin-ca".to_owned(),
                SiteWindowContent::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal)) => {
                    format!("ventana:rechazo:{}", refusal.code())
                }
            });
        }
    }

    /// Los bits de confianza de una CA local que **sí** ha quedado registrada,
    /// los mismos que `is_trusted_ssl_ca` acepta.
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

        /// Lo que este perfil sabe de esa CA: nada hasta que se instala, y los
        /// bits de confianza después.
        ///
        /// **Contestar siempre `None` no es un doble más simple, es uno que
        /// miente**: con él `settle_one` concluye que la CA entró sin bits, y
        /// el arranque entero quedaba como si la CA local no hubiera llegado a
        /// ningún almacén.
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

    /// Una CA local recién nacida, en un directorio que muere con la prueba.
    fn a_store() -> (tempfile::TempDir, LocalCaStore) {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = LocalCaStore::of(&crate::paths::Paths::under(directory.path()));
        (directory, store)
    }

    /// Una invocación con esos argumentos, como llega del escritorio.
    fn invoked_with(arguments: &[&str]) -> Invocation {
        let mut command_line = vec!["rfirma".to_owned()];
        command_line.extend(arguments.iter().map(|argument| (*argument).to_string()));
        Invocation {
            command_line,
            folder: PathBuf::from("/tmp"),
        }
    }

    /// La URL de arranque que manda una sede, con esos pares.
    fn a_launch(parameters: &str) -> String {
        format!("afirma://websocket?ports=51001,51002,51003&{parameters}")
    }

    /// Atiende ese arranque contra el mundo doblado.
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

    /// **La guardia del #390** (TD-71), primera mitad: una invocación de sede
    /// **no** abre la ventana del documento y **sí** atiende el trámite.
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

    /// **La guardia del #390** (TD-71), segunda mitad: con un PDF pasa
    /// exactamente lo contrario —se enseña la principal y el transporte no se
    /// toca—.
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

    /// Arrancar a secas es arrancar la aplicación, y nada más.
    #[test]
    fn starting_with_nothing_shows_the_main_window() {
        let world = World::default();
        let (_directory, store) = a_store();

        let startup = starting_with(&world, &store, &invoked_with(&[]));

        assert!(matches!(startup.opening, Opening::TheMainWindow));
        assert_eq!(world.steps(), ["confianza"]);
    }

    /// **ID-334.** Un rechazo no es un trámite: no se abre ninguna ventana de
    /// sede, aunque el canal se haya abierto para contestar por él (ID-248).
    #[test]
    fn a_refused_launch_opens_no_site_window() {
        let world = World::default();
        let (_directory, store) = a_store();
        // El protocolo 3 no existe en rfirma: es un rechazo del protocolo, y
        // como la sede sorteó puertos se contesta por el socket.
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

    /// **ID-329.** El material de la CA local que no se puede escribir no
    /// impide atender: se dice y el trámite sigue su camino.
    #[test]
    fn unwritable_local_ca_material_is_said_but_does_not_stop_the_errand() {
        let world = World::default();
        // Un directorio de datos donde no se puede escribir: la CA local no se
        // puede guardar, que es el único fallo que sale por el `Result`.
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

    /// **ID-280, ID-327.** Con un trámite ya vivo el que llega se queda fuera,
    /// y no se le abre ninguna ventana: la plaza la reparte
    /// [`LiveErrand::begin`] y nadie más.
    #[test]
    fn a_second_launch_with_a_live_errand_gets_no_window_of_its_own() {
        let world = World::default();
        let live = LiveErrand::default();
        assert!(
            live.begin(Errand::of(
                crate::protocol::ChannelCredential::parse(CREDENTIAL)
                    .expect("la credencial es buena"),
                PORTS[0]
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

    /// La segunda invocación **no toca los almacenes NSS** (ID-224, ID-329):
    /// refrescar es cosa del arranque, y aquí puede haber un trámite en marcha.
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

    /// **ID-341.** Con todos los puertos sorteados ocupados no hay canal por el
    /// que hablar, y el desenlace **se enseña en la ventana**: el #390 era
    /// justamente que no se enseñaba en ninguna parte.
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

    /// **ID-341.** Sin `ports` en la URL el rechazo tampoco tiene socket por el
    /// que salir: va a la ventana, y **al transporte no se le pide nada**.
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

    /// **ID-329, ID-341.** La CA local que no ha entrado en ningún almacén NSS
    /// llega a la ventana como lo que es: el desenlace que impide abrir el
    /// canal, aunque el canal esté en pie.
    #[test]
    fn a_local_ca_that_reached_no_store_is_the_dead_end_the_window_shows() {
        let world = World::default();
        let (_directory, store) = a_store();
        let invocation = invoked_with(&[&a_launch(&format!("v=4&idsession={CREDENTIAL}"))]);

        // Sin ningún perfil NSS que recorrer, la CA local no queda en ninguno:
        // es la misma conclusión medida que `TrustOutcome::nowhere`.
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

    /// Un canal que apunta su cierre: es lo único que hace falta para ver a
    /// [`HeldChannel`] por dentro, porque cerrar es lo único que hace.
    fn a_channel(port: u16, closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> OpenChannel {
        let closed = std::sync::Arc::clone(closed);
        OpenChannel::new(
            port,
            Shutdown::of(move || super::super::lock(&closed).push(port)),
        )
    }

    /// Qué puertos se han cerrado hasta ahora.
    fn closed_ports(closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> Vec<u16> {
        super::super::lock(closed).clone()
    }

    /// **ID-279, ID-280.** Sostener el canal de un rechazo **no cierra el del
    /// trámite vivo**: con un trámite en marcha, el que llega se queda fuera, y
    /// eso es exactamente lo contrario de que el que llega eche al que estaba.
    ///
    /// Es la mitad que no ve
    /// [`a_second_launch_with_a_live_errand_gets_no_window_of_its_own`]: ésa es
    /// de grada A sobre el caso de uso y no llega hasta la ranura.
    #[test]
    fn a_refusal_never_closes_the_channel_of_the_live_errand() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold(a_channel(PORTS[0], &closed));
        held.hold_a_refusal(a_channel(PORTS[1], &closed));

        assert!(
            closed_ports(&closed).is_empty(),
            "el canal del trámite vivo sigue sirviendo: {:?}",
            closed_ports(&closed)
        );
    }

    /// Un rechazo detrás de otro sí cierra al anterior: el primero ya contestó
    /// lo suyo, y su puerto no tiene por qué seguir atado.
    #[test]
    fn a_new_refusal_closes_the_refusal_it_replaces() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold_a_refusal(a_channel(PORTS[0], &closed));
        held.hold_a_refusal(a_channel(PORTS[1], &closed));

        assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
    }

    /// **ID-280.** Y un trámite nuevo sí cierra el canal del anterior: si hay
    /// otro sirviendo es que el primero terminó.
    #[test]
    fn a_new_serving_channel_closes_the_one_it_replaces() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold(a_channel(PORTS[0], &closed));
        held.hold(a_channel(PORTS[1], &closed));

        assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
    }
}
