//! **El estado del trámite, con un solo dueño** (ID-280, ID-321, RD-01).
//!
//! Todo lo que el trámite recuerda entre el momento en que llega la operación
//! y el momento en que la sede recibe su respuesta vive aquí, y sólo los verbos
//! de [`super`] lo leen y lo escriben. Antes del #406 estaba partido en dos
//! capas —el trámite vivo aquí y el consentimiento pendiente en el adaptador
//! de Tauri— y un error de consentimiento podía estar en dos ficheros.
//!
//! Lo que se guarda, y por qué:
//!
//! - **El trámite vivo del proceso**, del que hay uno como mucho (ID-280), con
//!   la credencial y el puerto de su canal.
//! - **El códec negociado al arrancar** ([`super::ports::ProtocolCodec`]): es
//!   con el que se leen las operaciones y se escriben las respuestas de ese
//!   canal, y por eso sobrevive al trámite —el canal sigue en pie cuando el
//!   trámite acaba—.
//! - **Por dónde se le contesta a la sede** (ID-321): el asa que trajo la
//!   operación, que se gasta al contestar.
//! - **La petición que la sede mandó**, para poder volver a atenderla sin que
//!   ella la mande otra vez (ID-341).
//! - **El fichero de paso** del documento que mandó la sede, que se borra al
//!   terminar: de ese documento no queda rastro ninguno (ID-286).
//! - **Lo que la persona tiene delante para consentir**, que se vuelve a
//!   comprobar antes de entregar nada (ID-259, ID-266) y que la ventana no
//!   puede devolver porque nunca cruzó (ADR-0011).
//! - **El último momento del trámite**, para la ventana de sede que todavía no
//!   había puesto la escucha cuando se publicó (ID-338).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::protocol::{AfirmaUrl, ChannelCredential, SiteFilter};

use super::outcome::{Moment, SiteOutcome};
use super::ports::{ProtocolCodec, ReplyHandle};

/// El códec negociado, compartido entre el trámite y quien lo apuntó.
pub type NegotiatedCodec = Arc<dyn ProtocolCodec + Send + Sync>;

/// **El trámite vivo del proceso**, si lo hay (ID-280), y todo lo que
/// recuerda mientras dura.
///
/// Es un cerrojo **de proceso** porque la instancia es única (ID-160, ID-279,
/// ID-281): mientras haya uno vivo, una segunda invocación `afirma://` se
/// rechaza por su propio socket ([`crate::app::site::attend_launch`]).
#[derive(Default)]
pub struct LiveErrand {
    errand: Mutex<Option<Errand>>,
    codec: Mutex<Option<NegotiatedCodec>>,
    scratch: Mutex<Option<PathBuf>>,
    reply: Mutex<Option<ReplyHandle>>,
    asked: Mutex<Option<AfirmaUrl>>,
    consent: Mutex<Option<PendingConsent>>,
    moment: Mutex<Option<Moment>>,
}

/// Lo que se sabe de un trámite en curso.
///
/// La credencial, el puerto y el códec con el que se habla por ese canal, y
/// nada más: el documento que la sede manda no se recuerda (ID-286), y la
/// operación la lleva quien la está atendiendo.
#[derive(Clone)]
pub struct Errand {
    credential: ChannelCredential,
    port: u16,
    codec: NegotiatedCodec,
}

impl std::fmt::Debug for Errand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Errand")
            .field("credential", &self.credential)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

