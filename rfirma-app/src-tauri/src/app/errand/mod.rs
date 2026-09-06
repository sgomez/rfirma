//! **El trámite de sede**: de la operación que llega por el canal a lo que la
//! sede recibe (ID-272, ID-275, ID-276, ID-280).
//!
//! [`super::site`] atiende la **invocación** —abre el canal en uno de los
//! puertos que la sede sorteó— y este módulo atiende lo que viene después: la
//! operación que llega por ese canal ya abierto, el momento del consentimiento
//! y la respuesta. Las operaciones que se atienden son `selectcert`, `sign` y
//! `cosign` ([`crate::protocol::operation`], ID-263).
//!
//! # El documento de la sede no se recuerda (ID-286)
//!
//! Lo que la sede manda entra por [`crate::memory::OpenedDocuments::remember_unrecorded`]
//! —la puerta que **no** deja rastro— y se escribe en un fichero de paso que
//! este módulo borra en cuanto el trámite contesta. De él no queda fila en
//! Recientes, ni colocación del recuadro, ni «último documento»: la postfirma
//! del trámite es [`super::signing::finish_for_the_site`], que ensambla y
//! devuelve los bytes sin escribir nada.
//!
//! # Los dos canales van desacompasados (ID-275)
//!
//! Lo que la sede recibe sale **en el acto**: no espera a que nadie cierre una
//! ventana. Por eso todo lo que este módulo devuelve es un [`SiteOutcome`]
//! —que el códec negociado pone en el cable al cerrarse el trámite— y, cuando
//! algo sale mal, lleva **además** la situación entera para la ventana: el
//! código `SAF_` no puede cargar con la precisión, y la ventana no puede cargar
//! con el acuse.
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

//! # La interfaz son tres verbos, y el estado tiene un solo dueño (RD-01)
//!
//! [`attend`] atiende la operación que llegó por el canal, [`consent`] sigue
//! el trámite con el certificado que la persona eligió, y [`decline`] lo cierra
//! con un `CANCEL`. Sólo estos verbos —y [`finish`], que es la segunda mitad de
//! consentir una firma porque el PIN va en medio— leen y escriben el estado
//! del trámite ([`LiveErrand`]): lo que la sede pidió, por dónde se le contesta,
//! qué se consintió y en qué momento está. Una orden de Tauri desempaqueta el
//! estado, llama a uno de ellos y traduce (ID-79); no decide nada.
//!
//! Sus dependencias entran por dos puertos ([`ports`]): el códec del protocolo,
//! que lee la operación y escribe la respuesta, y el transporte, que trae el
//! mensaje y ofrece el asa por la que se contesta. El trámite no nombra a
//! ningún adaptador concreto de los dos (RD-12).
//!
//! El reparto del módulo (RD-09):
//!
//! | Fichero | Qué es |
//! |---|---|
//! | [`state`] | el estado del trámite, con un solo dueño |
//! | [`request`] | lo que la sede quiere, sin versión |
//! | [`outcome`] | el vocabulario de salida: lo que la sede recibe y lo que queda para la ventana |
//! | [`ports`] | el códec del protocolo y el transporte |
//! | [`desk`] | la mesa del trámite: los dos consentimientos, decididos sobre ella |
//! | [`replies`] | las respuestas finales, y el único sitio que escribe en el cable |

pub mod desk;
pub mod outcome;
pub mod ports;
pub mod replies;
pub mod request;
pub mod state;

#[cfg(test)]
mod tests;

use crate::commands::orders::{SigningOrder, VisibleFieldsOrder};
use crate::commands::Failure;
use crate::pkcs11::StoreSecret;
use crate::protocol::AfirmaUrl;

use crate::app::filtering::FilterEngine;
use crate::app::policies::PolicyEngine;
use crate::app::signing::{self, SiteSigning};

pub use desk::{attend_operation, consent_for, consent_to_sign, ErrandDesk};
pub use outcome::{ErrandStep, Moment, NoCertificate, NoChannel, SigningConsent, SiteOutcome};
pub use ports::{ChannelTransport, Inbox, ProtocolCodec, ReplyHandle, Transport};
pub use replies::{
    declined, identify_with, identity_handed_over, signature_handed_over,
    the_signature_did_not_come_out,
};
pub use request::SiteRequest;
pub use state::{Errand, LiveErrand, NegotiatedCodec};

