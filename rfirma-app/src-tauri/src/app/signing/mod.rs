//! Sesión local de firma trifásica: prefirma, firma en el token y postfirma (ADR-0001, ADR-0016).

pub mod site;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::app::certificates::StampedHolder;
use crate::app::cycle::{
    self, CycleError, OpenCycle, SigningRequest, TokenSignature, NOTHING_FROM_A_SITE,
};
use crate::app::in_hand::DocumentInHand;
use crate::app::{certificates, documents, lock, recents};
use crate::commands::orders::SigningOrder;
use crate::commands::views::{Failure, SignedDocumentView};
use crate::destination::PortalDocument;
use crate::isolate::{Isolate, IsolateGone};
use crate::memory::{Configuration, ListedCertificates, Memory, OpenedDocuments};
use crate::pkcs11::{
    self, CertificateRef, SecretOnTheReaderKeypad, Store, StoreSecret, TokenCertificate, TokenError,
};
use crate::signing::{
    compose_layer2_text, AdmissibleDocument, SessionSeal, SignatureConfig, VisibleTextFields,
};

pub use site::{begin_for_the_site, finish_for_the_site, SiteRefusal, SiteSignature, SiteSigning};

/// Sesión de firma activa entre la prefirma y la postfirma (ADR-0016).
#[derive(Default)]
pub struct SigningSession {
    open: Mutex<Option<InFlight>>,
    delivered: Mutex<Option<PathBuf>>,
}

struct InFlight {
    cycle: OpenCycle,
    document: DocumentInHand,
    signature: Option<TokenSignature>,
    certificate: CertificateRef,
    signer_der: Vec<u8>,
    seal: SessionSeal,
}

