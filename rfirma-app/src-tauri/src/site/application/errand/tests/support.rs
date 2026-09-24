//! Los dobles y ayudantes en grada A compartidos por las pruebas del tramite.

use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

use crate::documents::application::documents::OpenedDocuments;
use crate::documents::domain::document::Document;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::NoToken;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token as _;
use crate::signing::adapters::failures::told_of_cycle;
use crate::signing::adapters::memory::Memory;
use crate::signing::application::session::{self, CycleFailure, DocumentToSign, SigningSession};
use crate::signing::application::tests::{ABridgeThatSigns, AnIsolateWith, NoIsolate};
use crate::signing::domain::bridge::{BridgeError, Format, SignatureOperation};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::ports::{Bridge, IsolateHost, Signer};
use crate::site::adapters::channel::{answer as what_the_channel_answers, Answer};
use crate::site::adapters::codec::V4Codec;
use crate::site::adapters::desk::signing_refusal_of;
use crate::site::application::errand::*;
use crate::site::application::tests::read_operation;
use crate::site::application::tests::{InMemoryBatchServices, InMemoryTokenSigning, NotAsked};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, OpenChannel, Shutdown,
};
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelCredential, ChannelMessage, NegotiatedCredential, SelectCertificate,
    SiteOperation,
};
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{
    Certificates, ScratchDocuments, SiteSigning, SiteSigningRequest, TokenSigning,
};

/// Motor de filtrado simulado para pruebas.
pub(crate) struct AnEngine {
    answers: RefCell<Vec<Vec<usize>>>,
}