/// **Verbo 1.** Atiende la operación que llegó por el canal ya abierto, con el
/// asa por la que se le contestará a la sede (ID-320, ID-330).
///
/// Lo primero, antes de nada que pueda contestar, es apuntar el asa: es por
/// donde sale todo lo que este trámite le diga a la sede (ID-321). Después la
/// operación se lee con el códec negociado, se atiende sobre la mesa, y de lo
/// que salga se apunta lo que haga falta para seguir —qué se consintió— y el
/// momento que la ventana tiene que enseñar (ID-338).
///
/// Devuelve `None` sólo si no hay códec negociado, y eso no pasa por el camino
/// del transporte: el canal por el que llega la operación lo abrió la
/// negociación de arranque, que es la que apuntó el códec.
pub fn attend<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    url: AfirmaUrl,
    reply: ReplyHandle,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    live.answer_through(reply);
    dispatch(desk, url, live)
}

/// **Verbo 1, otra vez.** Vuelve a atender la petición que la sede mandó, por
/// si se instaló un certificado con la ventana abierta (ID-278, ID-341).
///
/// **Continúa el trámite, no lo reinicia**: la misma petición, el mismo canal,
/// la misma asa —que se apuntó una sola vez, en [`attend`]— y el mismo trámite
/// vivo. La sede no ha recibido nada todavía y no tiene que invocar otra vez.
///
/// Sin petición apuntada no hay nada que volver a mirar, y es la respuesta
/// correcta: quien llegue aquí después de que el trámite haya contestado no
/// mueve nada.
pub fn look_again<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let url = live.the_request()?;
    dispatch(desk, url, live)
}

/// Lee la operación con el códec negociado, la atiende, y apunta lo que queda
/// para seguir y lo que la ventana enseña.
fn dispatch<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    url: AfirmaUrl,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let codec = live.codec()?;
    let step = desk::attend_operation(desk, &url, codec.decode(&url), live);

    match &step {
        ErrandStep::AskingForConsent { filter, .. } => live.remember_identity(filter.clone()),
        // Lo que la ventana enseña y lo que se queda para hacer cumplir lo que
        // pidió la sede son las dos mitades de lo mismo (ID-259, ID-266).
        ErrandStep::AskingToSign(asked) => live.remember_signature(state::PendingSignature {
            document: asked.document.clone(),
            filter: asked.filter.clone(),
            from_the_site: asked.from_the_site.clone(),
            unregistered_signatures: asked.unregistered_signatures,
        }),
        // **El callejón que sí tiene arreglo** (ID-278, ID-341): aquí no hay
        // nada que consentir ni nada que elegir, así que lo que quedara
        // apuntado de un reparto anterior no vale ya para nada. Con `NotOne`
        // el trámite sigue vivo —no se ha escrito nada en el cable— esperando
        // a que se instale uno y se vuelva a mirar.
        ErrandStep::NoCertificate { .. } => live.forget_the_consent(),
        // Ya está contestada: la mesa cerró el trámite y escribió la línea por
        // el asa (ID-322), y cerrarlo es lo que olvida el consentimiento.
        ErrandStep::Answering(_) => {}
    }

    if let Some(moment) = step.moment() {
        live.note(moment);
    }
    Some(step)
}

/// En qué queda consentir: identidad entregada, o una firma que sigue por el
/// PIN.
#[derive(Debug)]
pub enum Consented {
    /// La sede ya tiene el certificado (ID-275, ID-276), y el trámite ha
    /// terminado.
    IdentityHandedOver,
    /// La prefirma ha abierto el ciclo y esto es cómo hay que pedir el
    /// secreto al almacén (ID-189): el PIN entra por la misma orden que en
    /// el recorrido local, porque la fase que toca la clave privada no sabe de
    /// sedes (ADR-0001).
    SigningWith(StoreSecret),
}