impl Errand {
    /// El trámite que abre esa invocación en ese puerto, hablando con ese
    /// códec: lo decide la negociación de arranque y nadie más (RD-05).
    pub fn of(credential: ChannelCredential, port: u16, codec: NegotiatedCodec) -> Self {
        Self {
            credential,
            port,
            codec,
        }
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

/// Lo que queda pendiente de contestar, según lo que la sede pidiera.
enum PendingConsent {
    /// `selectcert`: para entregar identidad basta con volver a comprobar el
    /// filtro (ID-276).
    Identity(SiteFilter),
    /// `sign` o `cosign`: además del filtro hacen falta el documento y la
    /// política que la sede declaró.
    Signature(PendingSignature),
}

/// **Lo que hace falta para firmar cuando la persona ya ha consentido**, y que
/// la ventana no puede devolver.
///
/// Es la mitad del consentimiento que **no** es para mirar: las filas, la ronda
/// y el aviso de las firmas ilegibles se los lleva la ventana en su momento;
/// esto se queda aquí porque es lo que hace cumplir lo que pidió la sede, y eso
/// no se le pregunta a la ventana (ID-259, ID-266).
#[derive(Clone, Debug)]
pub(super) struct PendingSignature {
    /// El asa del documento que mandó la sede, la misma que cruzó a la ventana.
    pub(super) document: String,
    /// Lo que la sede pide del listado, que se vuelve a comprobar (ID-259).
    pub(super) filter: SiteFilter,
    /// Los `extraParams` que declaró, ya expandidos (ID-266).
    pub(super) from_the_site: BTreeMap<String, String>,
    /// Que el documento trae firmas que rFirma no sabe leer (ID-297).
    ///
    /// Se apunta porque **consentir el trámite es consentirlas**: la pregunta
    /// viaja dentro del momento del consentimiento y decir que no a ella es
    /// cancelar el trámite entero (ID-299, ID-301). Quien firma después de eso
    /// ya ha dicho que sí, y sin esta clave el puente abortaría la cofirma.
    pub(super) unregistered_signatures: bool,
}

impl LiveErrand {
    /// Un trámite que todavía no ha empezado pero ya sabe con qué códec se
    /// habla, para las pruebas que atacan los verbos sin pasar por la
    /// negociación de arranque.
    #[cfg(test)]
    pub fn speaking(codec: NegotiatedCodec) -> Self {
        let live = Self::default();
        *crate::app::lock(&live.codec) = Some(codec);
        live
    }

    /// Apunta el fichero de paso del documento que mandó la sede, para
    /// borrarlo al contestar.
    pub(super) fn keep_the_scratch(&self, path: PathBuf) {
        *crate::app::lock(&self.scratch) = Some(path);
    }

    /// Apunta la petición que la sede mandó por el canal, para poder volver a
    /// atenderla sin que ella la mande otra vez (ID-341).
    pub(super) fn keep_the_request(&self, url: AfirmaUrl) {
        *crate::app::lock(&self.asked) = Some(url);
    }

    /// Apunta el trámite que empieza. **No sustituye**: con uno vivo devuelve
    /// `false` y el que llega se queda fuera (ID-280).
    ///
    /// Es **la única puerta** del trámite único, y por eso mira y apunta bajo
    /// el mismo candado: quien la llame decide con lo que devuelve y no con un
    /// [`Self::current`] anterior, que sería mirar por una toma y apuntar por
    /// otra. Su valor de retorno no es opcional, y por eso no hay ningún
    /// «¿hay trámite vivo?» que preguntar antes: la plaza se pide aquí.
    ///
    /// Con el trámite entra **el códec negociado**, que se queda aunque el
    /// trámite termine: el canal sigue en pie y lo que llegue por él se lee y
    /// se contesta con el mismo códec.
    #[must_use = "con uno vivo devuelve false y el que llega no queda apuntado (ID-280)"]
    pub fn begin(&self, errand: Errand) -> bool {
        let mut live = crate::app::lock(&self.errand);
        if live.is_some() {
            return false;
        }
        *crate::app::lock(&self.codec) = Some(Arc::clone(&errand.codec));
        *live = Some(errand);
        true
    }

    /// **Apunta por dónde se le contesta a la sede** (ID-321).
    ///
    /// El asa la trae la operación que llegó por el canal, y se gasta al
    /// contestar: quien la reciba después de eso no escribe nada, que es lo que
    /// hace que cancelar dos veces —o cerrar la ventana con la sede ya
    /// servida— no mande nada por el cable (ID-340).
    ///
    /// **Sólo hay un asa apuntada, y la última gana.** Del trámite vivo hay uno
    /// solo, así que una segunda operación sobre él suelta el asa anterior: esa
    /// conexión se cierra sin línea, que es exactamente lo que dice el ID-323
    /// para un canal que se queda sin quien le conteste. Es la conducta que se
    /// quiere, no un descuido.
    pub fn answer_through(&self, reply: ReplyHandle) {
        *crate::app::lock(&self.reply) = Some(reply);
    }

    /// Contesta esto a la sede, si queda asa por la que contestar, **escrito
    /// con el códec negociado** (RD-03): el trámite no sabe cómo se escribe
    /// una respuesta en el cable.
    ///
    /// Sin asa no hay nada que hacer, y es la respuesta correcta en los dos
    /// casos en que pasa: un trámite que ya contestó y una operación que se
    /// despachó sin canal detrás. Y sin códec tampoco: no ha habido
    /// negociación, así que no hay sede al otro lado.
    pub(super) fn answer_the_site(&self, outcome: &SiteOutcome) {
        let Some(reply) = crate::app::lock(&self.reply).take() else {
            return;
        };
        if let Some(codec) = self.codec() {
            reply.answer(codec.encode(outcome));
        }
    }

    /// El códec con el que se habla por el canal de este trámite, si se ha
    /// negociado alguno.
    pub fn codec(&self) -> Option<NegotiatedCodec> {
        crate::app::lock(&self.codec).clone()
    }

    /// **Lo que la sede pidió por el canal**, mientras el trámite siga vivo.
    ///
    /// Se apunta para poder **volver a atenderlo sin reiniciar nada** (ID-341):
    /// quien no tenía ningún certificado instala uno con la ventana abierta y
    /// vuelve a mirar, y lo que se atiende es la misma petición, por el mismo
    /// canal y con la misma asa. Sin esto la única salida sería que la sede
    /// invocara otra vez.
    pub fn the_request(&self) -> Option<AfirmaUrl> {
        crate::app::lock(&self.asked).clone()
    }

    /// El trámite vivo, si lo hay.
    pub fn current(&self) -> Option<Errand> {
        crate::app::lock(&self.errand).clone()
    }

    /// Se acabó: la sede ya tiene su respuesta.
    ///
    /// Se llama **al contestar** y no al cerrar la ventana, que es lo mismo que
    /// dice el ID-275 desde el otro lado: el desenlace que la ventana enseña ya
    /// no es parte del trámite.
    ///
    /// **Contestar no es la única salida.** Cerrar la ventana de sede sin
    /// haber contestado es cancelar (ID-340), y sale por aquí igual porque lo
    /// que hace es llamar a [`super::decline`]: la sede recibe su `CANCEL` en
    /// el acto y el trámite deja de estar vivo. Lo que sigue sin pasar por aquí
    /// es una sede que se cae con el canal abierto: eso no tumba el trámite
    /// (ID-323), el desenlace se enseña en la ventana igual y el trámite acaba
    /// cuando la persona conteste, aunque ya no haya nadie escuchando.
    ///
    /// **El consentimiento se olvida aquí y en ningún otro sitio**: contestada
    /// la sede, ni la ventana ni el canal tienen ya nada que contestar con él,
    /// y así un fallo después de consentir lo deja olvidado siempre por el
    /// mismo camino.
    pub fn end(&self) {
        *crate::app::lock(&self.errand) = None;
        // Y el asa se cierra con él: soltarla sin escribir cierra la conexión
        // sin línea ninguna, que es lo que corresponde cuando el trámite acaba
        // sin respuesta que dar.
        drop(crate::app::lock(&self.reply).take());
        // Y el documento que mandó la sede se va con él: de él no queda rastro
        // ninguno (ID-286). Si el borrado falla —el fichero ya no está, o el
        // directorio se ha ido— no hay nada que contarle a nadie: el trámite ha
        // terminado y esto es limpieza.
        if let Some(scratch) = crate::app::lock(&self.scratch).take() {
            let _ = std::fs::remove_file(scratch);
        }
        // Y la petición: sin trámite vivo no hay nada que volver a atender.
        *crate::app::lock(&self.asked) = None;
        self.forget_the_consent();
    }

    /// El fichero de paso apuntado, si lo hay. **Sólo para las pruebas**: nadie
    /// del recorrido necesita la ruta, que es justamente lo que no cruza.
    #[cfg(test)]
    pub fn scratch_path(&self) -> Option<PathBuf> {
        crate::app::lock(&self.scratch).clone()
    }

    /// Apunta lo que la sede pide del listado para identificarse.
    pub(super) fn remember_identity(&self, filter: SiteFilter) {
        *crate::app::lock(&self.consent) = Some(PendingConsent::Identity(filter));
    }

    /// Apunta lo que hace falta para firmar lo que la sede mandó.
    pub(super) fn remember_signature(&self, pending: PendingSignature) {
        *crate::app::lock(&self.consent) = Some(PendingConsent::Signature(pending));
    }

    /// Lo que la sede pidió, si hay una identificación pendiente.
    pub(super) fn what_the_site_asked(&self) -> Option<SiteFilter> {
        match &*crate::app::lock(&self.consent) {
            Some(PendingConsent::Identity(filter)) => Some(filter.clone()),
            _ => None,
        }
    }

    /// Lo que hace falta para firmar, si hay una firma pendiente.
    pub(super) fn the_signature_consented(&self) -> Option<PendingSignature> {
        match &*crate::app::lock(&self.consent) {
            Some(PendingConsent::Signature(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Se acabó el consentimiento: ni la ventana ni el canal tienen ya nada
    /// que contestar con esto.
    pub(super) fn forget_the_consent(&self) {
        *crate::app::lock(&self.consent) = None;
    }

    /// **Apunta el momento en el que está el trámite**, pisando el anterior:
    /// lo que interesa es el último (ID-338).
    ///
    /// La ventana de sede se abre y **acto seguido** se le publica en qué quedó
    /// el arranque, pero entre que la página termina de cargar y que el frontal
    /// tiene puesta la escucha hay dos idas y vueltas por el IPC. El evento
    /// emitido en medio no lo oye nadie; por eso el momento se **guarda** y la
    /// ventana lo **pide** al montarse. Los momentos siguientes siguen llegando
    /// por el evento; lo que deja de depender del orden es el primero.
    pub fn note(&self, moment: Moment) {
        *crate::app::lock(&self.moment) = Some(moment);
    }

    /// El último momento apuntado, si hay alguno.
    ///
    /// **No se consume.** Al revés que la invocación con documento, aquí
    /// releer es lo correcto: la ventana puede recargarse, y lo que tiene que
    /// enseñar entonces es el momento en el que está el trámite, no ninguno.
    pub fn moment(&self) -> Option<Moment> {
        crate::app::lock(&self.moment).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::views::CertificateView;
    use crate::commands::views::StatusView;

    /// Un momento con una fila, para distinguirlo de otro.
    fn asking_with(label: &str) -> Moment {
        Moment::AskingForConsent {
            certificates: vec![CertificateView {
                id: "cert-1".to_owned(),
                label: label.to_owned(),
                holder_name: String::new(),
                id_number: String::new(),
                issuer: String::new(),
                store: "card".to_owned(),
                status: StatusView::Valid { not_after: 0 },
                remembered: false,
            }],
        }
    }

    /// Lo mínimo que hace falta para tener una firma consentida.
    fn a_pending_signature() -> PendingSignature {
        PendingSignature {
            document: "doc-1".to_owned(),
            filter: SiteFilter::default(),
            from_the_site: BTreeMap::new(),
            unregistered_signatures: false,
        }
    }

    /// **El momento apuntado sigue ahí para quien monta después** (ID-338).
    #[test]
    fn the_moment_survives_a_window_that_was_not_listening_yet() {
        let live = LiveErrand::default();
        assert!(live.moment().is_none(), "sin trámite no hay momento");

        live.note(Moment::Waiting);
        assert_eq!(live.moment(), Some(Moment::Waiting));
    }

    /// **Leerlo no lo consume**, al revés que la invocación con documento.
    #[test]
    fn reading_the_moment_leaves_it_where_it_was() {
        let live = LiveErrand::default();
        live.note(Moment::Waiting);

        let _ = live.moment();
        assert_eq!(live.moment(), Some(Moment::Waiting));
    }

    /// **El último momento pisa al anterior**: lo que la ventana pide al
    /// montarse es dónde está el trámite ahora, no por dónde pasó.
    #[test]
    fn the_last_moment_is_the_one_that_is_kept() {
        let live = LiveErrand::default();
        live.note(Moment::Waiting);
        live.note(asking_with("FIRMA"));

        assert_eq!(live.moment(), Some(asking_with("FIRMA")));
    }

    /// **ID-276**: los dos consentimientos no son intercambiables, y la
    /// asimetría es la que protege.
    #[test]
    fn a_consented_signature_is_never_an_identity_to_hand_over() {
        let live = LiveErrand::default();
        live.remember_signature(a_pending_signature());

        assert!(live.what_the_site_asked().is_none());
        assert!(live.the_signature_consented().is_some());
    }

    /// Y al revés: con una identificación consentida no hay nada que firmar.
    #[test]
    fn a_consented_identity_is_never_a_signature_to_begin() {
        let live = LiveErrand::default();
        live.remember_identity(SiteFilter::default());

        assert!(live.what_the_site_asked().is_some());
        assert!(live.the_signature_consented().is_none());
    }

    /// Y terminar deja las dos preguntas sin respuesta: lo que se contestó una
    /// vez no se contesta dos (ID-275), y el consentimiento se olvida por el
    /// mismo camino que todo lo demás.
    #[test]
    fn ending_leaves_nothing_to_answer_with() {
        let live = LiveErrand::default();
        live.remember_signature(a_pending_signature());
        live.end();
        assert!(live.the_signature_consented().is_none());

        live.remember_identity(SiteFilter::default());
        live.end();
        assert!(live.what_the_site_asked().is_none());
    }
}
