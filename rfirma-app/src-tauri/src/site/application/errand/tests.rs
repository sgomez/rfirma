//! Pruebas del trámite de sede en grada A.

use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::crossing::Failure;
use crate::documents::application::documents::{self, OpenedDocuments};
use crate::documents::domain::document::Document;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from, NoMemory, NoToken};
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::CertificateMemory;
use crate::identity::ports::Token as _;
use crate::signing::adapters::failures::told_of_cycle;
use crate::signing::adapters::memory::Memory;
use crate::signing::application::cycle::CycleError;
use crate::signing::application::session::{self, CycleFailure, DocumentToSign, SigningSession};
use crate::signing::application::tests::{a_memory, NoIsolate};
use crate::signing::domain::bridge::BridgeError;
use crate::site::adapters::channel::{answer as what_the_channel_answers, Answer};
use crate::site::adapters::codec::V4Codec;
use crate::site::adapters::desk::signing_refusal_of;
use crate::site::adapters::frontier;
use crate::site::application::session::SiteRefusal;
use crate::site::application::site::{attend_launch, Attendance};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, OpenChannel, Shutdown,
};
use crate::site::domain::protocol::{
    read_operation, AfirmaUrl, ChannelCredential, ChannelMessage, NegotiatedCredential, Parameter,
    SafCode, SelectCertificate, SignRequest, SignatureRound, SiteFilter, SiteOperation,
    SiteVisibleSignature, WireAnswer, THE_PORT_OF_THE_THIRD_PROTOCOL,
};
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{Certificates, ScratchDocuments, SiteSigning, SiteSigningRequest};
use base64::Engine as _;

/// Motor de filtrado simulado para pruebas.
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

/// Un transporte que abre siempre, en el puerto sorteado o en el fijo, y apunta lo que se le pidió.
fn a_transport(
    asked: &RefCell<Vec<ChannelDuty>>,
) -> impl Fn(&ChannelLocation, ChannelDuty) -> Result<OpenChannel, ChannelError> + '_ {
    move |location: &ChannelLocation, duty: ChannelDuty| {
        asked.borrow_mut().push(duty);
        let port = match location {
            ChannelLocation::Drawn(ports) => ports[0],
            ChannelLocation::Fixed(port) => *port,
            ChannelLocation::Service(ports) => ports[0],
            ChannelLocation::Relay(_) => 0,
        };
        Ok(OpenChannel::new(port, Shutdown::of(|| {})))
    }
}

fn a_launch(ports: &str) -> String {
    format!("afirma://websocket?ports={ports}&v=4&idsession={CREDENTIAL}")
}

/// Un arranque de la version 3: sin `ports`, atendido en el puerto fijo.
fn a_v3_launch() -> String {
    format!("afirma://websocket?v=3&idsession={CREDENTIAL}")
}

/// Asa de respuesta simulada y su receptor para pruebas.
fn the_wire() -> (ReplyHandle, tokio::sync::oneshot::Receiver<String>) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    (
        ReplyHandle::of(move |text| {
            let _ = sender.send(text);
        }),
        receiver,
    )
}

/// Códec negociado para pruebas.
fn a_codec() -> NegotiatedCodec {
    Arc::new(V4Codec)
}

/// Tabla de códecs para pruebas: la version 4 sortea, la 3 usa el puerto fijo.
fn a_codec_table() -> crate::site::application::site::CodecTable {
    crate::site::application::site::CodecTable {
        v4: Arc::new(V4Codec),
        v3: Arc::new(crate::site::adapters::codec_v3::V3Codec),
        v1: Arc::new(crate::site::adapters::codec_v1::V1Codec),
        relay: Arc::new(|key| {
            Arc::new(crate::site::adapters::codec_relay::RelayCodec::new(key))
                as crate::site::application::errand::NegotiatedCodec
        }),
    }
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

/// Una sesión de firma vacía, compartida: nadie de aquí abre un ciclo.
static A_SESSION: std::sync::LazyLock<SigningSession> =
    std::sync::LazyLock::new(SigningSession::default);

/// Lo que sale al cable, si ha salido algo.
fn what_the_site_received(wire: &mut tokio::sync::oneshot::Receiver<String>) -> Option<String> {
    wire.try_recv().ok()
}

/// Petición tal y como llega por el canal.
fn arriving_over_the_channel(message: &str) -> AfirmaUrl {
    let answered = what_the_channel_answers(
        &ChannelDuty::Serve(NegotiatedCredential::Required(a_credential())),
        true,
        message,
    );
    let Answer::Pending(url) = answered else {
        panic!("una operacion legitima queda pendiente: {answered:?}");
    };
    url
}

/// Operación leída con el códec del protocolo.
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

/// Los vecinos del trámite en grada A: el token vacío, lo listado, lo abierto, la memoria y una sesión sin ciclo.
struct TheNeighbours<'a> {
    stores: Vec<Store>,
    home: &'a Path,
    listed: &'a ListedCertificates,
    opened: &'a OpenedDocuments,
    memory: &'a Memory,
}

