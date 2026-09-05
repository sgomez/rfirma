//! **El trámite de sede**: de la operación que llega por el canal a lo que la
//! sede recibe (ID-272, ID-275, ID-276, ID-280).
//!
//! [`super::site`] atiende la **invocación** —abre el canal en uno de los
//! puertos que la sede sorteó— y este módulo atiende lo que viene después: la
//! operación que llega por ese canal ya abierto, el momento del consentimiento
//! y la respuesta. Hoy la única operación que se atiende es `selectcert`
//! ([`crate::protocol::operation`]).
//!
//! # Los dos canales van desacompasados (ID-275)
//!
//! Lo que la sede recibe sale **en el acto**: no espera a que nadie cierre una
//! ventana. Por eso todo lo que este módulo devuelve lleva su línea de cable
//! dentro ([`SiteReply::on_the_wire`]) y, cuando algo sale mal, lleva **además**
//! la situación entera para la ventana: el código `SAF_` no puede cargar con la
//! precisión, y la ventana no puede cargar con el acuse.
//!
//! # El consentimiento no se salta nunca (ID-272)
//!
//! `headless` y `mandatoryCertSelection` **se ignoran los dos**, y por eso no
//! se leen en ningún sitio: no hay parámetro de la sede que quite el momento en
//! el que la persona ve qué se le pide y puede decir que no. También con **un
//! solo** certificado, que encadenado con un almacén que no pide PIN sería una
//! entrega de identidad sin que la persona viera absolutamente nada. En
//! `selectcert` ese momento consiente **entregar identidad, no firmar**
//! (ID-276).
//!
//! # Un trámite cada vez (ID-280, ID-281)
//!
//! [`LiveErrand`] es el trámite vivo del proceso, y como mucho hay uno.
//! Mientras dure, una segunda invocación `afirma://` se rechaza por su propio
//! socket ([`super::site::attend_launch`]): atender dos a la vez es meter a la
//! persona en dos trámites de dos sedes con dos PIN a medias. Y es un cerrojo
//! **de proceso** porque la instancia es única (ID-160, ID-279, ID-281).

use std::path::Path;
use std::sync::Mutex;

use base64::Engine as _;

use crate::commands::views::CertificateView;
use crate::commands::Failure;
use crate::memory::{ListedCertificates, Memory};
use crate::pkcs11::{self, Store};
use crate::protocol::{
    read_operation, ChannelCredential, Refusal, SafCode, SelectCertificate, SiteFilter,
    SiteOperation, WireAnswer,
};

use super::filtering::{self, FilterEngine};
use super::frontier;

/// **El trámite vivo del proceso**, si lo hay (ID-280).
#[derive(Default)]
pub struct LiveErrand(Mutex<Option<Errand>>);

/// Lo que se sabe de un trámite en curso.
///
/// La credencial y el puerto, y nada más: el documento que la sede manda no se
/// recuerda (ID-286), y la operación la lleva quien la está atendiendo.
#[derive(Clone, Debug)]
pub struct Errand {
    credential: ChannelCredential,
    port: u16,
}

impl Errand {
    /// El trámite que abre esa invocación en ese puerto.
    pub fn of(credential: ChannelCredential, port: u16) -> Self {
        Self { credential, port }
    }

    /// La credencial con la que se cerró el canal.
    pub fn credential(&self) -> &ChannelCredential {
        &self.credential
    }

    /// El puerto en el que quedó escuchando.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl LiveErrand {
    /// Si hay un trámite de sede a medias ahora mismo.
    pub fn is_live(&self) -> bool {
        super::lock(&self.0).is_some()
    }

    /// Apunta el trámite que empieza. **No sustituye**: con uno vivo devuelve
    /// `false` y el que llega se queda fuera (ID-280).
    pub fn begin(&self, errand: Errand) -> bool {
        let mut live = super::lock(&self.0);
        if live.is_some() {
            return false;
        }
        *live = Some(errand);
        true
    }

    /// El trámite vivo, si lo hay.
    pub fn current(&self) -> Option<Errand> {
        super::lock(&self.0).clone()
    }

