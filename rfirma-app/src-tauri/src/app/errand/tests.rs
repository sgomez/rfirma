//! **Las pruebas del trámite**, en grada A: ni token, ni librería nativa, ni
//! socket (TD-51, TD-52). El transporte es un cierre, el códec el de la
//! versión 4 —o uno en memoria—, y los dos motores del puente, dobles.
//!
//! Van en un fichero aparte y no al pie de cada módulo porque prueban el
//! trámite entero, del arranque a la línea que sale, y eso cruza los cinco
//! ficheros del módulo.

use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::app::codec::V4Codec;
use crate::app::fixtures::{a_memory, a_usable_certificate, listed_from};
use crate::app::frontier;
use crate::app::in_hand::DocumentInHand;
use crate::app::signing::{SigningSession, SiteRefusal, SiteSignature};
use crate::app::site::{attend_launch, Attendance};
use crate::channel::{
    answer as what_the_channel_answers, Answer, ChannelDuty, ChannelError, OpenChannel, Shutdown,
};
use crate::ffi::BridgeError;
use crate::isolate::Isolate;
use crate::memory::{ListedCertificates, Memory, OpenedDocuments};
use crate::pkcs11::{Store, TokenCertificate};
use crate::protocol::{
    read_operation, AfirmaUrl, ChannelCredential, ChannelMessage, SafCode, SelectCertificate,
    SignRequest, SignatureRound, SiteFilter, SiteOperation, SiteVisibleSignature, WireAnswer,
};
use base64::Engine as _;

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
    fn select(&self, _properties: &str, _certificates: &str) -> Result<Vec<usize>, BridgeError> {
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

/// **El cable, sin socket delante** (TD-52): el asa que el servidor le da
/// al trámite y el otro extremo, por donde la prueba lee lo que sale.
fn the_wire() -> (ReplyHandle, tokio::sync::oneshot::Receiver<String>) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    (
        ReplyHandle::of(move |text| {
            let _ = sender.send(text);
        }),
        receiver,
    )
}

/// El códec negociado en todas estas pruebas: el de la versión 4, que es el
/// único que hay.
fn a_codec() -> NegotiatedCodec {
    Arc::new(V4Codec)
}

/// Un trámite que ya habla la versión 4, sin haber empezado todavía.
fn a_live() -> LiveErrand {
    LiveErrand::speaking(a_codec())
}

/// La operación leída por el códec, que es como le llega a la mesa.
fn decoded(url: &AfirmaUrl) -> SiteRequest {
    V4Codec.decode(url)
}

/// La línea que el códec escribe para ese desenlace.
fn on_the_wire(outcome: &SiteOutcome) -> String {
    V4Codec.encode(outcome)
}

/// Un aislado cuyo puente no abre: ninguna prueba de aquí cruza la frontera,
/// y la mesa lo pide igual porque consentir una firma lo necesita.
static AN_ISOLATE: std::sync::LazyLock<Isolate> = std::sync::LazyLock::new(|| {
    Isolate::start_with(|| Err(BridgeError::Failed("no hay libreria en grada A".to_owned())))
});

/// Una sesión de firma vacía, compartida: nadie de aquí abre un ciclo.
static A_SESSION: std::sync::LazyLock<SigningSession> =
    std::sync::LazyLock::new(SigningSession::default);

/// Lo que sale al cable, si ha salido algo.
fn what_the_site_received(wire: &mut tokio::sync::oneshot::Receiver<String>) -> Option<String> {
    wire.try_recv().ok()
}

/// La operación tal y como entra de verdad: por el canal, con las tres
/// guardias delante, y dejando la respuesta para el trámite (ID-320).
fn arriving_over_the_channel(message: &str) -> AfirmaUrl {
    let answered = what_the_channel_answers(&ChannelDuty::Serve(a_credential()), true, message);
    let Answer::Pending(url) = answered else {
        panic!("una operacion legitima queda pendiente: {answered:?}");
    };
    url
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
        stores: stores.to_vec(),
        installed_dir: home,
        listed,
        opened,
        memory,
        scratch_dir: scratch.to_path_buf(),
        isolate: &AN_ISOLATE,
        session: &A_SESSION,
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
    fn expand(&self, extra_params: &str, _format: &str) -> Result<String, crate::ffi::BridgeError> {
        self.asked.borrow_mut().push(extra_params.to_owned());
        self.answer.clone().map_err(|()| {
            crate::ffi::BridgeError::IncompatiblePolicy("no se puede aplicar".to_owned())
        })
    }
}