/// **Verbo 2.** La persona consiente con uno de los certificados que tenía
/// delante (ID-272, ID-276).
///
/// Qué se consintió lo dice el estado, no la ventana: de ella llega **sólo el
/// asa del certificado**, y el documento, el filtro y la política que declaró
/// la sede salen de lo que se apuntó al atender, porque hacerlos cumplir no es
/// cosa de la ventana (ID-259, ID-266).
///
/// - Si lo pendiente era una **identificación**, la sede recibe el certificado
///   en el acto y el trámite termina.
/// - Si era una **firma**, se abre el ciclo (la prefirma de la sede) y lo que
///   vuelve es cómo pedir el secreto; el trámite termina en [`finish`].
///
/// Si algo no sale, la sede se entera en el acto con el código que le toca a
/// la situación (ID-292), el trámite se cierra y **el consentimiento se olvida
/// por el mismo camino** —cerrar el trámite es lo que lo olvida—; la ventana
/// recibe además la situación entera (ID-29, ID-291).
pub fn consent<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, Failure> {
    if let Some(filter) = live.what_the_site_asked() {
        let outcome = identify_with(
            desk.engine,
            &desk.stores,
            &filter,
            certificate,
            desk.listed,
            live,
        );
        return match outcome.failure() {
            Some(failure) => Err(failure.clone()),
            None => Ok(Consented::IdentityHandedOver),
        };
    }

    let Some(pending) = live.the_signature_consented() else {
        return Err(Failure::new(
            "siteErrandNotLive",
            "no hay ninguna identificacion ni firma pendiente que contestar",
        ));
    };

    let order = SigningOrder {
        document: pending.document,
        certificate: certificate.to_owned(),
        // La sede coloca su recuadro en sus propios `extraParams`, y ésos
        // cruzan al puente crudos: aquí no hay visor sobre el que arrastrar
        // nada, y emitir una colocación nuestra movería el suyo (ID-282).
        placement: None,
        fields: VisibleFieldsOrder::default(),
        reason: String::new(),
        signed_at: String::new(),
        rubric: None,
        language: String::new(),
        allow_unregistered_signatures: pending.unregistered_signatures,
    };

    signing::begin_for_the_site(
        &SiteSigning {
            engine: desk.engine,
            filter: &pending.filter,
            from_the_site: &pending.from_the_site,
        },
        &order,
        &desk.stores,
        desk.listed,
        desk.opened,
        desk.isolate,
        desk.session,
    )
    .map(Consented::SigningWith)
    // El código que va al cable lo trae la negativa desde donde la situación
    // todavía tenía tipo (ID-292): aquí sólo se separa lo que recibe la sede
    // de lo que recibe la ventana, que es la situación entera (ID-291).
    .map_err(|refusal| {
        let failure = refusal.failure().clone();
        the_signature_did_not_come_out(live, refusal);
        failure
    })
}

/// **La segunda mitad de consentir una firma.** Postfirma del trámite de sede:
/// la sede recibe el certificado y el PDF firmado, y con eso el trámite termina
/// (ID-275).
///
/// **No devuelve el documento**, y esa ausencia es la decisión: el firmado de
/// una sede no cae en ninguna carpeta, no anota fila en la bandeja y no cambia
/// el certificado recordado (ID-264, ID-286). Lo que la ventana enseña después
/// es un desenlace, no un fichero.
pub fn finish<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
) -> Result<(), Failure> {
    let signed = signing::finish_for_the_site(desk.isolate, desk.session).map_err(|refusal| {
        let failure = refusal.failure().clone();
        the_signature_did_not_come_out(live, refusal);
        failure
    })?;
    signature_handed_over(live, &signed);
    Ok(())
}

/// **Verbo 3.** La persona dice que no: la sede recibe `CANCEL` en el acto
/// (ID-293, ID-275).
///
/// Contestada la sede, ya no queda asa: cancelar dos veces —o cerrar la ventana
/// después de haber contestado— no escribe nada (ID-340). Y con la respuesta
/// se va el consentimiento apuntado: no hay nada que contestar con él.
pub fn decline(live: &LiveErrand) -> SiteOutcome {
    declined(live)
}