    /// Se acabó: la sede ya tiene su respuesta.
    ///
    /// Se llama **al contestar** y no al cerrar la ventana, que es lo mismo que
    /// dice el ID-275 desde el otro lado: el desenlace que la ventana enseña ya
    /// no es parte del trámite.
    pub fn end(&self) {
        *super::lock(&self.0) = None;
    }
}

/// En qué queda la operación que llegó por el canal.
#[derive(Debug)]
pub enum ErrandStep {
    /// **El momento del consentimiento** (ID-272, ID-276): la ventana enseña
    /// estas filas y la persona decide. La sede no recibe nada todavía.
    AskingForConsent(Vec<CertificateView>),
    /// No hay nada que consentir: esto es lo que la sede recibe, y sale ya
    /// (ID-275).
    Answering(SiteReply),
}

/// Lo que se le contesta a la sede, y lo que queda para la ventana.
///
/// Los dos juntos en un tipo porque son **la misma decisión contada dos
/// veces**: el cable se lleva el código del catálogo cerrado y la ventana, la
/// situación entera con su detalle, que es lo que el ID-291 no deja salir.
#[derive(Debug)]
pub enum SiteReply {
    /// El certificado que la persona entregó, en Base64 URL-safe y **nada
    /// más**, tal y como lo espera el cliente publicado
    /// (`ProtocolInvocationLauncherSelectCert.java:262`).
    Certificate(String),
    /// La persona ha dicho que no (ID-293).
    Cancelled,
    /// La sede recibe el código; la ventana, la situación entera.
    Refused {
        /// Lo que sale al cable.
        answer: WireAnswer,
        /// Lo que se queda dentro y enseña la ventana (ID-29, ID-291).
        failure: Failure,
    },
    /// El rechazo es **del protocolo**, y esos nacen ya con su código: no hay
    /// situación nuestra detrás que traducir ni que enseñar traducida
    /// ([`frontier`], ID-288). El detalle crudo viaja dentro y **no sale al
    /// cable** (ID-291).
    RefusedByTheProtocol(Refusal),
}

impl SiteReply {
    /// La línea exacta que se escribe en el canal.
    pub fn on_the_wire(&self) -> String {
        match self {
            Self::Certificate(encoded) => encoded.clone(),
            Self::Cancelled => frontier::cancelled().on_the_wire(),
            Self::Refused { answer, .. } => answer.on_the_wire(),
            Self::RefusedByTheProtocol(refusal) => refusal.answer().on_the_wire(),
        }
    }

    /// Lo que la ventana tiene que enseñar, cuando hay algo que enseñar.
    pub fn failure(&self) -> Option<&Failure> {
        match self {
            Self::Refused { failure, .. } => Some(failure),
            _ => None,
        }
    }
}

/// **Caso de uso.** Atiende la operación que llegó por el canal ya abierto.
///
/// Devuelve o el momento del consentimiento —con el listado que la sede
/// acepta— o lo que la sede recibe sin que haya nada que consentir: un rechazo
/// del protocolo, o el `SAF_19` de un listado que se quedó vacío
/// (`ProtocolInvocationLauncherSelectCert.java:208`-`215`).
///
/// **Éste es el que lista el token**, y por eso su hermano
/// [`consent_for`] existe: la decisión —qué se enseña, qué se contesta y
/// cuándo— se prueba entera en grada A con un listado de andamio, igual que
/// [`filtering::listing_the_site_accepts`] y
/// [`filtering::keep_what_the_site_accepts`] (TD-20, TD-51).
///
/// Los criterios de rFirma se aplican al listar y la expresión de la sede
/// después, que es el orden del ID-258. Y la situación del token se traduce
/// **en la frontera** (ID-288): por eso se llama a
/// [`pkcs11::list_certificates_across`] y no al caso de uso de
/// [`filtering`], que la entrega ya envuelta para la ventana.
pub fn attend_operation<E: FilterEngine>(
    engine: &E,
    stores: &[Store],
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
    url: &crate::protocol::AfirmaUrl,
    live: &LiveErrand,
) -> ErrandStep {
    let request = match read_operation(url) {
        Ok(SiteOperation::SelectCertificate(request)) => request,
        Err(refusal) => return answering(live, SiteReply::RefusedByTheProtocol(refusal)),
    };

    let ours = match pkcs11::list_certificates_across(stores) {
        Ok(ours) => ours,
        Err(error) => {
            let code = frontier::code_of_token(error.situation());
            return answering(
                live,
                SiteReply::Refused {
                    answer: WireAnswer::refused(code),
                    failure: error.into(),
                },
            );
        }
    };

    consent_for(engine, &request, ours, installed_dir, listed, memory, live)
}