/// **Un códec en memoria** (RT-01): no lee la URL —contesta lo que se le
/// programó— y escribe cada desenlace tal cual se imprime, para que la prueba
/// vea que lo que sale al cable lo decide el códec y no el trámite.
struct ACodec {
    answers: std::sync::Mutex<Vec<SiteRequest>>,
}

impl ACodec {
    fn answering(requests: Vec<SiteRequest>) -> Self {
        Self {
            answers: std::sync::Mutex::new(requests),
        }
    }
}

impl ProtocolCodec for ACodec {
    fn decode(&self, _message: &AfirmaUrl) -> SiteRequest {
        let mut answers = crate::app::lock(&self.answers);
        if answers.is_empty() {
            return SiteRequest::SelectCertificate(SelectCertificate::default());
        }
        answers.remove(0)
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        format!("{outcome:?}")
    }
}

/// **RT-01**: los tres verbos, con un códec, un motor de filtros y un
/// transporte en memoria, mirando el `SiteOutcome` y el estado del trámite y
/// nada más: ni el JSON del cable ni el `State` de Tauri.
///
/// En grada A no hay ningún almacén que listar, así que atender acaba en lo
/// que la mesa contesta cuando no encuentra el almacén: sale por el asa con
/// la palabra del códec y el trámite termina. Con la plaza libre otra vez,
/// volver a mirar no tiene petición, consentir no tiene nada delante, y
/// declinar es lo único que escribe: la palabra del códec, y el trámite acaba
/// sin nada que consentir.
#[test]
fn the_three_verbs_run_the_errand_with_a_codec_a_filter_and_a_transport_in_memory() {
    let opened = RefCell::new(Vec::new());
    let transport = |ports: &[u16], duty: ChannelDuty| {
        opened.borrow_mut().push((ports[0], duty));
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    };
    let live = LiveErrand::default();

    // El transporte en memoria abre el canal por el puerto, y la plaza se
    // pide con el códec en memoria como negociado.
    let channel = Transport::open(&transport, &[54001], ChannelDuty::Serve(a_credential()))
        .expect("el transporte en memoria abre");
    let codec: NegotiatedCodec = Arc::new(ACodec::answering(Vec::new()));
    assert!(live.begin(Errand::of(
        a_credential(),
        channel.port(),
        Arc::clone(&codec)
    )));
    assert_eq!(opened.borrow().len(), 1, "un canal, y por el puerto pedido");

    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened_documents = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened_documents,
        &memory,
        &scratch,
    );

    // Verbo 1: atender. Sin almacen que listar, la mesa contesta en el acto.
    let (handle, mut wire) = the_wire();
    let step = attend(&desk, an_operation(""), handle, &live).expect("hay codec negociado");
    let ErrandStep::Answering(SiteOutcome::Refused { answer, failure }) = &step else {
        panic!("sin almacen la mesa contesta en el acto: {step:?}");
    };
    assert_eq!(*answer, WireAnswer::refused(SafCode::CannotFindKeystore));
    assert_eq!(failure.situation, "moduleNotFound");
    assert!(
        what_the_site_received(&mut wire).is_some_and(
            |line| line.starts_with("Refused { answer: Refused { code: CannotFindKeystore")
        ),
        "lo que sale lo escribe el codec negociado, no el tramite"
    );
    assert!(
        step.moment().is_none(),
        "lo contestado no es un momento nuevo"
    );
    assert!(live.current().is_none(), "y el tramite ha terminado");
    assert!(
        live.the_request().is_none(),
        "sin peticion que volver a atender"
    );
    assert!(
        live.codec().is_some(),
        "el codec negociado sobrevive al tramite: el canal sigue en pie"
    );

    // La plaza vuelve a estar libre: la siguiente sede la pide con el mismo
    // codec, y esta vez la persona dice que no antes de que llegue nada.
    assert!(live.begin(Errand::of(a_credential(), channel.port(), codec)));
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

    // Verbo 1, otra vez: sin peticion apuntada no hay nada que volver a mirar.
    assert!(look_again(&desk, &live).is_none());

    // Verbo 2 sin nada delante que consentir: se rechaza sin tocar el cable.
    let refused = consent(&desk, "cualquiera", &live).expect_err("no hay nada consentible");
    assert_eq!(refused.situation, "siteErrandNotLive");
    assert!(what_the_site_received(&mut wire).is_none());
    assert!(live.current().is_some(), "y el tramite sigue vivo");

    // Verbo 3: declinar escribe la palabra del codec, y se acaba.
    let outcome = decline(&live);
    assert!(matches!(outcome, SiteOutcome::Cancelled));
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some("Cancelled"),
        "lo que sale lo escribe el codec negociado, no el tramite"
    );
    assert!(live.current().is_none(), "el tramite ha terminado");
    assert!(
        consent(&desk, "cualquiera", &live).is_err(),
        "y no queda nada que consentir"
    );
}