impl Certificates for TheNeighbours<'_> {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        NoToken.list_across(&self.stores)
    }

    fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        crate::identity::application::certificates::rows_of(
            found,
            self.home,
            self.listed,
            self.memory,
        )
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        crate::identity::application::certificates::usable_certificate(found, handle, self.listed)
    }

    fn remembered(&self) -> Option<CertificateRef> {
        self.memory.remembered_certificate()
    }

    fn remember(&self, chosen: &CertificateRef) {
        crate::identity::application::certificates::remember_the_certificate(self.memory, chosen);
    }

    fn forget_the_remembered(&self) {
        crate::identity::application::certificates::forget_the_certificate(self.memory);
    }
}

impl ScratchDocuments for TheNeighbours<'_> {
    fn open_unrecorded(&self, path: std::path::PathBuf) -> String {
        self.opened.mint(Document::passing_through(path))
    }
}

impl SiteSigning for TheNeighbours<'_> {
    fn begin(&self, request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal> {
        let document = crate::documents::application::documents::opened_document(
            self.opened,
            request.document,
        )
        .map_err(CycleFailure::from)
        .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))?;
        session::begin_for_the_site(
            &crate::signing::adapters::files::RealDocumentBytes,
            DocumentToSign {
                handle: request.document.to_owned(),
                document,
            },
            request.certificate,
            session::DeclaredByTheSite {
                parameters: request.from_the_site,
                allow_unregistered_signatures: request.allow_unregistered_signatures,
            },
            &NoToken,
            &NoIsolate,
            &A_SESSION,
        )
        .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        let signed = session::finish(&NoIsolate, &A_SESSION)
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))?;
        Ok(SiteSignature {
            signed: signed.completed.into_pdf(),
            signer_der: signed.signer_der,
        })
    }
}

/// Los vecinos de una selección de certificado, que no abre ningún documento.
fn a_neighbourhood<'a>(
    home: &'a Path,
    listed: &'a ListedCertificates,
    opened: &'a OpenedDocuments,
    memory: &'a Memory,
) -> TheNeighbours<'a> {
    TheNeighbours {
        stores: Vec::new(),
        home,
        listed,
        opened,
        memory,
    }
}

/// Nadie abre nada en una selección de certificado.
fn opened_for_nobody() -> &'static OpenedDocuments {
    static NOBODY: std::sync::LazyLock<OpenedDocuments> =
        std::sync::LazyLock::new(OpenedDocuments::new);
    &NOBODY
}

/// Mesa de trabajo del trámite configurada para pruebas.
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
) -> ErrandDesk<'a, AnEngine, APolicyEngine, TheNeighbours<'a>> {
    ErrandDesk {
        engine,
        policies,
        neighbours: TheNeighbours {
            stores: stores.to_vec(),
            home,
            listed,
            opened,
            memory,
        },
        scratch_dir: scratch.to_path_buf(),
        scratch: std::sync::Arc::new(crate::site::adapters::scratch::RealScratch),
    }
}

/// Expansor de política para pruebas.
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
    ) -> Result<String, crate::signing::domain::bridge::BridgeError> {
        self.asked.borrow_mut().push(extra_params.to_owned());
        self.answer.clone().map_err(|()| {
            crate::signing::domain::bridge::BridgeError::IncompatiblePolicy(
                "no se puede aplicar".to_owned(),
            )
        })
    }
}