/// Prefirma local: valida admisibilidad, prepara la configuración y abre el ciclo.
pub fn begin(
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, Failure> {
    let document = DocumentInHand::taken(opened, &order.document)?;
    let bytes = admitted_bytes(document.document())?;
    let (config, reference, chain) = plan_signature(stores, listed, order)?;
    Ok(open_the_cycle(
        document,
        bytes,
        config,
        reference,
        chain,
        &NOTHING_FROM_A_SITE,
        isolate,
        session,
    )?)
}

/// Errores tipados durante el tramo trifásico de firma.
#[derive(Debug)]
pub enum CycleFailure {
    /// El documento no se ha podido leer del disco.
    DocumentUnreadable(String),
    /// El ciclo ha fallado en alguna de sus comprobaciones.
    Cycle(CycleError),
    /// El secreto debe introducirse en el teclado del lector.
    SecretOnTheReaderKeypad(SecretOnTheReaderKeypad),
    /// El hilo del isolate no está disponible.
    Gone(IsolateGone),
}

impl From<CycleError> for CycleFailure {
    fn from(error: CycleError) -> Self {
        Self::Cycle(error)
    }
}

impl From<TokenError> for CycleFailure {
    fn from(error: TokenError) -> Self {
        Self::Cycle(CycleError::from(error))
    }
}

impl From<SecretOnTheReaderKeypad> for CycleFailure {
    fn from(refusal: SecretOnTheReaderKeypad) -> Self {
        Self::SecretOnTheReaderKeypad(refusal)
    }
}

impl From<IsolateGone> for CycleFailure {
    fn from(gone: IsolateGone) -> Self {
        Self::Gone(gone)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "es el cuerpo compartido de dos casos de uso, no una interfaz"
)]
pub(super) fn open_the_cycle(
    document: DocumentInHand,
    bytes: Vec<u8>,
    config: crate::signing::SignatureConfig,
    reference: CertificateRef,
    chain: Vec<Vec<u8>>,
    from_the_site: &BTreeMap<String, String>,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, CycleFailure> {
    let secret = pkcs11::store_secret(&reference)?.admitted()?;
    let certificate = reference.clone();
    let signer_der = chain.first().cloned().unwrap_or_default();
    let from_the_site = from_the_site.clone();

    let cycle = on_the_bridge_with_situation(isolate, move |bridge| {
        let document = AdmissibleDocument::check(&bytes)?;
        cycle::presign(
            bridge,
            SigningRequest {
                document,
                chain: &chain,
                config: &config,
                from_the_site: &from_the_site,
                certificate: &reference,
            },
        )
    })?;

    let seal = cycle.seal_in_transit();
    *lock(&session.open) = Some(InFlight {
        cycle,
        document,
        signature: None,
        certificate,
        signer_der,
        seal,
    });
    Ok(secret)
}

/// Fase de firma en el token PKCS#11 con el PIN proporcionado (ADR-0001).
pub fn sign_on_token(session: &SigningSession, pin: &str) -> Result<(), Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.as_mut().ok_or_else(no_open_cycle)?;
    in_flight.signature = Some(in_flight.cycle.sign_on_token(pin)?);
    Ok(())
}

/// Postfirma: verifica el sello, compone el PDF y lo entrega en destino (ADR-0011, ADR-0016).
pub fn finish(
    isolate: &Isolate,
    session: &SigningSession,
    memory: &Memory,
    configuration: &Configuration,
    documents_folder: &Path,
) -> Result<SignedDocumentView, Failure> {
    let SignedCycle {
        cycle,
        document,
        signature,
        seal,
        certificate,
        ..
    } = take_signed_cycle(session)?;

    let signed = on_the_bridge(isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    let (landing, delivered) = documents::deliver(
        configuration,
        documents_folder,
        document.document(),
        &signed,
    )?;
    certificates::remember_the_certificate(memory, configuration, &certificate);
    if document.is_remembered() {
        recents::note_signed(memory, configuration, &landing);
    }
    *lock(&session.delivered) = Some(landing);
    Ok(delivered)
}

/// Ruta del último documento firmado entregado en esta sesión (ADR-0011).
pub fn signed_document(session: &SigningSession) -> Result<PathBuf, Failure> {
    lock(&session.delivered)
        .clone()
        .ok_or_else(no_signed_document)
}

/// Directorio del último documento firmado entregado en esta sesión (ADR-0011).
pub fn signed_folder(session: &SigningSession) -> Result<PathBuf, Failure> {
    let landing = signed_document(session)?;
    landing
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(no_signed_document)
}

fn no_signed_document() -> Failure {
    Failure::new("unknown", "no hay ningun documento firmado en esta sesion")
}

/// Indica si hay una sesión de firma activa en curso.
pub fn is_live(session: &SigningSession) -> bool {
    lock(&session.open).is_some()
}

/// Cancela la sesión activa descartando el ciclo en curso.
pub fn cancel(session: &SigningSession) {
    *lock(&session.open) = None;
}

fn layer2_text_of(order: &SigningOrder, holder: &StampedHolder) -> String {
    compose_layer2_text(
        &VisibleTextFields {
            signer_name: order
                .fields
                .signer_name
                .then_some(holder.common_name.as_str())
                .filter(|name| !name.is_empty()),
            issuer: order
                .fields
                .issuer
                .then_some(holder.issuer.as_str())
                .filter(|issuer| !issuer.is_empty()),
            signed_at: order.fields.signed_at.then_some(order.signed_at.as_str()),
            reason: order
                .fields
                .reason
                .then_some(order.reason.as_str())
                .filter(|reason| !reason.is_empty()),
            pseudonym: holder.pseudonym,
        },
        super::configuration::language_of(&order.language),
    )
}

/// Configuración de firma construida a partir de la orden y del certificado seleccionado.
pub fn config_for(
    order: &SigningOrder,
    chosen: &TokenCertificate,
) -> Result<SignatureConfig, Failure> {
    let holder = certificates::stamped_holder_of(chosen);
    Ok(SignatureConfig {
        placement: order
            .placement
            .as_ref()
            .map(|placement| placement.placement())
            .transpose()?,
        layer2_text: layer2_text_of(order, &holder),
        rubric_image: order.rubric.clone(),
        sign_reason: (!order.reason.is_empty()).then(|| order.reason.clone()),
        allow_unregistered_signatures: order.allow_unregistered_signatures,
    })
}

pub(crate) fn plan_signature(
    stores: &[Store],
    listed: &ListedCertificates,
    order: &SigningOrder,
) -> Result<(SignatureConfig, CertificateRef, Vec<Vec<u8>>), Failure> {
    let found = pkcs11::list_certificates_across(stores)?;
    let chosen = certificates::usable_certificate(&found, &order.certificate, listed)?;
    Ok((
        config_for(order, chosen)?,
        chosen.reference().clone(),
        vec![chosen.der().to_vec()],
    ))
}

/// Obtiene y valida los bytes de un documento para firmar.
pub fn admitted_bytes(document: &PortalDocument) -> Result<Vec<u8>, Failure> {
    admitted_bytes_with_situation(document).map_err(Failure::from)
}

pub(super) fn admitted_bytes_with_situation(
    document: &PortalDocument,
) -> Result<Vec<u8>, CycleFailure> {
    let bytes = std::fs::read(document.reading_path())
        .map_err(|error| CycleFailure::DocumentUnreadable(error.to_string()))?;
    AdmissibleDocument::check(&bytes).map_err(CycleError::from)?;
    Ok(bytes)
}

/// Comprueba si el documento contiene firmas previas no reconocibles.
pub fn unregistered_signatures_in(
    opened: &OpenedDocuments,
    document: &str,
) -> Result<bool, Failure> {
    let in_hand = DocumentInHand::taken(opened, document)?;
    let bytes = admitted_bytes(in_hand.document())?;
    Ok(AdmissibleDocument::check(&bytes)?.has_unregistered_signatures())
}

/// Extrae el ciclo completado en el token de la sesión activa.
pub fn take_signed_cycle(session: &SigningSession) -> Result<SignedCycle, Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.take().ok_or_else(no_open_cycle)?;
    let signature = in_flight
        .signature
        .ok_or_else(|| Failure::new("unknown", "todavía no se ha firmado en el token"))?;
    Ok(SignedCycle {
        cycle: in_flight.cycle,
        document: in_flight.document,
        signature,
        seal: in_flight.seal,
        certificate: in_flight.certificate,
        signer_der: in_flight.signer_der,
    })
}

/// Ciclo firmado en el token preparado para la postfirma.
pub struct SignedCycle {
    pub cycle: OpenCycle,
    pub document: DocumentInHand,
    pub signature: TokenSignature,
    pub seal: SessionSeal,
    pub certificate: CertificateRef,
    pub signer_der: Vec<u8>,
}

pub(crate) fn on_the_bridge<T: Send + 'static>(
    isolate: &Isolate,
    task: impl FnOnce(&crate::ffi::NativeBridge) -> Result<T, cycle::CycleError> + Send + 'static,
) -> Result<T, Failure> {
    on_the_bridge_with_situation(isolate, task).map_err(Failure::from)
}

