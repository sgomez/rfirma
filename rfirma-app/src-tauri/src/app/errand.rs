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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine as _;

use crate::commands::views::CertificateView;
use crate::commands::Failure;
use crate::memory::{handles, ListedCertificates, Memory, OpenedDocuments};
use crate::pkcs11::{self, Store};
use crate::protocol::{
    read_operation, visible_signature_of, ChannelCredential, Refusal, SafCode, SelectCertificate,
    SignRequest, SignatureRound, SiteFilter, SiteOperation, SiteVisibleSignature, WireAnswer,
};
use crate::signing::{AdmissibleDocument, ALLOW_UNREGISTERED_KEY};

use super::filtering::{self, FilterEngine};
use super::frontier;
use super::policies::{self, PolicyEngine};
use super::signing::SiteSignature;

/// **El trámite vivo del proceso**, si lo hay (ID-280).
///
/// Guarda además el **fichero de paso** del documento que mandó la sede, y lo
/// borra al terminar: de ese documento no queda rastro ninguno (ID-286), y el
/// único sitio donde se sabe que el trámite ha acabado es aquí.
#[derive(Default)]
pub struct LiveErrand {
    errand: Mutex<Option<Errand>>,
    scratch: Mutex<Option<PathBuf>>,
}

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
    /// Apunta el fichero de paso del documento que mandó la sede, para
    /// borrarlo al contestar.
    fn keep_the_scratch(&self, path: PathBuf) {
        *super::lock(&self.scratch) = Some(path);
    }

    /// Apunta el trámite que empieza. **No sustituye**: con uno vivo devuelve
    /// `false` y el que llega se queda fuera (ID-280).
    ///
    /// Es **la única puerta** del trámite único, y por eso mira y apunta bajo
    /// el mismo candado: quien la llame decide con lo que devuelve y no con un
    /// [`Self::current`] anterior, que sería mirar por una toma y apuntar por
    /// otra. Su valor de retorno no es opcional, y por eso no hay ningún
    /// «¿hay trámite vivo?» que preguntar antes: la plaza se pide aquí.
    #[must_use = "con uno vivo devuelve false y el que llega no queda apuntado (ID-280)"]
    pub fn begin(&self, errand: Errand) -> bool {
        let mut live = super::lock(&self.errand);
        if live.is_some() {
            return false;
        }
        *live = Some(errand);
        true
    }

    /// El trámite vivo, si lo hay.
    pub fn current(&self) -> Option<Errand> {
        super::lock(&self.errand).clone()
    }

    /// Se acabó: la sede ya tiene su respuesta.
    ///
    /// Se llama **al contestar** y no al cerrar la ventana, que es lo mismo que
    /// dice el ID-275 desde el otro lado: el desenlace que la ventana enseña ya
    /// no es parte del trámite.
    ///
    /// **Contestar no es la única salida, y hoy es la única que llama aquí.**
    /// Una sede que se cae con el canal abierto, o una ventana de sede que se
    /// cierra con el aspa en vez de cancelar, dejarían el trámite vivo para
    /// siempre y todo `afirma://` posterior rechazado con `SAF_45`. No es
    /// alcanzable mientras [`super::site::attend_launch`] no esté cableado;
    /// quien lo cablee —la ventana de sede (#362) y los manejadores (#357)— ha
    /// de atar **el cierre del canal** a esta llamada.
    pub fn end(&self) {
        *super::lock(&self.errand) = None;
        // Y el documento que mandó la sede se va con él: de él no queda rastro
        // ninguno (ID-286). Si el borrado falla —el fichero ya no está, o el
        // directorio se ha ido— no hay nada que contarle a nadie: el trámite ha
        // terminado y esto es limpieza.
        if let Some(scratch) = super::lock(&self.scratch).take() {
            let _ = std::fs::remove_file(scratch);
        }
    }

    /// El fichero de paso apuntado, si lo hay. **Sólo para las pruebas**: nadie
    /// del recorrido necesita la ruta, que es justamente lo que no cruza.
    #[cfg(test)]
    pub fn scratch_path(&self) -> Option<PathBuf> {
        super::lock(&self.scratch).clone()
    }
}