/// **Caso de uso.** El momento del consentimiento sobre un listado que **ya**
/// pasó por los criterios de rFirma (ID-258, ID-272).
///
/// O las filas que la ventana enseña, o lo que la sede recibe cuando no queda
/// nada que consentir.
pub fn consent_for<E: FilterEngine>(
    engine: &E,
    request: &SelectCertificate,
    ours: Vec<crate::pkcs11::TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
    live: &LiveErrand,
) -> ErrandStep {
    let accepted = match filtering::keep_what_the_site_accepts(engine, request.filter(), ours) {
        Ok(accepted) => accepted,
        // Lo único que puede fallar después de la criba de rFirma es el motor
        // prestado del puente, y lo que la sede ve de eso es que no se le ha
        // podido servir el almacén: `SAF_08` es lo que el original emite ahí
        // (`ProtocolInvocationLauncherSelectCert.java:217`-`224`).
        Err(failure) => {
            return answering(
                live,
                SiteReply::Refused {
                    answer: WireAnswer::refused(SafCode::CannotAccessKeystore),
                    failure,
                },
            )
        }
    };

    if accepted.is_empty() {
        // La sede se entera en el acto (ID-275); la ventana enseña **cuál de
        // las dos** situaciones del ID-278 es, y para eso le llega el detalle.
        return answering(
            live,
            SiteReply::Refused {
                answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
                failure: Failure::new(
                    "certificateNotFound",
                    "no queda ningun certificado que la sede acepte",
                ),
            },
        );
    }

    // Y aquí **no** se mira cuántos hay: con uno solo se consiente igual
    // (ID-272).
    ErrandStep::AskingForConsent(super::certificates::rows_of(
        accepted,
        installed_dir,
        listed,
        memory,
    ))
}

/// **Caso de uso.** La persona se identifica: la sede recibe el certificado en
/// el acto (ID-275, ID-276).
///
/// Y como [`attend_operation`], **éste lista el token**: la decisión entera es
/// de [`identity_handed_over`].
pub fn identify_with<E: FilterEngine>(
    engine: &E,
    stores: &[Store],
    filter: &SiteFilter,
    handle: &str,
    listed: &ListedCertificates,
    live: &LiveErrand,
) -> SiteReply {
    let found = match pkcs11::list_certificates_across(stores) {
        Ok(found) => found,
        Err(error) => {
            let code = frontier::code_of_token(error.situation());
            return over(
                live,
                SiteReply::Refused {
                    answer: WireAnswer::refused(code),
                    failure: error.into(),
                },
            );
        }
    };

    identity_handed_over(engine, filter, &found, handle, listed, live)
}

/// **Caso de uso.** Lo que la sede recibe cuando la persona se identifica con
/// uno de los certificados que tenía delante.
///
/// El filtro se vuelve a comprobar antes de entregar nada (ID-259): que el
/// certificado estuviera en la lista que la ventana enseñó no basta, porque la
/// ventana no es quien hace cumplir lo que pidió la sede.
pub fn identity_handed_over<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    found: &[crate::pkcs11::TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
    live: &LiveErrand,
) -> SiteReply {
    let chosen =
        match filtering::usable_certificate_for_the_site(engine, filter, found, handle, listed) {
            Ok(chosen) => chosen,
            // El certificado que la ventana señaló ya no está, ya no sirve o la
            // sede ya no lo acepta: para ella, ninguno que valga.
            Err(failure) => {
                return over(
                    live,
                    SiteReply::Refused {
                        answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
                        failure,
                    },
                )
            }
        };

    over(live, SiteReply::Certificate(on_the_wire(chosen.der())))
}