/// **RT-01**, la otra mitad: lo que el códec dice que no se atiende sale al
/// cable con la palabra del códec, y el trámite acaba sin dejar momento nuevo.
#[test]
fn what_the_codec_does_not_attend_is_answered_with_the_codec_s_own_line() {
    let live = LiveErrand::speaking(Arc::new(ACodec::answering(vec![SiteRequest::NotAttended(
        crate::protocol::Refusal::new(SafCode::UnsupportedOperation, "eso no"),
    )])));
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );

    let (handle, mut wire) = the_wire();
    let step = attend(&desk, an_operation(""), handle, &live).expect("hay codec");

    assert!(matches!(
        step,
        ErrandStep::Answering(SiteOutcome::RefusedByTheProtocol(_))
    ));
    assert!(
        step.moment().is_none(),
        "lo contestado no es un momento nuevo"
    );
    assert!(live.moment().is_none());
    assert!(
        what_the_site_received(&mut wire)
            .is_some_and(|line| line.starts_with("RefusedByTheProtocol(")),
        "la linea la escribe el codec"
    );
    assert!(
        live.the_request().is_none(),
        "lo que no se atiende no se apunta"
    );
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
    let live = a_live();
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

    // 2. Por ese canal llega la operación. La conversación la deja
    //    pendiente —no escribe nada y no cierra (ID-320)— y el trámite se
    //    queda con el asa por la que se le contestará (ID-321).
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
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
    let ErrandStep::AskingForConsent {
        certificates: rows, ..
    } = step
    else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(rows.len(), 1);
    assert!(
        live.current().is_some(),
        "consintiendo, el tramite sigue vivo"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el momento del consentimiento no escribe nada en el cable (ID-275)"
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
    let SiteOutcome::Certificate(der) = &reply else {
        panic!("la persona se ha identificado: {reply:?}");
    };
    assert_eq!(
        der,
        ours[0].der(),
        "lo que la persona entrego es su DER, sin envolver"
    );
    // Y el códec lo escribe en Base64 URL-safe y nada más.
    let encoded = base64::engine::general_purpose::URL_SAFE.encode(ours[0].der());
    assert_eq!(on_the_wire(&reply), encoded);
    // Y **el texto exacto que sale al cable** es ése y nada más (TD-72).
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(encoded.clone()),
        "la sede recibe el certificado en el acto, por el asa del tramite"
    );
    assert!(
        live.current().is_none(),
        "contestada la sede, el tramite deja de estar vivo sin que nadie cierre nada (ID-275)"
    );
}

/// **La gemela del trazador** (TD-72): la persona dice que no, y lo que sale
/// al cable es `CANCEL` en el acto. Después de contestar, ni cancelar otra
/// vez ni cerrar la ventana escriben nada (ID-340).
#[test]
fn a_selection_that_is_declined_ends_in_a_cancel_on_the_wire_and_nothing_after_it() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);

    let attendance = attend_launch(&a_launch("54001,54002,54003"), &a_transport(&asked), &live);
    assert!(
        matches!(attendance, Attendance::Serving { .. }),
        "la invocacion es buena: {attendance:?}"
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
    let step = consent_for(
        &engine,
        &requested(&url),
        ours,
        home.path(),
        &listed,
        &memory,
        &live,
    );
    assert!(
        matches!(step, ErrandStep::AskingForConsent { .. }),
        "hay algo que consentir: {step:?}"
    );
    assert_eq!(what_the_site_received(&mut wire), None);

    let reply = declined(&live);

    assert!(matches!(reply, SiteOutcome::Cancelled), "{reply:?}");
    assert_eq!(
        what_the_site_received(&mut wire),
        Some("CANCEL".to_owned()),
        "cancelar sale al cable en el acto, sin esperar a que nadie cierre nada"
    );
    assert!(live.current().is_none());

    // Y ya no queda asa: cerrar la ventana después de haber contestado —que
    // es lo mismo que cancelar (ID-340)— no manda nada.
    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), None);
}