impl AnEngine {
    /// Un motor que contesta eso, en ese orden, a cada llamada.
    pub(crate) fn answering(answers: &[&[usize]]) -> Self {
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

pub(crate) const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Un transporte que abre siempre, en el puerto sorteado o en el fijo, y apunta lo que se le pidió.
pub(crate) fn a_transport(
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

pub(crate) fn a_launch(ports: &str) -> String {
    format!("afirma://websocket?ports={ports}&v=4&idsession={CREDENTIAL}")
}

/// Un arranque de la version 3: sin `ports`, atendido en el puerto fijo.
pub(crate) fn a_v3_launch() -> String {
    format!("afirma://websocket?v=3&idsession={CREDENTIAL}")
}

/// Asa de respuesta simulada y su receptor para pruebas.
pub(crate) fn the_wire() -> (ReplyHandle, tokio::sync::oneshot::Receiver<String>) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    (
        ReplyHandle::of(move |text| {
            let _ = sender.send(text);
            Acknowledgement::immediate()
        }),
        receiver,
    )
}

/// Asa de respuesta simulada cuyo acuse de entrega nunca llega.
pub(crate) fn a_wire_that_never_confirms() -> (ReplyHandle, tokio::sync::oneshot::Receiver<String>)
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    (
        ReplyHandle::of(move |text| {
            let _ = sender.send(text);
            Acknowledgement::never()
        }),
        receiver,
    )
}

/// Códec negociado para pruebas.
pub(crate) fn a_codec() -> NegotiatedCodec {
    Arc::new(V4Codec)
}

/// Tabla de códecs para pruebas: la version 4 sortea, la 3 usa el puerto fijo.
pub(crate) fn a_codec_table() -> crate::site::application::site::CodecTable {
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
pub(crate) fn a_live() -> LiveErrand {
    LiveErrand::speaking(a_codec())
}

/// La operación leída por el códec, que es como le llega a la mesa.
pub(crate) fn decoded(url: &AfirmaUrl) -> SiteRequest {
    V4Codec.decode(url)
}

/// La línea que el códec escribe para ese desenlace.
pub(crate) fn on_the_wire(outcome: &SiteOutcome) -> String {
    V4Codec.encode(outcome)
}

/// Lo que sale al cable, si ha salido algo.
pub(crate) fn what_the_site_received(
    wire: &mut tokio::sync::oneshot::Receiver<String>,
) -> Option<String> {
    wire.try_recv().ok()
}

pub(crate) fn a_credential() -> ChannelCredential {
    ChannelCredential::parse(CREDENTIAL).expect("es una credencial buena")
}

/// Petición tal y como llega por el canal.
pub(crate) fn arriving_over_the_channel(message: &str) -> AfirmaUrl {
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
pub(crate) fn an_operation(parameters: &str) -> AfirmaUrl {
    let text = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}{parameters}");
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

pub(crate) fn requested(url: &AfirmaUrl) -> SelectCertificate {
    let SiteOperation::SelectCertificate(request) =
        read_operation(url).expect("es una operacion que se atiende")
    else {
        panic!("es una seleccion de certificado");
    };
    request
}

/// El hilo del puente en grada A: sin librería detrás, o con el doble que contesta las dos fases.
#[derive(Default)]
pub(crate) enum TheBridge {
    #[default]
    Missing,
    Answering(ABridgeThatSigns),
}

impl TheBridge {
    pub(crate) fn answering() -> Self {
        Self::Answering(ABridgeThatSigns::default())
    }

    pub(crate) fn extra_params_of_the_presign(&self) -> String {
        let Self::Answering(bridge) = self else {
            panic!("este puente no atiende nada");
        };
        let calls = bridge.calls();
        let call = calls.first().expect("la prefirma cruzo");
        call.extra_params.clone()
    }

    pub(crate) fn operation_of_the_presign(&self) -> SignatureOperation {
        let Self::Answering(bridge) = self else {
            panic!("este puente no atiende nada");
        };
        let calls = bridge.calls();
        calls.first().expect("la prefirma cruzo").operation
    }

    pub(crate) fn algorithm_of_the_presign(&self) -> String {
        let Self::Answering(bridge) = self else {
            panic!("este puente no atiende nada");
        };
        let calls = bridge.calls();
        calls.first().expect("la prefirma cruzo").algorithm.clone()
    }

    pub(crate) fn format_of_the_presign(&self) -> Format {
        let Self::Answering(bridge) = self else {
            panic!("este puente no atiende nada");
        };
        let calls = bridge.calls();
        calls.first().expect("la prefirma cruzo").format
    }

    /// El formato con el que llegó cada prefirma que cruzó, en orden.
    pub(crate) fn formats_of_the_presigns(&self) -> Vec<Format> {
        let Self::Answering(bridge) = self else {
            panic!("este puente no atiende nada");
        };
        bridge.calls().iter().map(|call| call.format).collect()
    }
}

impl IsolateHost for TheBridge {
    fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce(&dyn Bridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone> {
        match self {
            Self::Missing => NoIsolate.run(task),
            Self::Answering(bridge) => AnIsolateWith(bridge).run(task),
        }
    }
}

/// Un token que firma cualquier cosa con los mecanismos que declara, sin PKCS#11 delante.
pub(crate) struct ATokenThatSigns {
    offered: Vec<SignatureAlgorithm>,
    secrets_asked: std::sync::Mutex<usize>,
}

impl Default for ATokenThatSigns {
    fn default() -> Self {
        Self::offering(&SignatureAlgorithm::ALL)
    }
}

impl ATokenThatSigns {
    pub(crate) fn offering(offered: &[SignatureAlgorithm]) -> Self {
        Self {
            offered: offered.to_vec(),
            secrets_asked: std::sync::Mutex::new(0),
        }
    }

    pub(crate) fn secrets_asked(&self) -> usize {
        *crate::lock(&self.secrets_asked)
    }
}

impl Signer for ATokenThatSigns {
    fn accepts_the_secret(
        &self,
        _reference: &crate::identity::domain::certificate::CertificateRef,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), crate::identity::domain::error::TokenError> {
        Ok(())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        *crate::lock(&self.secrets_asked) += 1;
        Ok(StoreSecret::NotNeeded)
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        if self.offered.contains(&algorithm) {
            return Ok(());
        }
        Err(TokenError::new(
            Situation::MechanismNotOffered,
            format!(
                "el token no firma {} con {}: no esta entre los mecanismos de la ranura",
                algorithm.name(),
                algorithm.mechanism_type()
            ),
        ))
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        _pin: &str,
        _algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        Ok(vec![0x01; 256])
    }
}

/// Los vecinos del trámite en grada A: el token vacío, lo listado, lo abierto, la memoria y una sesión sin ciclo.
pub(crate) struct TheNeighbours<'a> {
    pub(crate) stores: Vec<Store>,
    pub(crate) home: &'a Path,
    pub(crate) listed: &'a ListedCertificates,
    pub(crate) opened: &'a OpenedDocuments,
    pub(crate) memory: &'a Memory,
    pub(crate) token: InMemoryTokenSigning,
    pub(crate) signer: ATokenThatSigns,
    pub(crate) ours: Vec<TokenCertificate>,
    pub(crate) bridge: TheBridge,
    pub(crate) session: SigningSession,
}

impl Certificates for TheNeighbours<'_> {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        if self.ours.is_empty() {
            return NoToken.list_across(&self.stores);
        }
        Ok(self.ours.clone())
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
}

impl TokenSigning for TheNeighbours<'_> {
    fn secret_of(&self, certificate: &TokenCertificate) -> Result<StoreSecret, SigningRefusal> {
        self.token.secret_of(certificate)
    }