/// Códec simulado para pruebas.
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
        let mut answers = crate::lock(&self.answers);
        if answers.is_empty() {
            return SiteRequest::SelectCertificate(SelectCertificate::default());
        }
        answers.remove(0)
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        format!("{outcome:?}")
    }
}
#[test]
fn the_three_verbs_run_the_errand_with_a_codec_a_filter_and_a_transport_in_memory() {
    let opened = RefCell::new(Vec::new());
    let transport = |location: &ChannelLocation, duty: ChannelDuty| {
        let ChannelLocation::Drawn(ports) = location else {
            panic!("esta prueba sortea puertos: {location:?}");
        };
        opened.borrow_mut().push((ports[0], duty));
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    };
    let live = LiveErrand::default();

    let channel = Transport::open(
        &transport,
        &ChannelLocation::Drawn(vec![54001]),
        ChannelDuty::Serve(NegotiatedCredential::Required(a_credential())),
    )
    .expect("el transporte en memoria abre");
    let codec: NegotiatedCodec = Arc::new(ACodec::answering(Vec::new()));
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
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

    let (handle, mut wire) = the_wire();
    let step = attend(&desk, an_operation(""), handle, &live).expect("hay codec negociado");
    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &step else {
        panic!("sin almacen la mesa contesta en el acto: {step:?}");
    };
    let (failure, code) = frontier::told(refusal);
    assert_eq!(code, SafCode::CannotFindKeystore);
    assert_eq!(failure.situation, "moduleNotFound");
    assert!(
        what_the_site_received(&mut wire)
            .is_some_and(|line| line.starts_with("Refused(Token(TokenError")),
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

    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        channel.port(),
        codec
    )));
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

    assert!(look_again(&desk, &live).is_none());

    let refused = consent(&desk, "cualquiera", &live).expect_err("no hay nada consentible");
    assert_eq!(Failure::from(refused).situation, "siteErrandNotLive");
    assert!(what_the_site_received(&mut wire).is_none());
    assert!(live.current().is_some(), "y el tramite sigue vivo");

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
#[test]
fn what_the_codec_does_not_attend_is_answered_with_the_codec_s_own_line() {
    let live = LiveErrand::speaking(Arc::new(ACodec::answering(vec![SiteRequest::NotAttended(
        crate::site::domain::protocol::Refusal::new(SafCode::UnsupportedOperation, "eso no"),
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
/// El mismo trámite de selección de certificado, de punta a punta, sobre la forma de arranque
/// que se le pase: puertos sorteados (protocolo 4) o puerto fijo (protocolo 3).
fn a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer_over(
    launch: &str,
    expected_port: u16,
) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0], &[0]]);

    let attendance = attend_launch(launch, &a_codec_table(), &a_transport(&asked), &live);
    let Attendance::Serving { channel, .. } = &attendance else {
        panic!("la invocacion es buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        expected_port,
        "el tramite escucha en el puerto que declara el protocolo"
    );
    assert!(
        live.current().is_some(),
        "el tramite queda vivo mientras se atiende"
    );

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
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
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
        "el momento del consentimiento no escribe nada en el cable"
    );

    let reply = identity_handed_over(
        &engine,
        request.filter(),
        false,
        &ours,
        &rows[0].id,
        &crate::site::application::tests::Directory {
            certificates: ours.clone(),
            listed: &listed,
            memory: &memory,
        },
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
    let encoded = base64::engine::general_purpose::URL_SAFE.encode(ours[0].der());
    assert_eq!(on_the_wire(&reply), encoded);
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(encoded.clone()),
        "la sede recibe el certificado en el acto, por el asa del tramite"
    );
    assert!(
        live.current().is_none(),
        "contestada la sede, el tramite deja de estar vivo sin que nadie cierre nada"
    );
}
#[test]
fn a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer() {
    a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer_over(
        &a_launch("54001,54002,54003"),
        54001,
    );
}
#[test]
fn a_selection_of_a_certificate_over_the_third_protocol_goes_all_the_way_from_the_launch_to_the_answer(
) {
    a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer_over(
        &a_v3_launch(),
        THE_PORT_OF_THE_THIRD_PROTOCOL,
    );
}
#[test]
fn a_selection_that_is_declined_ends_in_a_cancel_on_the_wire_and_nothing_after_it() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);

    let attendance = attend_launch(
        &a_launch("54001,54002,54003"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
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
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
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

    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), None);
}
#[test]
fn a_connection_that_drops_while_the_operation_is_pending_does_not_take_the_errand_down() {
    let live = a_live();
    let (handle, wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        54001,
        a_codec()
    )));

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