/// **ID-323.** Una conexión que se cae con la operación pendiente no tumba
/// el trámite: el desenlace se enseña igual y no se reintenta.
#[test]
fn a_connection_that_drops_while_the_operation_is_pending_does_not_take_the_errand_down() {
    let live = a_live();
    let (handle, wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(a_credential(), 54001, a_codec())));

    // La sede se fue: al otro extremo del asa ya no hay nadie.
    drop(wire);

    let reply = declined(&live);

    assert!(
        matches!(reply, SiteOutcome::Cancelled),
        "el desenlace es el mismo, lo lea alguien o no: {reply:?}"
    );
    assert!(
        live.current().is_none(),
        "y el tramite termina igual, sin reintentar nada"
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

/// La misma operación de firma, entrando **por el canal**: con las tres
/// guardias de la conversación delante y quedando pendiente de que el
/// trámite conteste (ID-320).
fn a_signature_arriving_over_the_channel(verb: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    arriving_over_the_channel(&format!(
        "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&dat={document}"
    ))
}

/// **El trazador entero de la firma** (TD-51, TD-72): invocación,
/// operación llegada por el canal, política expandida, listado filtrado,
/// consentimiento, y **el texto exacto que sale al cable** cuando la firma
/// termina, sin abrir un socket y sin token (TD-52).
///
/// La firma de verdad —prefirma, PIN y postfirma— es la grada C; lo que
/// esta prueba fija es la decisión que hay a cada lado de ella, y que la
/// sede recibe exactamente lo que parte `processSignResponse`: el
/// certificado, `|`, la firma, y **ningún tercer campo**.
///
/// Se recorre igual para `sign` y para `cosign` porque en PAdES cofirmar es
/// volver a firmar: lo único que cambia es lo que se le cuenta a la persona
/// antes de que consienta.
fn the_whole_signature_errand(verb: &str, round: SignatureRound) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n");
    let scratch = home.path().join("errand");

    // 1. La sede invoca, y el canal queda sirviendo su conversación.
    let attendance = attend_launch(&a_launch("54001,54002,54003"), &a_transport(&asked), &live);
    assert!(
        matches!(attendance, Attendance::Serving { .. }),
        "la invocacion es buena: {attendance:?}"
    );

    // 2. Por ese canal llega la firma. La conversación la deja pendiente
    //    —no escribe nada y no cierra (ID-320)— y el trámite se queda con
    //    el asa por la que se le contestará (ID-321).
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = a_signature_arriving_over_the_channel(verb);
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
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(consent.round, round, "la ronda es la que pidio la sede");
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
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el momento del consentimiento no escribe nada en el cable (ID-275)"
    );

    // 3. La persona consiente y teclea el PIN. La firma termina y la sede
    //    recibe certificado y firma, en ese orden y separados por `|`.
    let scratch_file = live
        .scratch_path()
        .expect("el fichero de paso queda apuntado en el tramite");
    let reply = signature_handed_over(
        &live,
        &SiteSignature {
            signed: b"%PDF-1.7 firmado".to_vec(),
            signer_der: ours[0].der().to_vec(),
        },
    );
    assert!(
        matches!(reply, SiteOutcome::Signature { .. }),
        "la firma ha terminado: {reply:?}"
    );

    let encode = base64::engine::general_purpose::URL_SAFE;
    let line = what_the_site_received(&mut wire)
        .expect("la sede recibe la firma en el acto, por el asa del tramite");
    assert_eq!(
        line,
        format!(
            "{}|{}",
            encode.encode(ours[0].der()),
            encode.encode(b"%PDF-1.7 firmado")
        ),
        "el texto exacto del cable es el certificado y la firma, en Base64 URL-safe"
    );
    assert_eq!(
        line.split('|').count(),
        2,
        "el cliente publicado parte por `|` y no espera ningun tercer campo"
    );
    assert!(
        live.current().is_none(),
        "contestada la sede, el tramite deja de estar vivo (ID-275)"
    );
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra al contestar (ID-286, ID-332)"
    );
}

/// **TD-72**, con `sign`: la sede manda un documento y recibe su firma.
#[test]
fn a_signature_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand("sign", SignatureRound::First);
}

