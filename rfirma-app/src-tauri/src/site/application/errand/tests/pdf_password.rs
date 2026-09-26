//! Pruebas de la contraseña del PDF cifrado: se pide a la persona hasta que lo abre, y con `headless` se rechaza.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::application::tests::a_memory;
use crate::site::adapters::frontier;
use crate::site::application::errand::*;
use crate::site::application::tests::{InMemoryBatchServices, InMemoryTriphaseServer, NotAsked};
use crate::site::domain::protocol::SafCode;
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{
    Certificates, ScratchDocuments, SiteSigning, SiteSigningRequest, TokenSigning,
};
use base64::Engine as _;

const A_PASSWORD_PROTECTED_PDF: &[u8] =
    b"%PDF-1.7\ntrailer\n<< /Size 9 /Encrypt 8 0 R /Root 1 0 R >>\n";

const THE_PASSWORD: &str = "1234";

/// Un PDF que solo abre `1234` como contraseña de propietario, y una persona que teclea lo que se le diga.
struct ALockedPdf<'a> {
    neighbours: TheNeighbours<'a>,
    listed: Vec<TokenCertificate>,
    typed: RefCell<Vec<Option<&'static str>>>,
    asked: RefCell<Vec<bool>>,
    begun_with: RefCell<Vec<BTreeMap<String, String>>>,
}

impl Certificates for ALockedPdf<'_> {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.listed.clone())
    }

    fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        self.neighbours.rows_of(found)
    }

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        self.neighbours.discovered_module(library)
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        self.neighbours.usable(found, handle)
    }

    fn automatic_selection_honoured(&self) -> bool {
        self.neighbours.automatic_selection_honoured()
    }
}

impl TokenSigning for ALockedPdf<'_> {
    fn secret_of(&self, certificate: &TokenCertificate) -> Result<StoreSecret, SigningRefusal> {
        self.neighbours.secret_of(certificate)
    }

    fn sign(
        &self,
        certificate: &TokenCertificate,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        algorithm: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        self.neighbours.sign(certificate, secret, algorithm, data)
    }
}

impl ScratchDocuments for ALockedPdf<'_> {
    fn open_unrecorded(&self, path: std::path::PathBuf) -> String {
        self.neighbours.open_unrecorded(path)
    }
}

impl SiteSigning for ALockedPdf<'_> {
    fn begin(&self, request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal> {
        self.begun_with
            .borrow_mut()
            .push(request.from_the_site.clone());
        if request
            .from_the_site
            .get("ownerPassword")
            .map(String::as_str)
            == Some(THE_PASSWORD)
        {
            return Ok(StoreSecret::NotNeeded);
        }
        Err(SigningRefusal {
            code: SafCode::PdfWrongPassword,
            situation: "bridgeFailed".to_owned(),
            detail: "BadPdfPasswordException".to_owned(),
            attempts_left: None,
        })
    }

    fn sign_on_token(
        &self,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), SigningRefusal> {
        Ok(())
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        unreachable!("estas pruebas no llegan a la postfirma")
    }

    fn the_pdf_password(&self, after_a_wrong_one: bool) -> Option<String> {
        self.asked.borrow_mut().push(after_a_wrong_one);
        self.typed.borrow_mut().remove(0).map(str::to_owned)
    }
}

/// Lo que pasó al consentir la firma del PDF cifrado con lo declarado y lo tecleado.
struct Consenting {
    consented: Result<Consented, ConsentError>,
    asked: Vec<bool>,
    begun_with: Vec<BTreeMap<String, String>>,
    received: Option<String>,
}

fn consenting(declared: &str, typed: &[Option<&'static str>]) -> Consenting {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0usize] as &[usize]; 8]);
    let policies = APolicyEngine::answering(declared);
    let desk = ErrandDesk {
        engine: &engine,
        policies: &policies,
        validation: &NotAsked,
        neighbours: ALockedPdf {
            neighbours: a_neighbourhood(home.path(), &listed, &opened, &memory),
            listed: ours.clone(),
            typed: RefCell::new(typed.to_vec()),
            asked: RefCell::new(Vec::new()),
            begun_with: RefCell::new(Vec::new()),
        },
        scratch_dir: home.path().join("errand"),
        scratch: Arc::new(crate::site::adapters::scratch::RealScratch),
        batch: Arc::new(InMemoryBatchServices::default()),
        triphase: Arc::new(InMemoryTriphaseServer::default()),
    };
    let properties = base64::engine::general_purpose::URL_SAFE.encode(declared);
    let url = a_signature_over(
        A_PASSWORD_PROTECTED_PDF,
        "sign",
        &format!("&properties={properties}"),
    );

    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSign(asked) = remembered(&live, step) else {
        panic!("el PDF cifrado llega a pedir certificado");
    };
    let consented = consent(&desk, &asked.certificates[0].id, &live);

    Consenting {
        consented,
        asked: desk.neighbours.asked.take(),
        begun_with: desk.neighbours.begun_with.take(),
        received: what_the_site_received(&mut wire),
    }
}

#[test]
fn a_wrong_password_under_headless_is_answered_with_the_code_of_confirmation_needed() {
    let done = consenting("headless=true\nuserPassword=mal\n", &[]);

    let Err(ConsentError::Refused(refusal)) = done.consented else {
        panic!("con headless no se pregunta: {:?}", done.consented);
    };
    assert_eq!(frontier::code_of(&refusal), SafCode::ConfirmationNeeded);
    assert!(done.asked.is_empty(), "no se pidio nada a nadie");
}

#[test]
fn a_wrong_password_under_a_mandatory_selection_set_to_false_is_asked_to_the_person() {
    let done = consenting(
        "mandatoryCertSelection=false\nuserPassword=mal\n",
        &[Some(THE_PASSWORD)],
    );

    assert!(
        matches!(done.consented, Ok(Consented::SigningWith(_))),
        "{:?}",
        done.consented
    );
    assert_eq!(done.asked, [true]);
}

#[test]
fn a_wrong_password_is_asked_to_the_person_until_the_pdf_opens() {
    let done = consenting("userPassword=mal\n", &[Some("otra"), Some(THE_PASSWORD)]);

    assert!(
        matches!(done.consented, Ok(Consented::SigningWith(_))),
        "{:?}",
        done.consented
    );
    assert_eq!(done.asked, [true, true]);
    let last = done.begun_with.last().expect("se abrio la firma");
    assert_eq!(
        last.get("ownerPassword").map(String::as_str),
        Some(THE_PASSWORD)
    );
    assert!(!last.contains_key("userPassword"));
    assert_eq!(done.received, None, "la sede sigue esperando la firma");
}

#[test]
fn a_protected_pdf_without_a_password_asks_the_person_for_it() {
    let done = consenting("", &[Some(THE_PASSWORD)]);

    assert!(
        matches!(done.consented, Ok(Consented::SigningWith(_))),
        "{:?}",
        done.consented
    );
    assert_eq!(done.asked, [false]);
}

#[test]
fn declining_the_pdf_password_answers_the_site_with_a_cancel() {
    let done = consenting("userPassword=mal\n", &[None]);

    assert!(
        matches!(done.consented, Err(ConsentError::Declined)),
        "{:?}",
        done.consented
    );
    assert_eq!(done.received.as_deref(), Some("CANCEL"));
}