/// Operación de firma entrando por el canal.
fn a_signature_arriving_over_the_channel(verb: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    arriving_over_the_channel(&format!(
        "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&dat={document}"
    ))
}

/// Trámite completo de firma con el canal abierto, sobre la forma de arranque que se le pase.
fn the_whole_signature_errand_over(
    launch: &str,
    expected_port: u16,
    verb: &str,
    round: SignatureRound,
) {
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

    let attendance = attend_launch(launch, &a_codec_table(), &a_transport(&asked), &live);
    let Attendance::Serving { channel, .. } = &attendance else {
        panic!("la invocacion es buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        expected_port,
        "el tramite escucha en el puerto que declara el protocolo"
    );

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
        "la politica la expandio el motor del original"
    );
    assert_eq!(
        std::fs::read(
            documents::opened_document(&opened, &consent.document)
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
        "el momento del consentimiento no escribe nada en el cable"
    );

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
        "contestada la sede, el tramite deja de estar vivo"
    );
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra al contestar"
    );
}
#[test]
fn a_signature_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand_over(
        &a_launch("54001,54002,54003"),
        54001,
        "sign",
        SignatureRound::First,
    );
}
#[test]
fn a_cosignature_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand_over(
        &a_launch("54001,54002,54003"),
        54001,
        "cosign",
        SignatureRound::Again,
    );
}
#[test]
fn a_signature_over_the_third_protocol_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand_over(
        &a_v3_launch(),
        THE_PORT_OF_THE_THIRD_PROTOCOL,
        "sign",
        SignatureRound::First,
    );
}
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

    let attendance = attend_launch(
        &a_launch("54001,54002,54003"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
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
        "el fichero de paso se borra tambien al cancelar"
    );

    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), None);
}
#[test]
fn a_signature_that_never_came_out_is_answered_with_the_code_of_a_failed_signature() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        54001,
        a_codec()
    )));

    let reply = the_signature_did_not_come_out(
        &live,
        SiteRefusal::Signing(signing_refusal_of(told_of_cycle(&CycleFailure::Cycle(
            CycleError::Bridge(BridgeError::Failed("la prefirma no ha salido".to_owned())),
        )))),
    );

    assert_eq!(
        what_the_site_received(&mut wire),
        Some("SAF_09: No se ha podido completar la firma electronica".to_owned()),
        "la sede recibe el codigo del catalogo, sin una palabra del detalle"
    );
    assert_eq!(
        reply
            .refusal()
            .map(|refusal| Failure::from(refusal).situation),
        Some("bridgeFailed".to_owned()),
        "y la ventana se queda con la situacion entera"
    );
    assert!(
        live.current().is_none(),
        "la firma que no sale cierra el tramite igual que la que sale"
    );
}
#[test]
fn a_broken_session_seal_is_answered_with_its_own_code() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        54001,
        a_codec()
    )));

    the_signature_did_not_come_out(
        &live,
        SiteRefusal::Signing(signing_refusal_of(told_of_cycle(&CycleFailure::Cycle(
            CycleError::Seal(crate::signing::domain::SealMismatch),
        )))),
    );

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::PostprocessingData).on_the_wire()),
        "el sello roto sale con SAF_42, que es el que el catalogo tiene para el"
    );
}
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
        !documents::opened_document(&opened, &consent.document)
            .expect("el documento esta en la mano")
            .is_remembered(),
        "el documento de la sede entra por la puerta que no recuerda"
    );
    assert!(
        crate::documents::application::recents::listed_rows(
            &memory,
            &crate::documents::adapters::files::RealFiles,
            &opened
        )
        .is_empty(),
        "no deja fila en Recientes"
    );
    assert_eq!(
        memory
            .state()
            .map(crate::signing::adapters::store::Loaded::into_value)
            .ok()
            .and_then(|state| state.visible_signature),
        None,
        "ni colocacion del recuadro"
    );

    let scratch_file = live.scratch_path().expect("hay fichero de paso");
    assert!(scratch_file.exists());
    declined(&live);
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra al contestar"
    );
}
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
        "la pagina contada desde el final la resuelve el puente, no rFirma"
    );
}
#[test]
fn an_optional_box_the_site_never_placed_is_signed_invisible() {
    let asked = a_consent_to_sign("visibleSignature=optional\nvisibleAppearance=custom\n");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("se firma igual, sin recuadro: {asked:?}");
    };
    assert_eq!(consent.visible, SiteVisibleSignature::Declined);
}
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
#[test]
fn an_appended_page_without_a_box_never_happens_and_the_errand_goes_on() {
    let asked = a_consent_to_sign("signaturePages=append\n");

    assert!(
        matches!(asked, ErrandStep::AskingToSign(_)),
        "sin esquinas no hay pagina que anadir: {asked:?}"
    );
}

