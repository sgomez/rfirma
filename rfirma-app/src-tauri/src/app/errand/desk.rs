//! **La mesa del trámite** (RD-09): todo lo que un trámite necesita tener a
//! mano, y los dos consentimientos que se deciden sobre ella.
//!
//! Aquí se decide **qué se enseña, qué se contesta y cuándo**: la petición ya
//! leída por el códec entra por [`attend_operation`], y lo que sale es o el
//! momento del consentimiento —con el listado que la sede acepta— o lo que la
//! sede recibe sin que haya nada que consentir. Quien escribe en el cable es
//! [`super::replies`], y sólo él.

use std::path::{Path, PathBuf};

use crate::commands::Failure;
use crate::isolate::Isolate;
use crate::memory::{handles, ListedCertificates, Memory, OpenedDocuments};
use crate::pkcs11::{self, Store};
use crate::protocol::{
    visible_signature_of, AfirmaUrl, SafCode, SelectCertificate, SignRequest, SiteFilter,
    WireAnswer,
};
use crate::signing::{AdmissibleDocument, ALLOW_UNREGISTERED_KEY};

use super::outcome::{ErrandStep, SigningConsent, SiteOutcome};
use super::replies::{answering, no_certificate_at_all, no_certificate_the_site_accepts};
use super::request::SiteRequest;
use super::state::LiveErrand;
use crate::app::filtering::{self, FilterEngine};
use crate::app::frontier;
use crate::app::policies::{self, PolicyEngine};
use crate::app::signing::SigningSession;
use crate::app::Environment;

/// Todo lo que un trámite necesita tener a mano.
///
/// Es un tipo y no diez argumentos porque los diez viajan siempre juntos: son
/// la raíz de composición vista desde el trámite de sede, igual que
/// [`super::Environment`] lo es desde una orden de la ventana.
pub struct ErrandDesk<'a, E: FilterEngine, P: PolicyEngine> {
    /// El motor de filtros, prestado del puente (ID-252).
    pub engine: &'a E,
    /// El expansor de política, prestado del mismo sitio (ID-266).
    pub policies: &'a P,
    /// Dónde se buscan los certificados, **ahora mismo**: los `.p12`
    /// instalados se releen en cada trámite (ID-192).
    pub stores: Vec<Store>,
    /// Dónde viven los `.p12` instalados (ID-192).
    pub installed_dir: &'a Path,
    /// Los certificados listados en esta sesión.
    pub listed: &'a ListedCertificates,
    /// Los documentos abiertos en esta sesión.
    pub opened: &'a OpenedDocuments,
    /// La memoria entre sesiones.
    pub memory: &'a Memory,
    /// Dónde cae el fichero de paso del documento que manda la sede, que se
    /// borra al contestar (ID-286).
    pub scratch_dir: PathBuf,
    /// El hilo dueño del puente, por el que pasa el ciclo trifásico de una
    /// firma consentida (ADR-0001).
    pub isolate: &'a Isolate,
    /// El ciclo a medias, entre el PIN y la postfirma: es el mismo del
    /// recorrido local, porque la fase que toca la clave privada no sabe de
    /// sedes.
    pub session: &'a SigningSession,
}