/// En qué queda la operación que llegó por el canal.
#[derive(Debug)]
pub enum ErrandStep {
    /// **El momento del consentimiento** (ID-272, ID-276): la ventana enseña
    /// estas filas y la persona decide. La sede no recibe nada todavía.
    AskingForConsent(Vec<CertificateView>),
    /// **El momento del consentimiento de una firma** (ID-272): la ventana
    /// enseña el documento que la sede manda y estas filas, y la persona
    /// decide. La sede no recibe nada todavía.
    AskingToSign(SigningConsent),
    /// No hay nada que consentir: esto es lo que la sede recibe, y sale ya
    /// (ID-275).
    Answering(SiteReply),
}

/// Lo que hay delante de la persona cuando una sede pide una firma.
///
/// No es una vista: es lo que hace falta para **seguir** el trámite si se
/// consiente, y por eso lleva dentro cosas que la ventana no pinta —el filtro y
/// los `extraParams`—. Las dos se vuelven a usar en la prefirma: el filtro
/// porque se comprueba otra vez antes del PIN (ID-259) y los `extraParams`
/// porque son la política que la sede declaró (ID-266).
#[derive(Debug)]
pub struct SigningConsent {
    /// El identificador con el que la ventana nombra el documento de la sede.
    /// **No es una ruta** (ADR-0011).
    pub document: String,
    /// `sign` o `cosign`, que es lo que hay que contarle a la persona.
    pub round: SignatureRound,
    /// Los certificados que la sede acepta, ya cribados.
    pub certificates: Vec<CertificateView>,
    /// Los `extraParams` de la sede, **ya expandidos** (ID-266).
    pub from_the_site: BTreeMap<String, String>,
    /// Qué recuadro pide la sede, ya decidido (ID-282). Lo que hay que saber de
    /// él está en [`crate::protocol::visible`]; aquí sólo viaja, porque la
    /// prefirma no vuelve a mirar los `extraParams` para averiguarlo.
    pub visible: SiteVisibleSignature,
    /// Lo que la sede pide del listado, para volver a comprobarlo (ID-259).
    pub filter: SiteFilter,
    /// Que el documento trae **firmas que rFirma no sabe leer** (ID-297).
    ///
    /// La pregunta vive **dentro** de este consentimiento y no en un sexto
    /// momento (ID-298): la ventana lo enseña con lo demás, y si la persona
    /// dice que no, lo que sale es `CANCEL` (ID-303). No hay recuento ni
    /// titulares detrás, y las firmas previas no se validan (ID-305).
    pub unregistered_signatures: bool,
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
    /// La firma que la sede pidió, en la forma que espera el cliente publicado:
    /// el certificado y la firma en Base64 URL-safe, separados por `|`
    /// (`NativeSignDataProcessor.java:53`-`104`, `RESULT_SEPARATOR` en `:23`).
    ///
    /// El tercer campo —`extraData`— **no se emite**: sólo lleva el nombre del
    /// fichero cargado, y aquí el documento lo mandó la sede.
    Signature {
        /// El DER del firmante, en Base64 URL-safe.
        certificate: String,
        /// El PDF firmado, en Base64 URL-safe.
        signature: String,
    },
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
            Self::Signature {
                certificate,
                signature,
            } => format!("{certificate}{RESULT_SEPARATOR}{signature}"),
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

/// El separador de los campos de la respuesta de firma
/// (`NativeSignDataProcessor.java:23`).
const RESULT_SEPARATOR: char = '|';

/// Todo lo que un trámite necesita tener a mano.
///
/// Es un tipo y no ocho argumentos porque los ocho viajan siempre juntos: son
/// la raíz de composición vista desde el trámite de sede, igual que
/// [`super::Environment`] lo es desde una orden de la ventana.
pub struct ErrandDesk<'a, E: FilterEngine, P: PolicyEngine> {
    /// El motor de filtros, prestado del puente (ID-252).
    pub engine: &'a E,
    /// El expansor de política, prestado del mismo sitio (ID-266).
    pub policies: &'a P,
    /// Dónde se buscan los certificados.
    pub stores: &'a [Store],
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
    pub scratch_dir: &'a Path,
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
    url: &crate::protocol::AfirmaUrl,
    live: &LiveErrand,
) -> ErrandStep {
    let operation = match read_operation(url) {
        Ok(operation) => operation,
        Err(refusal) => return answering(live, SiteReply::RefusedByTheProtocol(refusal)),
    };

    let ours = match pkcs11::list_certificates_across(desk.stores) {
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

    match operation {
        SiteOperation::SelectCertificate(request) => consent_for(
            desk.engine,
            &request,
            ours,
            desk.installed_dir,
            desk.listed,
            desk.memory,
            live,
        ),
        SiteOperation::Sign(request) => consent_to_sign(desk, &request, ours, live),
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
                SiteReply::Refused {
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
                    SiteReply::Refused {
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
        return answering(live, SiteReply::Cancelled);
    }

    // El recuadro se decide sobre los `extraParams` ya expandidos, que es donde
    // mira el original (ID-282, ID-283, ID-284). Las dos negativas caen aquí, a
    // tiempo: sin visor, sin diálogo y antes de que haya nada que consentir.
    let visible = match visible_signature_of(&from_the_site) {
        Ok(visible) => visible,
        Err(refusal) => return answering(live, SiteReply::RefusedByTheProtocol(refusal)),
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
                SiteReply::Refused {
                    answer: WireAnswer::refused(SafCode::CannotSaveData),
                    failure,
                },
            )
        }
    };

    ErrandStep::AskingToSign(SigningConsent {
        document,
        round: request.round(),
        certificates: super::certificates::rows_of(
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
    let accepted =
        filtering::keep_what_the_site_accepts(desk.engine, filter, ours).map_err(|failure| {
            answering(
                live,
                SiteReply::Refused {
                    answer: WireAnswer::refused(SafCode::CannotAccessKeystore),
                    failure,
                },
            )
        })?;

    if accepted.is_empty() {
        return Err(answering(
            live,
            SiteReply::Refused {
                answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
                failure: Failure::new(
                    "certificateNotFound",
                    "no queda ningun certificado que la sede acepte",
                ),
            },
        ));
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
    std::fs::create_dir_all(desk.scratch_dir)
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

/// **Caso de uso.** Lo que la sede recibe cuando la firma ha terminado
/// (ID-275).
///
/// El certificado delante y la firma detrás, separados por `|`, los dos en
/// Base64 URL-safe: es lo que `processSignResponse` parte
/// (`autoscript.js:2512`-`2549`).
pub fn signature_handed_over(live: &LiveErrand, signed: &SiteSignature) -> SiteReply {
    over(
        live,
        SiteReply::Signature {
            certificate: on_the_wire(&signed.signer_der),
            signature: on_the_wire(&signed.signed),
        },
    )
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
    use crate::app::in_hand::DocumentInHand;
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
            read_operation(url).expect("es una operacion que se atiende")
        else {
            panic!("es una seleccion de certificado");
        };
        request
    }

    /// El puesto de trabajo del trámite, con todo doblado (TD-51, TD-52).
    #[expect(
        clippy::too_many_arguments,
        reason = "es el constructor de un tipo de ocho campos, no una interfaz"
    )]
    fn a_desk<'a>(
        engine: &'a AnEngine,
        policies: &'a APolicyEngine,
        stores: &'a [Store],
        home: &'a Path,
        listed: &'a ListedCertificates,
        opened: &'a OpenedDocuments,
        memory: &'a Memory,
        scratch: &'a Path,
    ) -> ErrandDesk<'a, AnEngine, APolicyEngine> {
        ErrandDesk {
            engine,
            policies,
            stores,
            installed_dir: home,
            listed,
            opened,
            memory,
            scratch_dir: scratch,
        }
    }

    /// Un expansor de política doblado: devuelve lo que se le programó, y
    /// apunta lo que se le pidió.
    struct APolicyEngine {
        asked: RefCell<Vec<String>>,
        answer: Result<String, ()>,
    }

    impl APolicyEngine {
        fn answering(block: &str) -> Self {
            Self {
                asked: RefCell::new(Vec::new()),
                answer: Ok(block.to_owned()),
            }
        }

        fn that_refuses_the_policy() -> Self {
            Self {
                asked: RefCell::new(Vec::new()),
                answer: Err(()),
            }
        }
    }

    impl PolicyEngine for APolicyEngine {
        fn expand(
            &self,
            extra_params: &str,
            _format: &str,
        ) -> Result<String, crate::ffi::BridgeError> {
            self.asked.borrow_mut().push(extra_params.to_owned());
            self.answer.clone().map_err(|()| {
                crate::ffi::BridgeError::IncompatiblePolicy("no se puede aplicar".to_owned())
            })
        }
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
            matches!(attendance, Attendance::Serving { .. }),
            "la invocacion es buena: {attendance:?}"
        );
        assert!(
            live.current().is_some(),
            "el tramite queda vivo mientras se atiende"
        );

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
        assert!(
            live.current().is_some(),
            "consintiendo, el tramite sigue vivo"
        );

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
            live.current().is_none(),
            "contestada la sede, el tramite deja de estar vivo sin que nadie cierre nada (ID-275)"
        );
    }

    /// Un PDF mínimo, que es lo que la sede manda dentro de `dat`.
    const A_PDF: &[u8] = b"%PDF-1.7\n";

    /// Un PDF con una firma previa **que rFirma no sabe leer** (ID-297).
    const A_PDF_SIGNED_BY_SOMETHING_ELSE: &[u8] =
        b"%PDF-1.7\n9 0 obj\n<< /Type /Sig /SubFilter /adbe.pkcs7.whatever >>\nendobj\n";

    /// La petición de firma ya leída, que es lo que recibe el caso de uso.
    fn signature_requested(url: &AfirmaUrl) -> SignRequest {
        let SiteOperation::Sign(request) =
            read_operation(url).expect("es una operacion que se atiende")
        else {
            panic!("es una firma");
        };
        request
    }

    /// La operación de firma tal y como llega por el canal.
    fn a_signature(verb: &str, extra: &str) -> AfirmaUrl {
        a_signature_over(A_PDF, verb, extra)
    }

    /// La misma operación, sobre el documento que se le diga.
    fn a_signature_over(pdf: &[u8], verb: &str, extra: &str) -> AfirmaUrl {
        let document = base64::engine::general_purpose::URL_SAFE.encode(pdf);
        let text = format!(
            "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format=PAdES&\
             algorithm=SHA256withRSA&dat={document}{extra}"
        );
        let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
            panic!("una URL del protocolo es una operacion");
        };
        url
    }

    /// **El trazador de la firma** (TD-51): invocación, canal, operación leída
    /// del mensaje, política expandida, listado filtrado, consentimiento y
    /// respuesta, sin abrir un socket y sin token (TD-52).
    ///
    /// La firma de verdad —prefirma, PIN y postfirma— es la grada C; lo que
    /// esta prueba fija es la decisión que hay a cada lado de ella.
    #[test]
    fn a_signature_goes_from_the_launch_to_the_consent_and_back_to_the_wire() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let opened = OpenedDocuments::new();
        let live = LiveErrand::default();
        let asked = RefCell::new(Vec::new());
        let engine = AnEngine::answering(&[&[0]]);
        let policies =
            APolicyEngine::answering("policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n");
        let scratch = home.path().join("errand");

        // 1. La sede invoca, y el canal queda sirviendo su conversación.
        let attendance = attend_launch(&a_launch("54001,54002,54003"), &a_transport(&asked), &live);
        assert!(
            matches!(attendance, Attendance::Serving { .. }),
            "la invocacion es buena: {attendance:?}"
        );

        // 2. Por ese canal llega la firma, y lo que sale es el consentimiento.
        let step = consent_to_sign(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                &scratch,
            ),
            &signature_requested(&a_signature("sign", "")),
            ours.clone(),
            &live,
        );
        let ErrandStep::AskingToSign(consent) = step else {
            panic!("hay un certificado que la sede acepta: {step:?}");
        };
        assert_eq!(consent.round, SignatureRound::First);
        assert_eq!(consent.certificates.len(), 1);
        assert_eq!(
            consent
                .from_the_site
                .get("policyIdentifier")
                .map(String::as_str),
            Some("urn:oid:2.16.724.1.3.1.1.2.1.9"),
            "la politica la expandio el motor del original (ID-266)"
        );
        assert_eq!(
            std::fs::read(
                DocumentInHand::taken(&opened, &consent.document)
                    .expect("el documento esta en la mano")
                    .reading_path()
            )
            .expect("el fichero de paso existe"),
            A_PDF,
            "lo que se firma es lo que la sede mando"
        );
        assert!(
            live.current().is_some(),
            "consintiendo, el tramite sigue vivo"
        );

        // 3. La firma termina y la sede recibe certificado y firma, en ese
        //    orden y separados por `|`.
        let reply = signature_handed_over(
            &live,
            &SiteSignature {
                signed: b"%PDF-1.7 firmado".to_vec(),
                signer_der: ours[0].der().to_vec(),
            },
        );
        let encode = base64::engine::general_purpose::URL_SAFE;
        assert_eq!(
            reply.on_the_wire(),
            format!(
                "{}|{}",
                encode.encode(ours[0].der()),
                encode.encode(b"%PDF-1.7 firmado")
            )
        );
        assert!(
            live.current().is_none(),
            "contestada la sede, el tramite deja de estar vivo (ID-275)"
        );
    }

    /// **ID-286 / TD-64**: del documento que manda la sede no queda fila en
    /// Recientes, ni colocación del recuadro, ni fichero de paso.
    #[test]
    fn the_document_a_site_sends_leaves_no_trace_at_all() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let opened = OpenedDocuments::new();
        let live = LiveErrand::default();
        let engine = AnEngine::answering(&[&[0]]);
        let policies = APolicyEngine::answering("");
        let scratch = home.path().join("errand");

        let step = consent_to_sign(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                &scratch,
            ),
            &signature_requested(&a_signature("cosign", "")),
            ours.clone(),
            &live,
        );

        let ErrandStep::AskingToSign(consent) = step else {
            panic!("hay un certificado que la sede acepta: {step:?}");
        };
        assert_eq!(consent.round, SignatureRound::Again);
        assert!(
            !DocumentInHand::taken(&opened, &consent.document)
                .expect("el documento esta en la mano")
                .is_remembered(),
            "el documento de la sede entra por la puerta que no recuerda (ID-286)"
        );
        assert!(
            super::super::recents::listed_rows(&memory, &opened).is_empty(),
            "no deja fila en Recientes"
        );
        assert_eq!(
            memory
                .state()
                .map(crate::memory::Loaded::into_value)
                .ok()
                .and_then(|state| state.visible_signature),
            None,
            "ni colocacion del recuadro"
        );

        // Y el fichero de paso se va con el trámite.
        let scratch_file = live.scratch_path().expect("hay fichero de paso");
        assert!(scratch_file.exists());
        declined(&live);
        assert!(
            !scratch_file.exists(),
            "el fichero de paso se borra al contestar (ID-286)"
        );
    }

    /// **ID-282**: cuando la sede manda posición y página, la firma sigue
    /// adelante y el consentimiento lo dice: hay recuadro, y lo puso ella.
    #[test]
    fn a_box_the_site_placed_is_honoured_and_the_signature_goes_on() {
        let asked = a_consent_to_sign(
            "signaturePositionOnPageLowerLeftX=100\n\
             signaturePositionOnPageLowerLeftY=100\n\
             signaturePositionOnPageUpperRightX=300\n\
             signaturePositionOnPageUpperRightY=180\n\
             signaturePages=-1\n\
             visibleSignature=want\n",
        );

        let ErrandStep::AskingToSign(consent) = asked else {
            panic!("hay recuadro y hay certificado: {asked:?}");
        };
        assert_eq!(consent.visible, SiteVisibleSignature::PlacedByTheSite);
        assert_eq!(
            consent
                .from_the_site
                .get("signaturePages")
                .map(String::as_str),
            Some("-1"),
            "la pagina contada desde el final la resuelve el puente, no rFirma (ID-284)"
        );
    }

    /// **ID-282**: y sin ella, `optional` firma invisible sin hacer esperar a
    /// nadie.
    #[test]
    fn an_optional_box_the_site_never_placed_is_signed_invisible() {
        let asked = a_consent_to_sign("visibleSignature=optional\nvisibleAppearance=custom\n");

        let ErrandStep::AskingToSign(consent) = asked else {
            panic!("se firma igual, sin recuadro: {asked:?}");
        };
        assert_eq!(consent.visible, SiteVisibleSignature::Declined);
    }

    /// **ID-283**: un recuadro obligatorio que no viene colocado cancela con el
    /// código que el original ya tiene, y **antes** de que haya nada que
    /// consentir.
    #[test]
    fn a_mandatory_box_the_site_never_placed_cancels_before_anyone_is_asked() {
        let asked = a_consent_to_sign("visibleSignature=want\n");

        let ErrandStep::Answering(reply) = asked else {
            panic!("no hay donde colocar el recuadro: {asked:?}");
        };
        assert!(
            reply.on_the_wire().starts_with("SAF_43"),
            "lo que sale es el codigo de la firma visible: {}",
            reply.on_the_wire()
        );
    }

    /// **ID-284**: y una página en blanco añadida al documento se rechaza,
    /// porque eso es modificarlo antes de firmarlo. Con el recuadro puesto,
    /// que es la única situación en la que el original la añade de verdad.
    #[test]
    fn a_page_appended_to_the_document_is_refused_before_anyone_is_asked() {
        let asked = a_consent_to_sign(
            "signaturePositionOnPageLowerLeftX=100\n\
             signaturePositionOnPageLowerLeftY=100\n\
             signaturePositionOnPageUpperRightX=300\n\
             signaturePositionOnPageUpperRightY=180\n\
             signaturePages=append\n",
        );

        let ErrandStep::Answering(reply) = asked else {
            panic!("no se anaden paginas: {asked:?}");
        };
        assert!(
            reply.on_the_wire().starts_with("SAF_03"),
            "lo que sale es el rechazo del parametro: {}",
            reply.on_the_wire()
        );
    }

    /// Pero sin recuadro el original no añade ninguna página, así que el
    /// trámite sigue adelante y se firma invisible.
    #[test]
    fn an_appended_page_without_a_box_never_happens_and_the_errand_goes_on() {
        let asked = a_consent_to_sign("signaturePages=append\n");

        assert!(
            matches!(asked, ErrandStep::AskingToSign(_)),
            "sin esquinas no hay pagina que anadir: {asked:?}"
        );
    }

    /// El consentimiento de una firma cuya política se expande a ese bloque:
    /// lo que cambia entre las pruebas del recuadro es sólo eso.
    fn a_consent_to_sign(expanded: &str) -> ErrandStep {
        a_consent_to_sign_over(A_PDF, expanded)
    }

    /// El mismo consentimiento, sobre el documento que se le diga.
    fn a_consent_to_sign_over(pdf: &[u8], expanded: &str) -> ErrandStep {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let opened = OpenedDocuments::new();
        let live = LiveErrand::default();
        let engine = AnEngine::answering(&[&[0]]);
        let policies = APolicyEngine::answering(expanded);
        let scratch = home.path().join("errand");

        consent_to_sign(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                &scratch,
            ),
            &signature_requested(&a_signature_over(pdf, "sign", "")),
            ours,
            &live,
        )
    }

    /// **ID-297 / ID-298 / ID-299**: un PDF con firmas que rFirma no sabe leer
    /// **no se rechaza**; la pregunta viaja dentro del consentimiento.
    #[test]
    fn a_pdf_with_signatures_it_cannot_read_is_asked_about_inside_the_consent() {
        let asked = a_consent_to_sign_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "");

        let ErrandStep::AskingToSign(consent) = asked else {
            panic!("no es un rechazo, es un aviso: {asked:?}");
        };
        assert!(consent.unregistered_signatures);
    }

    /// **ID-299**: y de un PDF corriente no se dice nada.
    #[test]
    fn an_ordinary_pdf_asks_about_no_unregistered_signature() {
        let asked = a_consent_to_sign("");

        let ErrandStep::AskingToSign(consent) = asked else {
            panic!("hay certificado que la sede acepta: {asked:?}");
        };
        assert!(!consent.unregistered_signatures);
    }

    /// **ID-301**: la sede puede declarar que le vale, y aun así se pregunta.
    /// Lo que **no** puede es colar su `=true` hasta el puente: la clave sale
    /// del bloque que viaja con el consentimiento.
    #[test]
    fn a_site_that_allows_unregistered_signatures_does_not_skip_the_question() {
        let asked = a_consent_to_sign_over(
            A_PDF_SIGNED_BY_SOMETHING_ELSE,
            "allowCosigningUnregisteredSignatures=true\n",
        );

        let ErrandStep::AskingToSign(consent) = asked else {
            panic!("se pregunta igual: {asked:?}");
        };
        assert!(consent.unregistered_signatures);
        assert!(
            !consent
                .from_the_site
                .contains_key("allowCosigningUnregisteredSignatures"),
            "al puente solo se le manda tras el consentimiento (ID-301)"
        );
    }

    /// **ID-301 / ID-303**: y si la sede dice que no, se respeta como rechazo,
    /// y lo que sale al cable es `CANCEL` —nunca `SAF_50`, que queda para lo
    /// que sólo el puente puede ver—.
    #[test]
    fn a_site_that_forbids_unregistered_signatures_is_answered_with_a_cancel() {
        let asked = a_consent_to_sign_over(
            A_PDF_SIGNED_BY_SOMETHING_ELSE,
            "allowCosigningUnregisteredSignatures=false\n",
        );

        let ErrandStep::Answering(reply) = asked else {
            panic!("la sede ya contesto que no: {asked:?}");
        };
        assert_eq!(reply.on_the_wire(), "CANCEL");
    }

    /// Y ese `=false` no rechaza nada cuando no hay ninguna firma sin
    /// registrar: es la respuesta a una pregunta que no se hace.
    #[test]
    fn a_site_that_forbids_unregistered_signatures_still_signs_an_ordinary_pdf() {
        let asked = a_consent_to_sign("allowCosigningUnregisteredSignatures=false\n");

        let ErrandStep::AskingToSign(consent) = asked else {
            panic!("no hay nada que rechazar: {asked:?}");
        };
        assert!(!consent.unregistered_signatures);
    }

    /// **ID-266**: una política que no se puede aplicar no se firma alrededor,
    /// y la sede recibe el código que el catálogo tiene para eso.
    #[test]
    fn a_policy_that_cannot_be_applied_is_answered_with_the_code_of_an_invalid_policy() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let opened = OpenedDocuments::new();
        let live = LiveErrand::default();
        let engine = AnEngine::answering(&[&[0]]);
        let policies = APolicyEngine::that_refuses_the_policy();
        let scratch = home.path().join("errand");

        let step = consent_to_sign(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                &scratch,
            ),
            &signature_requested(&a_signature("sign", "")),
            ours.clone(),
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("la politica no se puede aplicar: {step:?}");
        };
        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(SafCode::InvalidPolicy).on_the_wire()
        );
        assert!(
            !scratch.exists(),
            "no se ha escrito nada: la politica se mira antes que el documento"
        );
    }

    /// Lo que no se puede firmar se rechaza **sobre los bytes**, antes de pedir
    /// nada y antes de escribir nada (ID-63, ID-292).
    #[test]
    fn a_document_that_is_not_a_pdf_is_refused_before_anything_is_written() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let ours = vec![a_usable_certificate("FIRMA")];
        let (listed, _) = listed_from(&ours);
        let opened = OpenedDocuments::new();
        let live = LiveErrand::default();
        let engine = AnEngine::answering(&[&[0]]);
        let policies = APolicyEngine::answering("");
        let scratch = home.path().join("errand");
        let text = format!(
            "afirma://sign?op=sign&idsession={CREDENTIAL}&format=PAdES&algorithm=SHA256&dat={}",
            base64::engine::general_purpose::URL_SAFE.encode(b"esto no es un PDF")
        );
        let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
            panic!("una URL del protocolo es una operacion");
        };

        let step = consent_to_sign(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                &scratch,
            ),
            &signature_requested(&url),
            ours.clone(),
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("eso no es un PDF: {step:?}");
        };
        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(SafCode::InvalidPdf).on_the_wire()
        );
        assert!(!scratch.exists(), "no se ha escrito nada");
    }

    /// **ID-263**: la contrafirma llega hasta aquí como `SAF_04`, por el mismo
    /// camino que cualquier otro rechazo del protocolo, y sin tocar el token.
    #[test]
    fn a_countersignature_is_answered_with_the_code_of_an_unsupported_operation() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let listed = ListedCertificates::new();
        let opened = OpenedDocuments::new();
        let live = LiveErrand::default();
        let engine = AnEngine::answering(&[]);
        let policies = APolicyEngine::answering("");
        let scratch = home.path().join("errand");

        let step = attend_operation(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                &scratch,
            ),
            &a_signature("countersign", ""),
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("countersign no existe en PAdES: {step:?}");
        };
        assert_eq!(
            reply.on_the_wire(),
            WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire()
        );
    }

    /// **ID-264**: y `save` y `signandsave` salen por el mismo sitio, que es lo
    /// que hace comprobable que estén fuera.
    #[test]
    fn saving_by_order_of_a_site_is_answered_with_the_same_refusal() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let listed = ListedCertificates::new();
        let opened = OpenedDocuments::new();
        let engine = AnEngine::answering(&[]);
        let policies = APolicyEngine::answering("");
        let scratch = home.path().join("errand");

        for verb in ["save", "signandsave"] {
            let live = LiveErrand::default();
            let step = attend_operation(
                &a_desk(
                    &engine,
                    &policies,
                    &[],
                    home.path(),
                    &listed,
                    &opened,
                    &memory,
                    &scratch,
                ),
                &a_signature(verb, ""),
                &live,
            );

            let ErrandStep::Answering(reply) = step else {
                panic!("«{verb}» esta fuera del alcance: {step:?}");
            };
            assert_eq!(
                reply.on_the_wire(),
                WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire()
            );
            assert!(!scratch.exists(), "y no ha escrito nada");
        }
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

        let engine = AnEngine::answering(&[]);
        let policies = APolicyEngine::answering("");
        let listed = ListedCertificates::new();
        let opened = OpenedDocuments::new();
        let step = attend_operation(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                home.path(),
            ),
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

        let engine = AnEngine::answering(&[]);
        let policies = APolicyEngine::answering("");
        let listed = ListedCertificates::new();
        let opened = OpenedDocuments::new();
        let step = attend_operation(
            &a_desk(
                &engine,
                &policies,
                &[],
                home.path(),
                &listed,
                &opened,
                &memory,
                home.path(),
            ),
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
        assert!(live.begin(Errand::of(a_credential(), 54001)));

        let reply = declined(&live);

        assert_eq!(reply.on_the_wire(), "CANCEL");
        assert!(live.current().is_none(), "cancelado, el tramite se acaba");
    }

    /// **ID-280**: con un trámite vivo, el segundo `afirma://` se rechaza por su
    /// propio socket mientras el primero siga vivo.
    #[test]
    fn a_second_launch_is_refused_while_the_first_errand_is_live() {
        let live = LiveErrand::default();
        let asked = RefCell::new(Vec::new());

        let first = attend_launch(&a_launch("54001"), &a_transport(&asked), &live);
        assert!(matches!(first, Attendance::Serving { .. }), "{first:?}");

        let second = attend_launch(&a_launch("55001"), &a_transport(&asked), &live);
        let Attendance::RefusingOverTheChannel { answer, .. } = second else {
            panic!("el segundo se rechaza por su socket: {second:?}");
        };
        assert_eq!(
            answer.on_the_wire(),
            WireAnswer::refused(SafCode::CannotOpenSocket).on_the_wire()
        );

        // Y el trámite que sigue apuntado es el primero: el segundo no sustituye
        // nada ni se cuela al lado, que es lo que el ID-280 prohíbe.
        let errand = live.current().expect("el primer tramite sigue vivo");
        assert_eq!(errand.port(), 54001);
    }

    /// **ID-280, la carrera**: entre pedir el canal y apuntar el trámite cabe
    /// otra invocación —el enlace profundo y la instancia única son dos caminos
    /// distintos hasta [`attend_launch`] (#357, #362)—. Aquí la otra se lleva la
    /// plaza **mientras** el transporte abre, y la que llega tarde no se cuela
    /// al lado: se le **cierra por su asa** el canal recién abierto y sale su
    /// rechazo. Preguntando antes de apuntar, las dos quedarían servidas.
    #[test]
    fn a_launch_that_loses_the_place_while_its_channel_opens_has_it_closed_and_is_refused() {
        use std::cell::Cell;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let live = LiveErrand::default();
        let closed = Arc::new(AtomicBool::new(false));
        let opened = Cell::new(0_u8);

        // Un transporte que, la primera vez que se le pide un canal, deja que
        // otra invocación apunte su trámite antes de devolverlo.
        let transport = |ports: &[u16], _duty: ChannelDuty| {
            opened.set(opened.get() + 1);
            if opened.get() == 1 {
                assert!(live.begin(Errand::of(a_credential(), 54001)));
                let closed = Arc::clone(&closed);
                return Ok(OpenChannel::new(
                    ports[0],
                    Shutdown::of(move || closed.store(true, Ordering::SeqCst)),
                ));
            }
            Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
        };

        let attendance = attend_launch(&a_launch("55001,55002"), &transport, &live);

        let Attendance::RefusingOverTheChannel { answer, .. } = attendance else {
            panic!("la que llega tarde se rechaza por su socket: {attendance:?}");
        };
        assert_eq!(
            answer.on_the_wire(),
            WireAnswer::refused(SafCode::CannotOpenSocket).on_the_wire()
        );
        assert!(
            closed.load(Ordering::SeqCst),
            "el canal de la que llega tarde deja de escuchar: soltarlo sin llamar al asa no lo cierra"
        );

        // Y el trámite apuntado sigue siendo el de la otra, entero.
        let errand = live
            .current()
            .expect("el tramite de la otra sede sigue vivo");
        assert_eq!(errand.port(), 54001);
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

        assert!(matches!(next, Attendance::Serving { .. }), "{next:?}");
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