/// **Caso de uso.** La persona ha dicho que no: `CANCEL` sale en el acto
/// (ID-275, ID-293).
pub fn declined(live: &LiveErrand) -> SiteReply {
    over(live, SiteReply::Cancelled)
}

/// El certificado tal y como viaja: Base64 **URL-safe con relleno**, que es lo
/// que produce `Base64.encode(certEncoded, true)` del original —su alfabeto
/// cambia `+` y `/`, pero el `=` del final se queda— y lo único que el cliente
/// deshace (`autoscript.js:2462`-`2471`).
fn on_the_wire(der: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(der)
}

/// Contesta y cierra el trámite: la sede ya tiene lo suyo.
fn answering(live: &LiveErrand, reply: SiteReply) -> ErrandStep {
    ErrandStep::Answering(over(live, reply))
}

/// Lo mismo, cuando lo que se devuelve es la respuesta y nada más.
fn over(live: &LiveErrand, reply: SiteReply) -> SiteReply {
    live.end();
    reply
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::app::fixtures::{a_memory, a_usable_certificate, listed_from};
    use crate::app::site::{attend_launch, Attendance};
    use crate::channel::{ChannelDuty, ChannelError, OpenChannel, Shutdown};
    use crate::ffi::BridgeError;
    use crate::pkcs11::TokenCertificate;
    use crate::protocol::{AfirmaUrl, ChannelMessage};

    /// **Grada A**: ni token, ni librería nativa, ni socket (TD-51, TD-52). El
    /// transporte es un cierre y el motor de filtros, un doble.
    struct AnEngine {
        answers: RefCell<Vec<Vec<usize>>>,
    }

    impl AnEngine {
        /// Un motor que contesta eso, en ese orden, a cada llamada.
        fn answering(answers: &[&[usize]]) -> Self {
            Self {
                answers: RefCell::new(answers.iter().map(|one| one.to_vec()).collect()),
            }
        }
    }

    impl FilterEngine for AnEngine {
        fn select(
            &self,
            _properties: &str,
            _certificates: &str,
        ) -> Result<Vec<usize>, BridgeError> {
            let mut answers = self.answers.borrow_mut();
            if answers.is_empty() {
                return Ok(Vec::new());
            }
            Ok(answers.remove(0))
        }
    }

    const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

    /// Un transporte que abre siempre, y apunta lo que se le pidió.
    fn a_transport(
        asked: &RefCell<Vec<ChannelDuty>>,
    ) -> impl Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError> + '_ {
        move |ports: &[u16], duty: ChannelDuty| {
            asked.borrow_mut().push(duty);
            Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
        }
    }

    fn a_launch(ports: &str) -> String {
        format!("afirma://websocket?ports={ports}&v=4&idsession={CREDENTIAL}")
    }

    /// La operación tal y como llega por el canal: se lee con el códec del
    /// protocolo, que es por donde entra de verdad.
    fn an_operation(parameters: &str) -> AfirmaUrl {
        let text = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}{parameters}");
        let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
            panic!("una URL del protocolo es una operacion");
        };
        url
    }

    fn requested(url: &AfirmaUrl) -> SelectCertificate {
        let SiteOperation::SelectCertificate(request) =
            read_operation(url).expect("es una operacion que se atiende");
        request
    }

    /// **El trazador entero** (TD-51): invocación, canal, operación leída del
    /// mensaje, listado filtrado, consentimiento y respuesta, sin abrir un
    /// socket (TD-52). Lo único doblado, además del transporte, es el listado
    /// del token, que es lo que [`attend_operation`] añade encima.
    #[test]
    fn a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let live = LiveErrand::default();
        let asked = RefCell::new(Vec::new());
        let engine = AnEngine::answering(&[&[0], &[0]]);

        // 1. La sede invoca, y el canal queda sirviendo su conversación.
        let attendance = attend_launch(&a_launch("54001,54002,54003"), &a_transport(&asked), &live);
        assert!(
            matches!(attendance, Attendance::Serving(_)),
            "la invocacion es buena: {attendance:?}"
        );
        assert!(live.is_live(), "el tramite queda vivo mientras se atiende");

        // 2. Por ese canal llega la operación, y lo que sale es el momento del
        //    consentimiento con el listado que la sede acepta.
        let url = an_operation("");
        let request = requested(&url);
        let step = consent_for(
            &engine,
            &request,
            ours.clone(),
            home.path(),
            &listed,
            &memory,
            &live,
        );
        let ErrandStep::AskingForConsent(rows) = step else {
            panic!("hay un certificado que la sede acepta: {step:?}");
        };
        assert_eq!(rows.len(), 1);
        assert!(live.is_live(), "consintiendo, el tramite sigue vivo");

        // 3. La persona se identifica, y la sede recibe el certificado.
        let reply = identity_handed_over(
            &engine,
            request.filter(),
            &ours,
            &rows[0].id,
            &listed,
            &live,
        );
        let SiteReply::Certificate(encoded) = &reply else {
            panic!("la persona se ha identificado: {reply:?}");
        };
        assert_eq!(
            encoded,
            &base64::engine::general_purpose::URL_SAFE.encode(ours[0].der()),
            "el certificado viaja en Base64 URL-safe y nada mas"
        );
        assert_eq!(reply.on_the_wire(), *encoded);
        assert!(
            !live.is_live(),
            "contestada la sede, el tramite deja de estar vivo sin que nadie cierre nada (ID-275)"
        );
    }

    /// **ID-272**: el consentimiento aparece **también** con un solo
    /// certificado, y ni `headless` ni `mandatoryCertSelection` lo quitan.
    #[test]
    fn neither_headless_nor_the_mandatory_selection_skips_the_consent() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("EL UNICO")];
        let (listed, _) = listed_from(&ours);
        let live = LiveErrand::default();
        let url = an_operation("&headless=true&mandatoryCertSelection=true");

        let step = consent_for(
            &AnEngine::answering(&[&[0]]),
            &requested(&url),
            ours,
            home.path(),
            &listed,
            &memory,
            &live,
        );

        let ErrandStep::AskingForConsent(rows) = step else {
            panic!("el consentimiento no se salta nunca: {step:?}");
        };
        assert_eq!(rows.len(), 1, "uno solo se consiente igual");
    }

    /// Los dos parámetros **ni se leen**: la prueba de arriba mira la conducta,
    /// y ésta mira que no exista el camino que la cambiaría.
    #[test]
    fn the_two_parameters_of_the_silent_signature_are_not_read_anywhere() {
        let production = include_str!("errand.rs")
            .split_once("\nmod tests {")
            .expect("este modulo tiene pruebas")
            .0;

        for parameter in ["\"headless\"", "\"mandatoryCertSelection\""] {
            assert!(
                !production.contains(parameter),
                "{parameter} se lee en algun sitio: el consentimiento se podria saltar (ID-272)"
            );
        }
    }

    /// **ID-258 / ID-278**: si la sede los excluye a todos, lo que recibe es
    /// `SAF_19`, y sale ya.
    #[test]
    fn a_site_that_excludes_them_all_gets_the_code_of_an_empty_keystore() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let live = LiveErrand::default();
        let url = an_operation("");

        let step = consent_for(
            &AnEngine::answering(&[&[]]),
            &requested(&url),
            ours,
            home.path(),
            &listed,
            &memory,
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("no hay nada que consentir: {step:?}");
        };
        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
        );
        assert!(
            reply.failure().is_some(),
            "la ventana enseña la situacion entera (ID-275)"
        );
    }

    /// Un rechazo del protocolo —un criterio fuera de la lista blanca— sale con
    /// su código **sin tocar el token**.
    #[test]
    fn a_refusal_of_the_protocol_never_reaches_the_token() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let live = LiveErrand::default();
        let properties =
            base64::engine::general_purpose::URL_SAFE.encode(b"filters=inventado:loquesea\n");

        let step = attend_operation(
            &AnEngine::answering(&[]),
            &[],
            home.path(),
            &ListedCertificates::new(),
            &memory,
            &an_operation(&format!("&properties={properties}")),
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("el criterio no esta en la lista blanca: {step:?}");
        };
        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(SafCode::Params).on_the_wire()
        );
    }

    /// Sin ningún almacén donde mirar, la sede recibe el código de la situación
    /// del token, traducido por la frontera y no por este módulo (ID-288).
    #[test]
    fn a_token_that_cannot_be_listed_answers_with_the_code_of_its_own_situation() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let live = LiveErrand::default();

        let step = attend_operation(
            &AnEngine::answering(&[]),
            &[],
            home.path(),
            &ListedCertificates::new(),
            &memory,
            &an_operation(""),
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("no hay almacenes: {step:?}");
        };
        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(frontier::code_of_token(
                crate::pkcs11::Situation::ModuleNotFound
            ))
            .on_the_wire()
        );
    }

    /// **ID-293**: la cancelación es la persona diciendo que no, y sale en el
    /// acto.
    #[test]
    fn the_person_saying_no_is_the_only_cancellation() {
        let live = LiveErrand::default();
        live.begin(Errand::of(a_credential(), 54001));

        let reply = declined(&live);

        assert_eq!(reply.on_the_wire(), "CANCEL");
        assert!(!live.is_live(), "cancelado, el tramite se acaba");
    }

    /// **ID-280**: con un trámite vivo, el segundo `afirma://` se rechaza por su
    /// propio socket mientras el primero siga vivo.
    #[test]
    fn a_second_launch_is_refused_while_the_first_errand_is_live() {
        let live = LiveErrand::default();
        let asked = RefCell::new(Vec::new());

        let first = attend_launch(&a_launch("54001"), &a_transport(&asked), &live);
        assert!(matches!(first, Attendance::Serving(_)), "{first:?}");

        let second = attend_launch(&a_launch("55001"), &a_transport(&asked), &live);
        let Attendance::RefusingOverTheChannel { answer, .. } = second else {
            panic!("el segundo se rechaza por su socket: {second:?}");
        };
        assert_eq!(
            answer.on_the_wire(),
            WireAnswer::refused(SafCode::CannotOpenSocket).on_the_wire()
        );
    }

    /// Y en cuanto el primero contesta, la sede siguiente sí es atendida: lo que
    /// cierra el trámite es la respuesta, no que se cierre una ventana (ID-275).
    #[test]
    fn once_the_first_site_has_its_answer_the_next_launch_is_attended() {
        let live = LiveErrand::default();
        let asked = RefCell::new(Vec::new());

        attend_launch(&a_launch("54001"), &a_transport(&asked), &live);
        declined(&live);

        let next = attend_launch(&a_launch("55001"), &a_transport(&asked), &live);

        assert!(matches!(next, Attendance::Serving(_)), "{next:?}");
    }

    /// El trámite vivo recuerda **la credencial y el puerto**, que es lo que
    /// hace falta para saber con quién se está hablando. El documento de la
    /// sede no se recuerda (ID-286).
    #[test]
    fn the_live_errand_remembers_the_credential_and_the_port_and_nothing_else() {
        let live = LiveErrand::default();
        let asked = RefCell::new(Vec::new());

        attend_launch(&a_launch("54001"), &a_transport(&asked), &live);

        let errand = live.current().expect("hay tramite vivo");
        assert_eq!(errand.credential().as_str(), CREDENTIAL);
        assert_eq!(errand.port(), 54001);
    }

    /// **ID-259**: un certificado que la sede ya no acepta no se entrega, y lo
    /// que ella recibe es que no hay ninguno que valga.
    #[test]
    fn a_certificate_the_site_no_longer_accepts_is_never_handed_over() {
        let ours: Vec<TokenCertificate> = vec![a_usable_certificate("FIRMA")];
        let (listed, handles) = listed_from(&ours);
        let live = LiveErrand::default();

        let reply = identity_handed_over(
            &AnEngine::answering(&[&[]]),
            &SiteFilter::default(),
            &ours,
            &handles[0],
            &listed,
            &live,
        );

        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
        );
        assert!(
            reply
                .failure()
                .is_some_and(|it| it.situation == "certificateNotFound"),
            "la ventana sabe cual es la situacion: {reply:?}"
        );
    }

    fn a_credential() -> ChannelCredential {
        ChannelCredential::parse(CREDENTIAL).expect("es una credencial buena")
    }
}