/// Consentimiento de una firma con política expandida.
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
#[test]
fn a_pdf_with_signatures_it_cannot_read_is_asked_about_inside_the_consent() {
    let asked = a_consent_to_sign_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("no es un rechazo, es un aviso: {asked:?}");
    };
    assert!(consent.unregistered_signatures);
}
#[test]
fn an_ordinary_pdf_asks_about_no_unregistered_signature() {
    let asked = a_consent_to_sign("");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("hay certificado que la sede acepta: {asked:?}");
    };
    assert!(!consent.unregistered_signatures);
}
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
        "al puente solo se le manda tras el consentimiento"
    );
}
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
#[test]
fn a_site_that_forbids_unregistered_signatures_still_signs_an_ordinary_pdf() {
    let asked = a_consent_to_sign("allowCosigningUnregisteredSignatures=false\n");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("no hay nada que rechazar: {asked:?}");
    };
    assert!(!consent.unregistered_signatures);
}
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
#[test]
fn signing_and_saving_by_order_of_a_site_is_answered_with_the_same_refusal() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let verb = "signandsave";
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

/// Operación de guardado tal y como llega por el canal.
fn a_save(extra: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    let text = format!("afirma://save?op=save&idsession={CREDENTIAL}&dat={document}{extra}");
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Operación de carga tal y como llega por el canal.
fn a_load(extra: &str) -> AfirmaUrl {
    let text = format!("afirma://load?op=load&idsession={CREDENTIAL}{extra}");
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Una mesa sin ningun almacen: cualquier operacion que mirase certificados fallaria con
/// `CannotFindKeystore` (ver `the_three_verbs_run_the_errand...`). Guardar y cargar no la miran.
fn a_desk_without_any_store<'a>(
    engine: &'a AnEngine,
    policies: &'a APolicyEngine,
    home: &'a Path,
    listed: &'a ListedCertificates,
    opened: &'a OpenedDocuments,
    memory: &'a Memory,
    scratch: &'a Path,
) -> ErrandDesk<'a, AnEngine, APolicyEngine, TheNeighbours<'a>> {
    a_desk(engine, policies, &[], home, listed, opened, memory, scratch)
}

#[test]
fn saving_by_order_of_a_site_never_looks_at_certificates() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend_operation(&desk, &a_save(""), decoded(&a_save("")), &live);

    let ErrandStep::Saving(consent) = step else {
        panic!("«save» no mira certificados y espera al dialogo: {step:?}");
    };
    assert_eq!(consent.data, A_PDF);
}

#[test]
fn loading_by_order_of_a_site_never_looks_at_certificates() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend_operation(
        &desk,
        &a_load("&multiload=true"),
        decoded(&a_load("&multiload=true")),
        &live,
    );

    let ErrandStep::Loading(consent) = step else {
        panic!("«load» no mira certificados y espera al dialogo: {step:?}");
    };
    assert!(consent.multiple);
}

#[test]
fn the_save_moment_carries_only_the_name_the_site_proposed() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend(&desk, a_save("&filename=firma.pdf"), the_wire().0, &live)
        .expect("hay codec negociado");
    assert_eq!(
        step.moment(),
        Some(Moment::Saving {
            filename: Some("firma.pdf".to_owned())
        })
    );
    assert_eq!(
        live.the_saving_pending()
            .expect("hay guardado pendiente")
            .filename,
        Some("firma.pdf".to_owned())
    );
}