/// **TD-72**, con `cosign`: el mismo recorrido, y lo único que cambia es la
/// ronda que se le cuenta a la persona.
#[test]
fn a_cosignature_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand("cosign", SignatureRound::Again);
}

/// **La gemela del trazador** (TD-72): la persona dice que no a la firma, y
/// lo que sale al cable es `CANCEL` en el acto. Del documento que mandó la
/// sede no queda tampoco el fichero de paso: se borra por la salida que sea
/// (ID-332).
#[test]
fn a_signature_that_is_declined_ends_in_a_cancel_and_leaves_no_scratch_behind() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let attendance = attend_launch(&a_launch("54001,54002,54003"), &a_transport(&asked), &live);
    assert!(
        matches!(attendance, Attendance::Serving { .. }),
        "la invocacion es buena: {attendance:?}"
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = a_signature_arriving_over_the_channel("sign");
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
        ours,
        &live,
    );
    assert!(
        matches!(step, ErrandStep::AskingToSign(_)),
        "hay algo que consentir: {step:?}"
    );
    assert_eq!(what_the_site_received(&mut wire), None);
    let scratch_file = live
        .scratch_path()
        .expect("el fichero de paso queda apuntado en el tramite");

    let reply = declined(&live);

    assert!(matches!(reply, SiteOutcome::Cancelled), "{reply:?}");
    assert_eq!(
        what_the_site_received(&mut wire),
        Some("CANCEL".to_owned()),
        "cancelar sale al cable en el acto, sin esperar a que nadie cierre nada"
    );
    assert!(live.current().is_none());
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra tambien al cancelar (ID-332)"
    );

    // Y ya no queda asa: cerrar la ventana después de haber contestado no
    // manda nada (ID-340).
    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), None);
}

/// **ID-322 / ID-291**: la firma que no sale se le cuenta a la sede con un
/// código del catálogo y a la ventana con la situación entera, que es la
/// que sí puede ser precisa.
#[test]
fn a_signature_that_never_came_out_is_answered_with_the_code_of_a_failed_signature() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(a_credential(), 54001, a_codec())));

    let reply = the_signature_did_not_come_out(
        &live,
        SiteRefusal::new(
            SafCode::SignatureFailed,
            Failure::new("bridgeFailed", "la prefirma no ha salido"),
        ),
    );

    assert_eq!(
        what_the_site_received(&mut wire),
        Some("SAF_09: No se ha podido completar la firma electronica".to_owned()),
        "la sede recibe el codigo del catalogo, sin una palabra del detalle (ID-291)"
    );
    assert_eq!(
        reply.failure().map(|failure| failure.situation.as_str()),
        Some("bridgeFailed"),
        "y la ventana se queda con la situacion entera (ID-29)"
    );
    assert!(
        live.current().is_none(),
        "la firma que no sale cierra el tramite igual que la que sale"
    );
}

/// **ID-292**: y el código lo manda la situación, no el sitio donde se
/// falla. El sello del ADR-0016 que no cuadra es `SAF_42`, no el `SAF_09`
/// de la firma que no sale: la sede tiene que poder distinguir «no se ha
/// podido firmar» de «se firmó y no se pudo ensamblar».
#[test]
fn a_broken_session_seal_is_answered_with_its_own_code() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(a_credential(), 54001, a_codec())));

    the_signature_did_not_come_out(
        &live,
        SiteRefusal::new(
            frontier::code_of_broken_seal(),
            Failure::new("sealMismatch", "el sello de sesion no cuadra"),
        ),
    );

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::PostprocessingData).on_the_wire()),
        "el sello roto sale con SAF_42, que es el que el catalogo tiene para el"
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
    let live = a_live();
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
        on_the_wire(&reply).starts_with("SAF_43"),
        "lo que sale es el codigo de la firma visible: {}",
        on_the_wire(&reply)
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
        on_the_wire(&reply).starts_with("SAF_03"),
        "lo que sale es el rechazo del parametro: {}",
        on_the_wire(&reply)
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
    let live = a_live();
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
    assert_eq!(on_the_wire(&reply), "CANCEL");
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
    let live = a_live();
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
        on_the_wire(&reply),
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
    let live = a_live();
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
        on_the_wire(&reply),
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
    let live = a_live();
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
        decoded(&a_signature("countersign", "")),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("countersign no existe en PAdES: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
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
        let live = a_live();
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
            decoded(&a_signature(verb, "")),
            &live,
        );

        let ErrandStep::Answering(reply) = step else {
            panic!("«{verb}» esta fuera del alcance: {step:?}");
        };
        assert_eq!(
            on_the_wire(&reply),
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
    let live = a_live();
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

    let ErrandStep::AskingForConsent {
        certificates: rows, ..
    } = step
    else {
        panic!("el consentimiento no se salta nunca: {step:?}");
    };
    assert_eq!(rows.len(), 1, "uno solo se consiente igual");
}

