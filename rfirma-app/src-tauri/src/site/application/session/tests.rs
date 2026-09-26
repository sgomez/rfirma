use super::{begin_for_the_site, finish_for_the_site, SiteTerms};
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_certificate, NoMemory, NoToken};
use crate::identity::domain::error::TokenError;
use crate::signing::adapters::failures::told_of_cycle;
use crate::signing::application::session::{config_for, SigningSession};
use crate::signing::application::tests::{an_order, NoIsolate};
use crate::signing::domain::bridge::Format;
use crate::site::application::tests::Directory;
use crate::site::domain::protocol::{AskedAlgorithm, SafCode, SiteFilter};
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{FilterEngine, Neighbours, SiteSigningRequest};
use std::collections::BTreeMap;

const SOURCE: &str = include_str!("../session.rs");

/// Quien firma para la sede en producción: el adaptador sobre las raíces vecinas.
const THE_SIGNER: &str = include_str!("../../adapters/desk.rs");

fn production_half() -> &'static str {
    SOURCE
}

#[test]
fn the_postsign_of_a_site_errand_writes_nothing_anywhere() {
    let site_postsign = THE_SIGNER
        .split_once("fn finish(&self)")
        .expect("la postfirma de la sede sigue aqui")
        .1;

    for forbidden in [
        "deliver",
        "note_signed",
        "note_delivered",
        "remember_the_certificate",
    ] {
        assert!(
            !site_postsign.contains(forbidden),
            "la postfirma de la sede llama a «{forbidden}»: el documento que manda una sede no \
             deja rastro y rFirma no guarda ficheros por orden suya"
        );
    }
}

#[test]
fn the_presign_of_a_site_errand_checks_the_filter_again_before_the_pin() {
    let site_presign = production_half()
        .split_once("pub fn begin_for_the_site<")
        .expect("la prefirma de la sede sigue aqui")
        .1
        .split_once("\n/// ")
        .expect("y termina donde empieza la siguiente")
        .0;

    let (before, after) = site_presign
        .split_once("filtering::usable_certificate_for_the_site(")
        .expect("el filtro de la sede se vuelve a comprobar antes de pedir el secreto");
    assert!(
        !before.contains("neighbours.begin("),
        "y se comprueba antes de abrir el ciclo"
    );
    assert!(after.contains("neighbours.begin("), "que se abre después");
}

#[test]
fn a_signature_the_site_placed_carries_no_geometry_of_our_own() {
    let config = config_for(
        &crate::signing::domain::SigningChoice::for_the_site(false),
        &a_certificate("FIRMA", &[]),
    )
    .expect("no hay que colocar");

    assert_eq!(config.placement, None);
    for key in crate::signing::domain::Setting::Geometry.keys() {
        assert!(!config.extra_params().contains_key(*key), "'{key}' es suya");
    }
}

#[test]
fn a_site_signature_cannot_begin_on_a_document_that_is_not_open() {
    let order = an_order();
    let certificates = vec![crate::identity::application::tests::a_usable_certificate(
        "FIRMA",
    )];
    let listed = ListedCertificates::new();
    let handles = listed.replace(
        certificates
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );
    let engine = AcceptingEngine;

    let failure = begin_for_the_site(
        &SiteTerms {
            engine: &engine,
            filter: &SiteFilter::default(),
            format: Format::Pades,
            algorithm: AskedAlgorithm::Sha256,
            operation: crate::signing::domain::bridge::SignatureOperation::Sign,
            from_the_site: &BTreeMap::new(),
            allow_unregistered_signatures: false,
        },
        &order.document,
        &handles[0],
        &NobodyHasItOpen {
            directory: Directory {
                certificates,
                listed: &listed,
                memory: &NoMemory,
            },
        },
    )
    .expect_err("ese documento no esta abierto");

    let (told, code) = crate::site::adapters::frontier::told(&failure);
    assert_eq!(told.situation, "documentUnreadable");
    assert_eq!(
        code,
        SafCode::CannotReadData,
        "y la sede recibe el codigo de lo que ha pasado, no uno para todo"
    );
}