#[test]
fn the_loading_moment_never_carries_the_starting_folder() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend(
        &desk,
        a_load("&filePath=/home/persona"),
        the_wire().0,
        &live,
    )
    .expect("hay codec negociado");
    assert_eq!(step.moment(), Some(Moment::Loading));
    assert_eq!(
        live.the_loading_pending()
            .expect("hay carga pendiente")
            .starting_folder,
        Some("/home/persona".to_owned())
    );
}

#[test]
fn a_file_is_written_where_the_person_chose_and_the_site_gets_save_ok() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_save(""),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let destination = home.path().join("firma.pdf");
    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &destination,
        A_PDF,
        &live,
    );

    assert!(matches!(outcome, SiteOutcome::Saved));
    assert_eq!(std::fs::read(&destination).expect("se ha escrito"), A_PDF);
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some("SAVE_OK")
    );
}

#[test]
fn a_save_that_cannot_be_written_is_answered_with_saf_05() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_save(""),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let unwritable = home.path().join("no-existe").join("firma.pdf");
    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &unwritable,
        A_PDF,
        &live,
    );

    assert!(matches!(
        outcome,
        SiteOutcome::Refused(SiteRefusal::CannotSaveData(_))
    ));
    assert_eq!(
        on_the_wire(&outcome),
        WireAnswer::refused(SafCode::CannotSaveData).on_the_wire()
    );
    assert!(
        what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_05")),
        "sale el codigo del catalogo"
    );
}

#[test]
fn saved_writes_the_data_it_is_given_never_a_pending_consent_it_does_not_read() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let destination = home.path().join("firma.pdf");
    std::fs::write(&destination, "lo que ya habia").expect("se escribe el previo");

    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &destination,
        A_PDF,
        &live,
    );

    assert!(matches!(outcome, SiteOutcome::Saved));
    assert_eq!(
        std::fs::read(&destination).expect("se ha escrito"),
        A_PDF,
        "escribe lo que se le pasa, nunca cero bytes por falta de consentimiento pendiente"
    );
}

#[test]
fn the_files_the_person_chose_go_out_named_and_apart_with_a_bar() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let first = home.path().join("uno.pdf");
    let second = home.path().join("dos.pdf");
    std::fs::write(&first, A_PDF).expect("se escribe la primera");
    std::fs::write(&second, A_PDF).expect("se escribe la segunda");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_load("&multiload=true"),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let chosen = vec![
        ("uno.pdf".to_owned(), first),
        ("dos.pdf".to_owned(), second),
    ];
    let outcome = crate::site::application::errand::loaded(
        &crate::site::adapters::scratch::RealScratch,
        &chosen,
        &live,
    );

    let SiteOutcome::Loaded(files) = &outcome else {
        panic!("se han cargado los ficheros: {outcome:?}");
    };
    assert_eq!(files.len(), 2);
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some("uno.pdf:JVBERi0xLjcK|dos.pdf:JVBERi0xLjcK")
    );
}