/// Los dos parámetros **ni se leen**: la prueba de arriba mira la conducta,
/// y ésta mira que no exista el camino que la cambiaría.
#[test]
fn the_two_parameters_of_the_silent_signature_are_not_read_anywhere() {
    let production = concat!(
        include_str!("mod.rs"),
        include_str!("desk.rs"),
        include_str!("replies.rs"),
        include_str!("state.rs")
    );

    for parameter in ["\"headless\"", "\"mandatoryCertSelection\""] {
        assert!(
            !production.contains(parameter),
            "{parameter} se lee en algun sitio: el consentimiento se podria saltar (ID-272)"
        );
    }
}

/// **ID-258 / ID-278**: si la sede los excluye a todos, lo que recibe es
/// `SAF_19`, y sale ya. La ventana enseña la otra mitad: cuál de las dos
/// situaciones es y cuántos certificados tiene la persona (ID-341).
#[test]
fn a_site_that_excludes_them_all_gets_the_code_of_an_empty_keystore() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
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

    let ErrandStep::NoCertificate {
        reason,
        owned,
        answered: Some(reply),
    } = step
    else {
        panic!("no hay nada que consentir: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
    assert!(
        reply.failure().is_some(),
        "la ventana enseña la situacion entera (ID-275)"
    );
    assert_eq!(reason, NoCertificate::TheSiteExcludedThemAll);
    assert_eq!(owned, 1, "y cuantos tiene la persona, que es su almacen");
}

/// Un rechazo del protocolo —un criterio fuera de la lista blanca— sale con
/// su código **sin tocar el token**.
#[test]
fn a_refusal_of_the_protocol_never_reaches_the_token() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let live = a_live();
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
        decoded(&an_operation(&format!("&properties={properties}"))),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("el criterio no esta en la lista blanca: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::Params).on_the_wire()
    );
}

/// Sin ningún almacén donde mirar, la sede recibe el código de la situación
/// del token, traducido por la frontera y no por este módulo (ID-288).
#[test]
fn a_token_that_cannot_be_listed_answers_with_the_code_of_its_own_situation() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let live = a_live();

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
        decoded(&an_operation("")),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("no hay almacenes: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
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
    let live = a_live();
    assert!(live.begin(Errand::of(a_credential(), 54001, a_codec())));

    let reply = declined(&live);

    assert_eq!(on_the_wire(&reply), "CANCEL");
    assert!(live.current().is_none(), "cancelado, el tramite se acaba");
}