#[test]
fn a_postsign_without_an_open_cycle_is_refused_with_what_the_signer_said() {
    let listed = ListedCertificates::new();
    let neighbours = NobodyHasItOpen {
        directory: Directory {
            certificates: Vec::new(),
            listed: &listed,
            memory: &NoMemory,
        },
    };
    let failure = finish_for_the_site(&neighbours).expect_err("no hay ciclo");

    let (told, code) = crate::site::adapters::frontier::told(&failure);
    assert_eq!(told.situation, "unknown");
    assert_eq!(code, SafCode::SignatureFailed);
}

/// Un motor que acepta todo lo que le pasan.
struct AcceptingEngine;

impl FilterEngine for AcceptingEngine {
    fn select(
        &self,
        _properties: &str,
        certificates: &str,
    ) -> Result<Vec<usize>, crate::signing::domain::bridge::BridgeError> {
        Ok((0..certificates.split(';').count()).collect())
    }
}

/// Quien firma cuando ningún documento está abierto: la sesión vacía sobre el token y el hilo de
/// la grada A, con los certificados de un `Directory` para lo que el filtro vuelve a comprobar.
struct NobodyHasItOpen<'a> {
    directory: Directory<'a>,
}

impl Neighbours for NobodyHasItOpen<'_> {
    fn listed(
        &self,
    ) -> Result<Vec<crate::identity::domain::certificate::TokenCertificate>, TokenError> {
        self.directory.listed()
    }

    fn rows_of(
        &self,
        found: Vec<crate::identity::domain::certificate::TokenCertificate>,
    ) -> Vec<crate::identity::domain::certificate::ListedCertificate> {
        self.directory.rows_of(found)
    }

    fn discovered_module(&self, library: &str) -> Option<std::path::PathBuf> {
        self.directory.discovered_module(library)
    }

    fn usable<'a>(
        &self,
        found: &'a [crate::identity::domain::certificate::TokenCertificate],
        handle: &str,
    ) -> Result<&'a crate::identity::domain::certificate::TokenCertificate, TokenError> {
        self.directory.usable(found, handle)
    }

    fn automatic_selection_honoured(&self) -> bool {
        self.directory.automatic_selection_honoured()
    }

    fn open_unrecorded(&self, _path: std::path::PathBuf) -> String {
        unreachable!("ninguna prueba de esta sesion apunta un documento de paso")
    }

    fn begin(&self, request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal> {
        let failure = crate::signing::application::session::CycleFailure::from(
            crate::documents::domain::error::DocumentError::no_longer_open(),
        );
        let _ = (request, &NoToken, &NoIsolate);
        Err(crate::site::adapters::desk::signing_refusal_of(
            told_of_cycle(&failure),
        ))
    }

    fn sign_on_token(
        &self,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), SigningRefusal> {
        unreachable!("ninguna prueba de esta sesion llega a firmar en el token")
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        crate::signing::application::session::finish(&NoIsolate, &SigningSession::default())
            .map(|_| unreachable!("no hay ciclo que cerrar"))
            .map_err(|failure| {
                crate::site::adapters::desk::signing_refusal_of(told_of_cycle(&failure))
            })
    }

    fn the_pdf_password(&self, _after_a_wrong_one: bool) -> Option<String> {
        None
    }

    fn secret_of(
        &self,
        _certificate: &crate::identity::domain::certificate::TokenCertificate,
    ) -> Result<StoreSecret, SigningRefusal> {
        unreachable!("ninguna prueba de esta sesion pide el secreto del lote remoto")
    }

    fn sign(
        &self,
        _certificate: &crate::identity::domain::certificate::TokenCertificate,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        _algorithm: &str,
        _data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        unreachable!("ninguna prueba de esta sesion firma por el lote remoto")
    }
}

use crate::identity::domain::secret::StoreSecret;