#[test]
fn a_file_that_disappeared_before_it_could_be_read_is_answered_with_saf_25() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_load(""),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let missing = home.path().join("no-existe.pdf");
    let outcome = crate::site::application::errand::loaded(
        &crate::site::adapters::scratch::RealScratch,
        &[("no-existe.pdf".to_owned(), missing)],
        &live,
    );

    assert!(matches!(
        outcome,
        SiteOutcome::Refused(SiteRefusal::CannotLoadData(_))
    ));
    assert!(
        what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_25")),
        "sale el codigo del catalogo"
    );
}
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
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
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
            "{parameter} se lee en algun sitio: el consentimiento se podria saltar"
        );
    }
}
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
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
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
        reply.refusal().is_some(),
        "la ventana enseña la situacion entera"
    );
    assert_eq!(reason, NoCertificate::TheSiteExcludedThemAll);
    assert_eq!(owned, 1, "y cuantos tiene la persona, que es su almacen");
}
#[test]
fn a_refusal_of_the_protocol_never_reaches_the_token() {
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
        &an_operation("&dat=file:///etc/shadow"),
        decoded(&an_operation("&dat=file:///etc/shadow")),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("el protocolo rechaza la lectura de un fichero local: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused_because_of(SafCode::Params, Parameter::Data).on_the_wire()
    );
}
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
        WireAnswer::refused(crate::identity::adapters::failures::code_of_token(
            crate::identity::domain::error::Situation::ModuleNotFound
        ))
        .on_the_wire()
    );
}
#[test]
fn the_person_saying_no_is_the_only_cancellation() {
    let live = a_live();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        54001,
        a_codec()
    )));

    let reply = declined(&live);

    assert_eq!(on_the_wire(&reply), "CANCEL");
    assert!(live.current().is_none(), "cancelado, el tramite se acaba");
}
#[test]
fn a_second_launch_is_refused_while_the_first_errand_is_live() {
    let live = a_live();
    let asked = RefCell::new(Vec::new());

    let first = attend_launch(
        &a_launch("54001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    assert!(matches!(first, Attendance::Serving { .. }), "{first:?}");

    let second = attend_launch(
        &a_launch("55001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    let Attendance::RefusingOverTheChannel { answer, .. } = second else {
        panic!("el segundo se rechaza por su socket: {second:?}");
    };
    assert_eq!(
        answer.on_the_wire(),
        WireAnswer::refused(SafCode::CannotOpenSocket).on_the_wire()
    );

    let errand = live.current().expect("el primer tramite sigue vivo");
    assert_eq!(errand.port(), 54001);
}
#[test]
fn a_launch_that_loses_the_place_while_its_channel_opens_has_it_closed_and_is_refused() {
    use std::cell::Cell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let live = a_live();
    let closed = Arc::new(AtomicBool::new(false));
    let opened = Cell::new(0_u8);

    let transport = |location: &ChannelLocation, _duty: ChannelDuty| {
        let ChannelLocation::Drawn(ports) = location else {
            panic!("esta prueba sortea puertos: {location:?}");
        };
        opened.set(opened.get() + 1);
        if opened.get() == 1 {
            assert!(live.begin(Errand::of(
                NegotiatedCredential::Required(a_credential()),
                54001,
                a_codec()
            )));
            let closed = Arc::clone(&closed);
            return Ok(OpenChannel::new(
                ports[0],
                Shutdown::of(move || closed.store(true, Ordering::SeqCst)),
            ));
        }
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    };

    let attendance = attend_launch(
        &a_launch("55001,55002"),
        &a_codec_table(),
        &transport,
        &live,
    );

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

    let errand = live
        .current()
        .expect("el tramite de la otra sede sigue vivo");
    assert_eq!(errand.port(), 54001);
}
#[test]
fn once_the_first_site_has_its_answer_the_next_launch_is_attended() {
    let live = a_live();
    let asked = RefCell::new(Vec::new());

    attend_launch(
        &a_launch("54001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    declined(&live);

    let next = attend_launch(
        &a_launch("55001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );

    assert!(matches!(next, Attendance::Serving { .. }), "{next:?}");
}
#[test]
fn the_live_errand_remembers_the_credential_and_the_port_and_nothing_else() {
    let live = a_live();
    let asked = RefCell::new(Vec::new());

    attend_launch(
        &a_launch("54001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );

    let errand = live.current().expect("hay tramite vivo");
    assert_eq!(
        errand.credential(),
        &NegotiatedCredential::Required(a_credential())
    );
    assert_eq!(errand.port(), 54001);
}
#[test]
fn a_certificate_the_site_no_longer_accepts_is_never_handed_over() {
    let ours: Vec<TokenCertificate> = vec![a_usable_certificate("FIRMA")];
    let (listed, handles) = listed_from(&ours);
    let live = a_live();

    let reply = identity_handed_over(
        &AnEngine::answering(&[&[]]),
        &SiteFilter::default(),
        false,
        &ours,
        &handles[0],
        &crate::site::application::tests::Directory {
            certificates: ours.clone(),
            listed: &listed,
            memory: &NoMemory,
        },
        &live,
    );

    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
    assert!(
        reply
            .refusal()
            .is_some_and(|it| Failure::from(it).situation == "certificateNotFound"),
        "la ventana sabe cual es la situacion: {reply:?}"
    );
}
#[test]
fn with_no_certificate_at_all_nothing_goes_out_and_the_errand_stays_live() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let engine = AnEngine::answering(&[]);

    let live = a_live();
    assert!(
        live.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            54001,
            a_codec()
        )),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));

    live.keep_the_request(url.clone());
    let step = consent_for(
        &engine,
        &requested(&url),
        Vec::new(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
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
        live.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            54001,
            a_codec()
        )),
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

    let inadmissible = a_live();
    assert!(
        inadmissible.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            54002,
            a_codec()
        )),
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
#[test]
fn leaving_the_no_certificate_screen_cancels_the_errand() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let engine = AnEngine::answering(&[]);

    let live = a_live();
    assert!(
        live.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            54001,
            a_codec()
        )),
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
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    declined(&live);

    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some(crate::site::domain::protocol::CANCELLED),
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
#[test]
fn a_sticky_selection_answers_with_the_remembered_certificate_without_asking() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

    let engine = AnEngine::answering(&[&[0]]);
    let request = requested(&an_operation("&sticky=true"));
    let step = consent_for(
        &engine,
        &request,
        ours.clone(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    assert!(
        step.moment().is_none(),
        "el certificado pegado no abre ningun momento: {step:?}"
    );
    let ErrandStep::Answering(reply) = &step else {
        panic!("la sede recibe el certificado en el acto: {step:?}");
    };
    assert_eq!(
        on_the_wire(reply),
        base64::engine::general_purpose::URL_SAFE.encode(ours[0].der())
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(base64::engine::general_purpose::URL_SAFE.encode(ours[0].der()))
    );
}
#[test]
fn a_sticky_selection_asks_when_the_remembered_one_is_outside_the_filter() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA"), a_usable_certificate("OTRO")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[1].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();

    let engine = AnEngine::answering(&[&[0], &[0]]);
    let request = requested(&an_operation("&sticky=true"));
    let neighbours = a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory);
    let step = consent_for(&engine, &request, ours.clone(), &neighbours, &live);

    let ErrandStep::AskingForConsent {
        certificates: rows,
        sticky,
        ..
    } = step
    else {
        panic!("el recordado no cruza el filtro: se pregunta");
    };
    assert!(sticky, "la sede pego el certificado que se elija");

    let reply = identity_handed_over(
        &engine,
        request.filter(),
        sticky,
        &ours,
        &rows[0].id,
        &neighbours,
        &live,
    );
    assert!(matches!(reply, SiteOutcome::Certificate(_)));
    assert!(
        memory
            .remembered_certificate()
            .is_some_and(|one| one.is_the_same_as(ours[0].reference())),
        "el elegido queda recordado"
    );
}
#[test]
fn resetsticky_forgets_the_remembered_one_and_asks_all_the_same() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();

    let engine = AnEngine::answering(&[&[0]]);
    let request = requested(&an_operation("&sticky=true&resetsticky=true"));
    let step = consent_for(
        &engine,
        &request,
        ours.clone(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    assert!(
        matches!(step, ErrandStep::AskingForConsent { .. }),
        "olvidado el recordado, se pregunta: {step:?}"
    );
    assert_eq!(
        memory.remembered_certificate(),
        None,
        "'resetsticky' olvida antes de resolver"
    );
}
#[test]
fn without_sticky_the_remembered_certificate_changes_nothing() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();

    let engine = AnEngine::answering(&[&[0]]);
    let request = requested(&an_operation(""));
    let neighbours = a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory);
    let step = consent_for(&engine, &request, ours.clone(), &neighbours, &live);

    let ErrandStep::AskingForConsent {
        certificates: rows,
        sticky,
        ..
    } = step
    else {
        panic!("sin 'sticky' se pregunta siempre");
    };
    assert!(!sticky, "sin 'sticky' no hay nada que recordar");
    assert!(
        memory.remembered_certificate().is_some(),
        "el recordado sigue donde estaba"
    );

    let reply = identity_handed_over(
        &AnEngine::answering(&[&[0]]),
        request.filter(),
        sticky,
        &ours,
        &rows[0].id,
        &neighbours,
        &live,
    );
    assert!(matches!(reply, SiteOutcome::Certificate(_)));
}