impl<'a> ErrandDesk<'a, Isolate, Isolate> {
    /// **La mesa de producción**, armada desde la raíz de composición: los dos
    /// motores son el puente en su hilo ([`super::super::engines`]), y **el
    /// directorio de paso es el de los ficheros que no se guardan** —el
    /// documento que manda la sede se borra al contestar (ID-286), así que su
    /// sitio es el temporal—. Que sea aquí donde se decide, y no en la orden,
    /// es lo que deja a la orden en desempaquetar, llamar y traducir (ID-79).
    pub fn at(
        environment: &'a Environment,
        opened: &'a OpenedDocuments,
        isolate: &'a Isolate,
        session: &'a SigningSession,
    ) -> Self {
        Self {
            engine: isolate,
            policies: isolate,
            stores: environment.all_stores(),
            installed_dir: &environment.installed_certificates,
            listed: &environment.listed,
            opened,
            memory: &environment.memory,
            scratch_dir: std::env::temp_dir(),
            isolate,
            session,
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
pub fn attend_operation<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    url: &AfirmaUrl,
    request: SiteRequest,
    live: &LiveErrand,
) -> ErrandStep {
    let operation = match request {
        SiteRequest::NotAttended(refusal) => {
            return answering(live, SiteOutcome::RefusedByTheProtocol(refusal))
        }
        attended => attended,
    };

    // La petición se apunta antes de atenderla: es lo que deja volver a
    // mirar el almacén sin reiniciar el trámite (ID-341).
    live.keep_the_request(url.clone());

    let ours = match pkcs11::list_certificates_across(&desk.stores) {
        Ok(ours) => ours,
        Err(error) => {
            let code = frontier::code_of_token(error.situation());
            return answering(
                live,
                SiteOutcome::Refused {
                    answer: WireAnswer::refused(code),
                    failure: error.into(),
                },
            );
        }
    };

    match operation {
        SiteRequest::SelectCertificate(request) => consent_for(
            desk.engine,
            &request,
            ours,
            desk.installed_dir,
            desk.listed,
            desk.memory,
            live,
        ),
        SiteRequest::Sign(request) => consent_to_sign(desk, &request, ours, live),
        SiteRequest::NotAttended(_) => unreachable!("se ha despachado arriba"),
    }
}

/// **Caso de uso.** El momento del consentimiento de una firma, sobre un
/// listado que **ya** pasó por los criterios de rFirma (ID-258, ID-272).
///
/// El orden de los cuatro pasos es la decisión de este módulo, y ninguno es
/// intercambiable:
///
/// 1. **La admisibilidad primero** (ID-63): un PDF cifrado, certificado o que
///    no es un PDF se rechaza sobre los bytes, sin token y **antes** de que la
///    persona vea nada que consentir. Del mismo husmeo sale si el documento
///    trae firmas que no sabemos leer, que **no es un rechazo**: viaja con el
///    consentimiento para que la pregunta quepa dentro de él (ID-299).
/// 2. **La política después**, porque una que no se puede aplicar hace que no
///    haya firma que ofrecer (ID-266). Y pegado a ella el recuadro, que se lee
///    de los `extraParams` **ya expandidos** (ID-282), en el mismo sitio en el
///    que lo mira el original.
/// 3. **El listado**, con la criba de la sede encima de la de rFirma (ID-258).
/// 4. Y sólo entonces se guarda el documento y se pide el consentimiento: hasta
///    aquí no se ha escrito ni un byte en el disco.
///
/// Es público por lo mismo que [`consent_for`]: **éste no lista el token**, y
/// eso es lo que permite probar la decisión entera en grada A con un listado de
/// andamio (TD-20, TD-51).
pub fn consent_to_sign<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    request: &SignRequest,
    ours: Vec<crate::pkcs11::TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    let admitted = match AdmissibleDocument::check(request.document()) {
        Ok(admitted) => admitted,
        Err(inadmissible) => {
            return answering(
                live,
                SiteOutcome::Refused {
                    answer: WireAnswer::refused(frontier::code_of_inadmissible(inadmissible)),
                    failure: inadmissible.into(),
                },
            )
        }
    };

    let mut from_the_site =
        match policies::expanded_for_the_site(desk.policies, request.declared_params()) {
            Ok(expanded) => expanded,
            Err(error) => {
                return answering(
                    live,
                    SiteOutcome::Refused {
                        answer: WireAnswer::refused(frontier::code_of_bridge(&error)),
                        failure: error.into(),
                    },
                )
            }
        };

    // `allowCosigningUnregisteredSignatures` es de rFirma desde el ID-301: se
    // lee lo que declaró la sede y se **quita** del bloque, para que un `=true`
    // suyo no cruce al puente sin que nadie lo haya consentido.
    let allowed_by_the_site = from_the_site
        .remove(ALLOW_UNREGISTERED_KEY)
        .map(|declared| declared.trim().eq_ignore_ascii_case("true"));
    let unregistered_signatures = admitted.has_unregistered_signatures();
    // `=false` es la sede contestando que no a la pregunta que íbamos a hacer,
    // y una negativa a esa pregunta sale como `CANCEL`, igual que si la hubiera
    // dicho la persona (ID-301, ID-303). `SAF_50` no es de aquí: queda para el
    // puente, que es quien puede ver lo que el husmeo de bytes no vio.
    if unregistered_signatures && allowed_by_the_site == Some(false) {
        return answering(live, SiteOutcome::Cancelled);
    }

    // El recuadro se decide sobre los `extraParams` ya expandidos, que es donde
    // mira el original (ID-282, ID-283, ID-284). Las dos negativas caen aquí, a
    // tiempo: sin visor, sin diálogo y antes de que haya nada que consentir.
    let visible = match visible_signature_of(&from_the_site) {
        Ok(visible) => visible,
        Err(refusal) => return answering(live, SiteOutcome::RefusedByTheProtocol(refusal)),
    };

    let accepted = match accepted_listing(desk, request.filter(), ours, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let document = match keep_the_document(desk, live, request.document()) {
        Ok(document) => document,
        Err(failure) => {
            return answering(
                live,
                SiteOutcome::Refused {
                    answer: WireAnswer::refused(SafCode::CannotSaveData),
                    failure,
                },
            )
        }
    };

    ErrandStep::AskingToSign(SigningConsent {
        document,
        round: request.round(),
        certificates: crate::app::certificates::rows_of(
            accepted,
            desk.installed_dir,
            desk.listed,
            desk.memory,
        ),
        from_the_site,
        visible,
        filter: request.filter().clone(),
        // Un `=true` de la sede **no salta la pregunta** (ID-301): lo que la
        // enciende es lo que dicen los bytes, y nada más.
        unregistered_signatures,
    })
}

/// El listado que la sede acepta, o el paso que la despacha con su código.
///
/// Es el cuerpo que [`consent_for`] y [`consent_to_sign`] comparten: las dos
/// cribas son las mismas y los dos códigos también, porque la sede no distingue
/// si se quedó sin certificados pidiendo identidad o pidiendo firma.
fn accepted_listing<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    filter: &SiteFilter,
    ours: Vec<crate::pkcs11::TokenCertificate>,
    live: &LiveErrand,
) -> Result<Vec<crate::pkcs11::TokenCertificate>, ErrandStep> {
    if ours.is_empty() {
        return Err(no_certificate_at_all());
    }

    let owned = ours.len();
    let accepted =
        filtering::keep_what_the_site_accepts(desk.engine, filter, ours).map_err(|failure| {
            answering(
                live,
                SiteOutcome::Refused {
                    answer: WireAnswer::refused(SafCode::CannotAccessKeystore),
                    failure,
                },
            )
        })?;

    if accepted.is_empty() {
        return Err(no_certificate_the_site_accepts(live, owned));
    }
    Ok(accepted)
}

/// Deja el documento que mandó la sede donde se pueda leer y firmar, **sin que
/// quede rastro de él** (ID-286).
///
/// Entra por [`OpenedDocuments::remember_unrecorded`], que es la puerta que no
/// escribe fila, y el fichero de paso queda apuntado en el trámite vivo para
/// borrarlo al contestar. El nombre es un asa acuñada y no el que la sede
/// quisiera: la sede no nombra ficheros en este equipo.
fn keep_the_document<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
    bytes: &[u8],
) -> Result<String, Failure> {
    std::fs::create_dir_all(&desk.scratch_dir)
        .map_err(|error| Failure::new("folderMissing", error.to_string()))?;
    let path = desk.scratch_dir.join(format!("{}.pdf", handles::mint()));
    std::fs::write(&path, bytes).map_err(|error| Failure::new("unwritable", error.to_string()))?;
    let _ = crate::paths::restrict_to_owner(&path);
    live.keep_the_scratch(path.clone());
    Ok(desk
        .opened
        .remember_unrecorded(crate::destination::PortalDocument::opened(path)))
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
    if ours.is_empty() {
        return no_certificate_at_all();
    }

    let owned = ours.len();
    let accepted = match filtering::keep_what_the_site_accepts(engine, request.filter(), ours) {
        Ok(accepted) => accepted,
        // Lo único que puede fallar después de la criba de rFirma es el motor
        // prestado del puente, y lo que la sede ve de eso es que no se le ha
        // podido servir el almacén: `SAF_08` es lo que el original emite ahí
        // (`ProtocolInvocationLauncherSelectCert.java:217`-`224`).
        Err(failure) => {
            return answering(
                live,
                SiteOutcome::Refused {
                    answer: WireAnswer::refused(SafCode::CannotAccessKeystore),
                    failure,
                },
            )
        }
    };

    if accepted.is_empty() {
        // La sede se entera en el acto (ID-275); la ventana enseña **cuál de
        // las dos** situaciones del ID-278 es (ID-341).
        return no_certificate_the_site_accepts(live, owned);
    }

    // Y aquí **no** se mira cuántos hay: con uno solo se consiente igual
    // (ID-272).
    ErrandStep::AskingForConsent {
        certificates: crate::app::certificates::rows_of(accepted, installed_dir, listed, memory),
        filter: request.filter().clone(),
    }
}
