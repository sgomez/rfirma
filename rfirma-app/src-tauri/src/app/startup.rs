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

use super::errand::{Errand, LiveErrand};
use super::invocation::Invocation;
use super::site::{self, Attendance, ChannelTransport};
use super::trust;

/// **El abridor de la ventana de sede** (ID-333, ID-334): crea la ventana y le
/// publica el trámite.
///
/// Recibe el trámite ya apuntado —el que se quedó con la plaza en
/// [`LiveErrand::begin`]— y no devuelve nada: si la ventana no se puede crear
/// no hay decisión que tomar aquí, y quien la crea es quien lo cuenta.
pub type SiteWindowOpener<'a> = &'a dyn Fn(&Errand);

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
    let said = refresh_the_local_ca(trust);

    let Some(url) = invocation.site_launch() else {
        return Startup {
            said,
            opening: Opening::TheMainWindow,
        };
    };

    Startup {
        said,
        opening: Opening::TheSiteErrand(attend_site_launch(url, transport, window, live)),
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
/// le publica a la ventana es el trámite que quedó apuntado, no el que se
/// intentó.
pub fn attend_site_launch(
    url: &str,
    transport: ChannelTransport<'_>,
    window: SiteWindowOpener<'_>,
    live: &LiveErrand,
) -> Attendance {
    let attendance = site::attend_launch(url, transport, live);

    if let Attendance::Serving(_) = attendance {
        if let Some(errand) = live.current() {
            window(&errand);
        }
    }

    attendance
}

/// Deja la CA local de confianza donde se pueda, y devuelve lo que hay que
/// decir (ID-329).
///
/// El material que no se puede leer ni escribir tampoco interrumpe el arranque:
/// se dice y se sigue, porque la ventana principal no depende de la CA local
/// para abrirse.
fn refresh_the_local_ca(trust: TrustAtStartup<'_>) -> Vec<String> {
    match trust::refresh_local_ca_trust(trust.store, trust.profiles, trust.stores, Moment::Startup)
    {
        Ok(outcome) => trust::narrate_startup_outcome(outcome, trust.profiles),
        Err(error) => vec![format!(
            "rfirma: no se puede refrescar la CA local ({error}); el arranque sigue sin ella"
        )],
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
#[derive(Default)]
pub struct HeldChannel(std::sync::Mutex<Option<OpenChannel>>);

impl HeldChannel {
    /// Se queda con el canal. El que hubiera **se cierra**: sólo hay un trámite
    /// a la vez (ID-280), y el anterior ya no tiene quien lo conteste.
    pub fn hold(&self, channel: OpenChannel) {
        if let Some(previous) = super::lock(&self.0).replace(channel) {
            previous.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;
    use std::sync::Mutex;

    use crate::channel::{ChannelDuty, ChannelError, Shutdown};
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
            let port = *ports.first().expect("la sede sorteó puertos");
            Ok(OpenChannel::new(port, Shutdown::of(|| {})))
        }

        /// El abridor de ventana: apunta el puerto del trámite que se le
        /// publica.
        fn window(&self, errand: &Errand) {
            self.note(&format!("ventana:{}", errand.port()));
        }
    }

    impl TrustStores for World {
        fn install(
            &self,
            _profile: &Path,
            _certificate_der: &[u8],
            _nickname: &str,
        ) -> Result<(), TrustError> {
            self.note("confianza");
            Ok(())
        }

        fn trust_of(
            &self,
            _profile: &Path,
            _certificate_der: &[u8],
        ) -> Result<Option<u32>, TrustError> {
            Ok(None)
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
            &|errand| world.window(errand),
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
                Opening::TheSiteErrand(Attendance::Serving(_))
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
                Opening::TheSiteErrand(Attendance::Serving(_))
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
            &|errand| world.window(errand),
            &live,
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
            &|errand| world.window(errand),
            &live,
        );

        assert!(matches!(attendance, Attendance::Serving(_)));
        assert_eq!(
            world.steps(),
            ["canal".to_owned(), format!("ventana:{}", PORTS[0])],
            "ni un almacén se abre en la segunda invocación"
        );
    }
}