/// **ID-280**: con un trámite vivo, el segundo `afirma://` se rechaza por su
/// propio socket mientras el primero siga vivo.
#[test]
fn a_second_launch_is_refused_while_the_first_errand_is_live() {
    let live = a_live();
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

    let live = a_live();
    let closed = Arc::new(AtomicBool::new(false));
    let opened = Cell::new(0_u8);

    // Un transporte que, la primera vez que se le pide un canal, deja que
    // otra invocación apunte su trámite antes de devolverlo.
    let transport = |ports: &[u16], _duty: ChannelDuty| {
        opened.set(opened.get() + 1);
        if opened.get() == 1 {
            assert!(live.begin(Errand::of(a_credential(), 54001, a_codec())));
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
    let live = a_live();
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
    let live = a_live();
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
    let live = a_live();

    let reply = identity_handed_over(
        &AnEngine::answering(&[&[]]),
        &SiteFilter::default(),
        &ours,
        &handles[0],
        &listed,
        &live,
    );

    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
    assert!(
        reply
            .failure()
            .is_some_and(|it| it.situation == "certificateNotFound"),
        "la ventana sabe cual es la situacion: {reply:?}"
    );
}

/// **ID-278, ID-341.** Sin ni un certificado instalado **no sale nada al
/// cable** y el trámite sigue vivo: la ventana lo enseña con su motivo, y
/// la petición queda apuntada para poder volver a atenderla —instalar uno y
/// volver a mirar— sin que la sede invoque otra vez.
#[test]
fn with_no_certificate_at_all_nothing_goes_out_and_the_errand_stays_live() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let engine = AnEngine::answering(&[]);

    let live = a_live();
    assert!(
        live.begin(Errand::of(a_credential(), 54001, a_codec())),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));

    // El listado vacío es el de la persona que no tiene ninguno. Se le
    // habla al caso de uso que **no** lista el token, que es donde vive la
    // decisión y donde se prueba entera (TD-20, TD-51).
    live.keep_the_request(url.clone());
    let step = consent_for(
        &engine,
        &requested(&url),
        Vec::new(),
        home.path(),
        &listed,
        &memory,
        &live,
    );

    assert!(
        matches!(
            step,
            ErrandStep::NoCertificate {
                reason: NoCertificate::NotOne,
                owned: 0,
                answered: None,
            }
        ),
        "la ventana lo enseña con su motivo: {step:?}"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "a la sede no se le ha dicho nada todavia"
    );
    assert!(
        live.current().is_some(),
        "y el tramite sigue vivo: instalar uno todavia lo arregla"
    );
    assert_eq!(
        live.the_request().as_ref(),
        Some(&url),
        "con la peticion apuntada, volver a mirar no reinicia nada"
    );
}

/// **ID-278, ID-341.** Y por el camino de la firma, lo mismo: la decisión
/// vive en `accepted_listing`, que es lo que `consent_to_sign` comparte
/// con [`consent_for`], y llega **después** de la admisibilidad del
/// documento y de la política —a tiempo, antes de escribir el fichero de
/// paso—. Ese orden es lo que se clava aquí, y lo clava el par: con un PDF
/// bueno sale la pantalla del almacén vacío, y con algo que no es un PDF
/// sale el `SAF_43` aunque tampoco haya ni un certificado.
#[test]
fn on_the_signing_path_an_empty_keystore_stops_before_anything_is_written() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let live = a_live();
    assert!(
        live.begin(Errand::of(a_credential(), 54001, a_codec())),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

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
        Vec::new(),
        &live,
    );

    assert!(
        matches!(
            step,
            ErrandStep::NoCertificate {
                reason: NoCertificate::NotOne,
                owned: 0,
                answered: None,
            }
        ),
        "sin ni un certificado la ventana lo enseña con su motivo: {step:?}"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "a la sede no se le ha dicho nada todavia"
    );
    assert!(
        live.current().is_some(),
        "y el tramite sigue vivo: instalar uno todavia lo arregla"
    );
    assert!(
        !scratch.exists(),
        "y no se ha escrito el fichero de paso: la decision llega antes"
    );

    // La otra mitad del orden: lo que el documento no admisible despacha,
    // lo despacha **antes** de mirar el almacén.
    let inadmissible = a_live();
    assert!(
        inadmissible.begin(Errand::of(a_credential(), 54002, a_codec())),
        "la plaza es suya"
    );
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
        &signature_requested(&a_signature_over(b"esto no es un PDF", "sign", "")),
        Vec::new(),
        &inadmissible,
    );
    let ErrandStep::Answering(reply) = step else {
        panic!("la admisibilidad va primero, y eso no es un PDF: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::InvalidPdf).on_the_wire(),
        "el almacen vacio no le roba el turno a la admisibilidad"
    );
}

/// Y decir que no desde esa pantalla es decir que no al trámite: la sede
/// recibe su `CANCEL` en el acto (ID-340), y con él se va la petición
/// apuntada.
#[test]
fn leaving_the_no_certificate_screen_cancels_the_errand() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let engine = AnEngine::answering(&[]);

    let live = a_live();
    assert!(
        live.begin(Errand::of(a_credential(), 54001, a_codec())),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
    live.keep_the_request(url.clone());
    consent_for(
        &engine,
        &requested(&url),
        Vec::new(),
        home.path(),
        &listed,
        &memory,
        &live,
    );

    declined(&live);

    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some(crate::protocol::CANCELLED),
        "la sede recibe su CANCEL"
    );
    assert!(
        live.the_request().is_none(),
        "y no queda nada que reatender"
    );
}

fn a_credential() -> ChannelCredential {
    ChannelCredential::parse(CREDENTIAL).expect("es una credencial buena")
}