pub(crate) fn on_the_bridge_with_situation<T: Send + 'static>(
    isolate: &Isolate,
    task: impl FnOnce(&crate::ffi::NativeBridge) -> Result<T, cycle::CycleError> + Send + 'static,
) -> Result<T, CycleFailure> {
    match isolate.run(task) {
        Err(gone) => Err(gone.into()),
        Ok(Err(bridge)) => Err(CycleError::from(bridge).into()),
        Ok(Ok(outcome)) => outcome.map_err(CycleFailure::from),
    }
}

fn no_open_cycle() -> Failure {
    Failure::new("unknown", "no hay ninguna firma empezada")
}

#[cfg(test)]
mod tests {
    use super::{
        admitted_bytes, begin, cancel, config_for, finish, is_live, sign_on_token, signed_document,
        signed_folder, take_signed_cycle, SigningSession,
    };
    use crate::app::fixtures::{a_certificate, a_memory, an_order};
    use crate::commands::orders::{PlacementOrder, SigningOrder};
    use crate::destination::PortalDocument;
    use crate::isolate::Isolate;
    use crate::memory::{Configuration, ListedCertificates, OpenedDocuments};
    use crate::signing::PageSet;

    const SOURCE: &str = include_str!("mod.rs");

    fn production_half() -> &'static str {
        half_of(SOURCE)
    }

    fn half_of(source: &'static str) -> &'static str {
        source
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(source)
    }

    #[test]
    fn the_pin_is_never_kept_in_the_half_open_cycle() {
        let session = production_half()
            .split_once("struct InFlight {")
            .expect("la sesión sigue aquí")
            .1
            .split_once("\n}")
            .expect("y tiene cuerpo")
            .0;

        assert!(
            !session.contains("pin"),
            "el PIN se está guardando: {session}"
        );
    }

    #[test]
    fn the_seal_travels_apart_from_the_cycle_that_issued_it() {
        // ADR-0016.
        let session = production_half()
            .split_once("struct InFlight {")
            .expect("la sesión sigue aquí")
            .1;

        assert!(session.contains("seal: SessionSeal"));
    }

    #[test]
    fn the_signed_badge_is_written_by_the_postsign_and_by_nothing_else() {
        let writers = [
            ("app/signing/mod.rs", production_half()),
            ("app/recents.rs", half_of(include_str!("../recents.rs"))),
            ("app/signing/site.rs", half_of(include_str!("site.rs"))),
            (
                "app/errand/mod.rs",
                half_of(include_str!("../errand/mod.rs")),
            ),
            (
                "app/errand/desk.rs",
                half_of(include_str!("../errand/desk.rs")),
            ),
            (
                "app/errand/replies.rs",
                half_of(include_str!("../errand/replies.rs")),
            ),
            (
                "app/errand/state.rs",
                half_of(include_str!("../errand/state.rs")),
            ),
            ("app/policies.rs", half_of(include_str!("../policies.rs"))),
            ("app/documents.rs", half_of(include_str!("../documents.rs"))),
            ("app/in_hand.rs", half_of(include_str!("../in_hand.rs"))),
            (
                "app/invocation.rs",
                half_of(include_str!("../invocation.rs")),
            ),
            ("app/preview.rs", half_of(include_str!("../preview.rs"))),
            (
                "commands/mod.rs",
                half_of(include_str!("../../commands/mod.rs")),
            ),
            (
                "commands/site_window.rs",
                half_of(include_str!("../../commands/site_window.rs")),
            ),
        ];

        for (file, source) in writers {
            let written = source.matches("Badge::Signed").count();
            let expected = usize::from(file == "app/recents.rs");
            assert_eq!(
                written, expected,
                "«{file}» escribe la insignia Firmado {written} veces y tenia que escribirla \
                 {expected}: el unico sitio es `recents::note_signed`, y quien lo llama es la \
                 postfirma"
            );
        }

        let recents = half_of(include_str!("../recents.rs"));
        let note_signed = recents
            .split_once("pub fn note_signed(")
            .expect("el anotador del firmado sigue aqui")
            .1;
        assert!(
            note_signed.contains("Badge::Signed"),
            "y esta dentro de `note_signed`"
        );
        let postsign = production_half()
            .split_once("pub fn finish(")
            .expect("la postfirma sigue aqui")
            .1;
        assert!(
            postsign.contains("recents::note_signed("),
            "a quien solo llama la postfirma"
        );
    }

    #[test]
    fn a_document_that_is_not_remembered_gets_no_row_when_it_is_signed() {
        let postsign = production_half()
            .split_once("pub fn finish(")
            .expect("la postfirma sigue aqui")
            .1;
        let before_the_row = postsign
            .split_once("recents::note_signed(")
            .expect("la postfirma anota la fila")
            .0;

        assert!(
            before_the_row.contains("if document.is_remembered() {"),
            "la fila del firmado se escribe sin preguntar si el documento se recuerda"
        );
    }

    #[test]
    fn only_the_postsign_remembers_the_certificate() {
        let source = production_half();

        assert_eq!(
            source
                .matches("certificates::remember_the_certificate(")
                .count(),
            1,
            "se recuerda desde un solo sitio"
        );
        let postsign = source
            .split_once("pub fn finish(")
            .expect("la postfirma sigue aqui")
            .1;
        assert!(
            postsign.contains("certificates::remember_the_certificate("),
            "y ese sitio es la postfirma"
        );
    }

    #[test]
    fn the_geometry_of_the_order_becomes_pades_points() {
        let certificate = a_certificate("FIRMA", &[]);

        let config = config_for(&an_order(), &certificate).expect("el recuadro cabe");

        let placement = config.placement.expect("la ventana coloco el recuadro");
        assert_eq!(placement.pages, PageSet::only_page(1));
        assert_eq!(placement.rect.lower_left_x, 72);
        assert_eq!(placement.rect.upper_right_y, 600);
    }

    #[test]
    fn a_box_outside_the_page_is_refused_instead_of_being_clipped_in_silence() {
        let order = SigningOrder {
            placement: Some(PlacementOrder {
                rect: [72.0, 500.0, 900.0, 600.0],
                ..an_order().placement.expect("el andamio trae recuadro")
            }),
            ..an_order()
        };

        let failure = config_for(&order, &a_certificate("FIRMA", &[])).expect_err("se sale");

        assert_eq!(failure.situation, "boxOutOfPage");
    }

    #[test]
    fn an_empty_reason_is_not_sent_at_all() {
        let config = config_for(&an_order(), &a_certificate("FIRMA", &[])).expect("cabe");

        assert_eq!(config.sign_reason, None);
    }

    #[test]
    fn a_reason_that_was_written_does_travel() {
        let order = SigningOrder {
            reason: "Conforme".to_owned(),
            ..an_order()
        };

        let config = config_for(&order, &a_certificate("FIRMA", &[])).expect("cabe");

        assert_eq!(config.sign_reason.as_deref(), Some("Conforme"));
    }

    #[test]
    fn there_is_nothing_to_finish_when_no_cycle_was_started() {
        let session = SigningSession::default();

        let Err(failure) = take_signed_cycle(&session) else {
            panic!("no hay ciclo abierto que llevarse");
        };

        assert_eq!(failure.situation, "unknown");
    }

    #[test]
    fn there_is_nothing_to_open_before_the_first_signature_of_the_session() {
        let session = SigningSession::default();

        let Err(failure) = signed_document(&session) else {
            panic!("no se ha firmado nada todavia");
        };
        assert_eq!(failure.situation, "unknown");
        assert!(signed_folder(&session).is_err());
    }

    #[test]
    fn the_two_openers_read_the_landing_the_postsign_left_behind() {
        let session = SigningSession::default();
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let landing = folder.path().join("contrato-firmado.pdf");
        *crate::app::lock(&session.delivered) = Some(landing.clone());

        assert_eq!(signed_document(&session).expect("hay firmado"), landing);
        assert_eq!(signed_folder(&session).expect("y carpeta"), folder.path());
    }

    #[test]
    fn a_session_with_no_open_cycle_is_not_live() {
        assert!(!is_live(&SigningSession::default()));
    }

    #[test]
    fn a_cancelled_session_is_not_live_either() {
        let session = SigningSession::default();

        cancel(&session);

        assert!(!is_live(&session));
    }

    #[test]
    fn the_remembered_landing_never_leaves_the_backend() {
        let crossing = production_half()
            .split_once("pub struct SigningSession {")
            .expect("la sesion sigue aqui")
            .1
            .split_once("\n}")
            .expect("y tiene cuerpo")
            .0;

        assert!(
            crossing.contains("delivered"),
            "la sesion tiene que recordar donde cayo el firmado: {crossing}"
        );
        assert!(
            !crossing.contains("Serialize"),
            "la sesion no se serializa: si cruzara, cruzaria una ruta del anfitrion"
        );
    }

    #[test]
    fn what_is_not_a_pdf_is_refused_before_the_pin() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let other = home.path().join("hoja.ods");
        std::fs::write(&other, b"PK\x03\x04").expect("deberia escribirse el temporal");

        let failure =
            admitted_bytes(&PortalDocument::opened(other)).expect_err("no es un PDF que firmar");

        assert_eq!(failure.situation, "notAPdf");
    }

    #[test]
    fn a_document_that_is_gone_is_told_apart_from_one_that_is_not_a_pdf() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");

        let failure = admitted_bytes(&PortalDocument::opened(home.path().join("no-esta.pdf")))
            .expect_err("no esta");

        assert_eq!(failure.situation, "documentUnreadable");
    }

    #[test]
    fn a_signature_cannot_begin_on_a_document_that_is_not_open() {
        let order = SigningOrder {
            document: "00000000000000000000000000000000".to_owned(),
            ..an_order()
        };

        let failure = begin(
            &order,
            &[],
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &Isolate::start(),
            &SigningSession::default(),
        )
        .expect_err("ese documento no esta abierto");

        assert_eq!(failure.situation, "documentUnreadable");
    }

    #[test]
    fn the_postsign_stops_before_the_bridge_when_no_cycle_was_started() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");

        let failure = finish(
            &Isolate::start(),
            &SigningSession::default(),
            &a_memory(home.path()),
            &Configuration::default(),
            home.path(),
        )
        .expect_err("no hay ciclo abierto");

        assert_eq!(failure.situation, "unknown");
    }

    #[test]
    fn the_pin_has_nothing_to_sign_when_no_cycle_was_started() {
        let failure =
            sign_on_token(&SigningSession::default(), "1234").expect_err("no hay ciclo abierto");

        assert_eq!(failure.situation, "unknown");
    }

    #[test]
    fn cancelling_leaves_no_cycle_behind() {
        let session = SigningSession::default();

        cancel(&session);

        assert!(
            take_signed_cycle(&session).is_err(),
            "no queda ciclo que llevarse"
        );
    }
}