    fn sign(
        &self,
        certificate: &TokenCertificate,
        secret: &str,
        algorithm: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        self.token.sign(certificate, secret, algorithm, data)
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
                format: request.format,
                algorithm: crate::site::adapters::desk::composed_for(
                    request.algorithm,
                    request.certificate.key_kind(),
                ),
                operation: request.operation,
                parameters: request.from_the_site,
                allow_unregistered_signatures: request.allow_unregistered_signatures,
            },
            &self.signer,
            &self.bridge,
            &self.session,
        )
        .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))
    }

    fn sign_on_token(&self, secret: &str) -> Result<(), SigningRefusal> {
        session::sign_on_token(&self.signer, &self.session, secret)
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        let signed = session::finish(&self.bridge, &self.session)
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))?;
        Ok(SiteSignature {
            signature: signed.completed.into_signed_document(),
            signer_der: signed.signer_der,
        })
    }

    fn the_pdf_password(&self, _after_a_wrong_one: bool) -> Option<String> {
        None
    }
}

/// Los mismos vecinos, pero la firma ya está hecha: para probar la rama de `finish` que compone
/// el guardado de `signandsave` sin pasar por ningún ciclo de firma real (grada A no tiene uno).
pub(crate) struct ASignerThatSucceeds<'a> {
    pub(crate) neighbours: TheNeighbours<'a>,
    pub(crate) listed: Vec<TokenCertificate>,
    pub(crate) signature: SiteSignature,
}

impl Certificates for ASignerThatSucceeds<'_> {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.listed.clone())
    }

    fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        self.neighbours.rows_of(found)
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        self.neighbours.usable(found, handle)
    }
}

impl TokenSigning for ASignerThatSucceeds<'_> {
    fn secret_of(&self, certificate: &TokenCertificate) -> Result<StoreSecret, SigningRefusal> {
        self.neighbours.secret_of(certificate)
    }

    fn sign(
        &self,
        certificate: &TokenCertificate,
        secret: &str,
        algorithm: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        self.neighbours.sign(certificate, secret, algorithm, data)
    }
}

impl ScratchDocuments for ASignerThatSucceeds<'_> {
    fn open_unrecorded(&self, path: std::path::PathBuf) -> String {
        self.neighbours.open_unrecorded(path)
    }
}

impl SiteSigning for ASignerThatSucceeds<'_> {
    fn begin(&self, _request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal> {
        Ok(StoreSecret::NotNeeded)
    }

    fn sign_on_token(&self, _secret: &str) -> Result<(), SigningRefusal> {
        Ok(())
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        Ok(SiteSignature {
            signature: self.signature.signature.clone(),
            signer_der: self.signature.signer_der.clone(),
        })
    }

    fn the_pdf_password(&self, _after_a_wrong_one: bool) -> Option<String> {
        None
    }
}

/// Los vecinos de una selección de certificado, que no abre ningún documento.
pub(crate) fn a_neighbourhood<'a>(
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
        token: InMemoryTokenSigning::default(),
        signer: ATokenThatSigns::default(),
        ours: Vec::new(),
        bridge: TheBridge::default(),
        session: SigningSession::default(),
    }
}

/// Nadie abre nada en una selección de certificado.
pub(crate) fn opened_for_nobody() -> &'static OpenedDocuments {
    static NOBODY: std::sync::LazyLock<OpenedDocuments> =
        std::sync::LazyLock::new(OpenedDocuments::new);
    &NOBODY
}

/// Mesa de trabajo del trámite configurada para pruebas.
#[expect(
    clippy::too_many_arguments,
    reason = "es el constructor de un tipo de ocho campos, no una interfaz"
)]
pub(crate) fn a_desk<'a>(
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
        validation: &NotAsked,
        neighbours: TheNeighbours {
            stores: stores.to_vec(),
            home,
            listed,
            opened,
            memory,
            token: InMemoryTokenSigning::default(),
            signer: ATokenThatSigns::default(),
            ours: Vec::new(),
            bridge: TheBridge::default(),
            session: SigningSession::default(),
        },
        scratch_dir: scratch.to_path_buf(),
        scratch: std::sync::Arc::new(crate::site::adapters::scratch::RealScratch),
        batch: std::sync::Arc::new(InMemoryBatchServices::default()),
        triphase: std::sync::Arc::new(
            crate::site::application::tests::InMemoryTriphaseServer::default(),
        ),
    }
}

/// Expansor de política para pruebas.
pub(crate) struct APolicyEngine {
    pub(crate) asked: RefCell<Vec<String>>,
    answer: Result<String, ()>,
}

impl APolicyEngine {
    pub(crate) fn answering(block: &str) -> Self {
        Self {
            asked: RefCell::new(Vec::new()),
            answer: Ok(block.to_owned()),
        }
    }

    pub(crate) fn that_refuses_the_policy() -> Self {
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
        _signed_data_length: usize,
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
pub(crate) struct ACodec {
    answers: std::sync::Mutex<Vec<SiteRequest>>,
}

impl ACodec {
    pub(crate) fn answering(requests: Vec<SiteRequest>) -> Self {
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
