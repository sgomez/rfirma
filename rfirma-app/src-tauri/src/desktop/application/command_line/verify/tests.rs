use std::cell::RefCell;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, Utc};

use super::super::{attend, CommandLinePorts, Outcome, FAILED, REFUSED, SUCCEEDED};
use super::*;
use crate::desktop::ports::{
    AskedSecret, CertificateFilter, CertificateStores, CommandLineFiles, CommandLineSigning,
    DesktopHandover, DocumentSigner, GraphicalPicker, LocalTimeZone, OfferedCertificate,
    SecretDescriptor, SignatureReader, SignatureVerifier, Terminal, WindowChoice, WindowOffer,
};
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::{
    DocumentFinding, DocumentSignature, DocumentSignatures, SignatureStatus, SigningDate, Validity,
    ValidityReason,
};
use crate::site::domain::protocol::SiteFilter;

const AN_XML: &[u8] = b"<?xml version=\"1.0\"?><root/>";

const AN_INVOICE: &[u8] =
    b"<?xml version=\"1.0\"?><fe:Facturae><FileHeader/><Parties/><Invoices/></fe:Facturae>";

const A_PDF: &[u8] = b"%PDF-1.7\n1 0 obj\n<< >>\nendobj\n";

/// Un `SignedData` mínimo: la cabecera que reconoce `is_cms_signed_data`.
const A_CMS: &[u8] = &[
    0x30, 0x80, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02, 0xa0, 0x80,
];

struct OneFile(&'static [u8]);

impl CommandLineFiles for OneFile {
    fn read(&self, _path: &Path) -> Result<Vec<u8>, String> {
        Ok(self.0.to_vec())
    }

    fn write(&self, path: &Path, _bytes: &[u8]) -> Result<(), String> {
        panic!("verify no escribe {}", path.display())
    }
}

struct NoFile;

impl CommandLineFiles for NoFile {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        Err(format!("{} no existe", path.display()))
    }

    fn write(&self, path: &Path, _bytes: &[u8]) -> Result<(), String> {
        panic!("verify no escribe {}", path.display())
    }
}

/// El validador que contesta siempre lo mismo y apunta con qué formato se le preguntó.
struct Answering {
    answer: Result<Vec<String>, String>,
    asked: RefCell<Vec<Format>>,
}

impl Answering {
    fn with(results: &[&str]) -> Self {
        Self {
            answer: Ok(results.iter().map(|result| (*result).to_owned()).collect()),
            asked: RefCell::new(Vec::new()),
        }
    }

    fn failing() -> Self {
        Self {
            answer: Err("el isolate no arranca".to_owned()),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl SignatureVerifier for Answering {
    fn results_of(&self, _document: &[u8], format: Format) -> Result<Vec<String>, BridgeError> {
        self.asked.borrow_mut().push(format);
        self.answer.clone().map_err(BridgeError::Failed)
    }
}

/// La lectura de firmas que contesta siempre lo mismo.
struct Reading(Result<Vec<DocumentSignature>, String>);

impl SignatureReader for Reading {
    fn signatures_in(&self, _document: &[u8]) -> Result<DocumentSignatures, BridgeError> {
        self.0
            .clone()
            .map(|signatures| DocumentSignatures::new(signatures, false))
            .map_err(BridgeError::Failed)
    }
}

/// La lectura de firmas que además trae hallazgos del documento.
struct ReadingWithFindings(Vec<DocumentSignature>, Vec<DocumentFinding>);

impl SignatureReader for ReadingWithFindings {
    fn signatures_in(&self, _document: &[u8]) -> Result<DocumentSignatures, BridgeError> {
        Ok(DocumentSignatures::new(self.0.clone(), false).with_findings(self.1.clone()))
    }
}

/// La zona horaria de Madrid en verano.
struct SummerInMadrid;

impl LocalTimeZone for SummerInMadrid {
    fn offset_at(&self, _instant: DateTime<Utc>) -> FixedOffset {
        FixedOffset::east_opt(2 * 3600).expect("+02:00 es un desplazamiento")
    }
}

/// Los puertos que `verify` no debería llegar a tocar.
struct Untouched;

impl CertificateStores for Untouched {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        panic!("verify no abre almacenes")
    }

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        panic!("verify no busca el módulo {library}")
    }
}

impl CertificateFilter for Untouched {
    fn accepted(
        &self,
        _filter: &SiteFilter,
        _certificates: Vec<TokenCertificate>,
    ) -> Result<Vec<TokenCertificate>, String> {
        panic!("verify no filtra certificados")
    }
}

impl Terminal for Untouched {
    fn is_interactive(&self) -> bool {
        panic!("verify no pregunta a la terminal")
    }

    fn secret(&self, asked: &AskedSecret<'_>) -> Result<ProtectedSecret, String> {
        panic!("verify no pide el secreto de {}", asked.alias)
    }

    fn chosen(&self, offered: &[OfferedCertificate], _preselected: usize) -> Result<usize, String> {
        panic!("verify no elige entre {} certificados", offered.len())
    }
}

impl SecretDescriptor for Untouched {
    fn read(&self, descriptor: u32) -> Result<ProtectedSecret, String> {
        panic!("verify no lee el descriptor {descriptor}")
    }
}

impl DesktopHandover for Untouched {
    fn hand_over(
        &self,
        file: &Path,
        _intent: crate::desktop::domain::command_line::WindowIntent,
    ) -> Result<(), String> {
        panic!("verify sin -gui no abre la ventana con {}", file.display())
    }
}

impl GraphicalPicker for Untouched {
    fn has_a_display(&self) -> bool {
        panic!("verify no abre la ventana de sede")
    }

    fn chosen(&self, document: &Path, _offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        panic!("verify no elige certificado para {}", document.display())
    }
}

impl SignatureReader for Untouched {
    fn signatures_in(&self, document: &[u8]) -> Result<DocumentSignatures, BridgeError> {
        panic!(
            "verify sin -v no lee las firmas de {} bytes",
            document.len()
        )
    }
}

impl LocalTimeZone for Untouched {
    fn offset_at(&self, instant: DateTime<Utc>) -> FixedOffset {
        panic!("verify sin -v no pasa a hora local {instant}")
    }
}

impl DocumentSigner for Untouched {
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        panic!("verify no firma {}", request.input.display())
    }

    fn remember(&self, _certificate: &TokenCertificate) {
        panic!("verify no recuerda certificados")
    }

    fn remembered(&self) -> Option<CertificateRef> {
        panic!("verify no propone certificados")
    }
}

fn verified(words: &[&str], files: &dyn CommandLineFiles, verifier: &Answering) -> Outcome {
    attended(words, files, verifier, &Untouched, &Untouched)
}

fn verified_reading_with(words: &[&str], reader: &dyn SignatureReader) -> Outcome {
    let verifier = Answering::with(&["Firma valida"]);
    attended(words, &OneFile(A_PDF), &verifier, reader, &SummerInMadrid)
}

fn verified_reading(words: &[&str], reader: &Reading) -> Outcome {
    let verifier = Answering::with(&["Firma valida"]);
    attended(words, &OneFile(A_PDF), &verifier, reader, &SummerInMadrid)
}

fn attended(
    words: &[&str],
    files: &dyn CommandLineFiles,
    verifier: &Answering,
    reader: &dyn SignatureReader,
    time_zone: &dyn LocalTimeZone,
) -> Outcome {
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    attend(
        &arguments,
        &CommandLinePorts {
            stores: &Untouched,
            terminal: &Untouched,
            descriptor: &Untouched,
            desktop: &Untouched,
            filter: &Untouched,
            files,
            verifier,
            reader,
            time_zone,
            signer: &Untouched,
            window: &Untouched,
        },
    )
}

fn a_signature(name: &str, id_number: &str, signing_time: Option<&str>) -> DocumentSignature {
    DocumentSignature {
        name: name.to_owned(),
        id_number: id_number.to_owned(),
        organization_identifier: None,
        organization_name: None,
        issuer: "AC FNMT Usuarios".to_owned(),
        certificate_serial_number: "0123ABCD".to_owned(),
        signing_time: signing_time.map(str::to_owned),
        status: Some(SignatureStatus::Valid),
        reason: None,
        validity: Validity::Valid,
        validity_reason: None,
        signing_date: None,
        closes_document: false,
        countersignatures: Vec::new(),
    }
}

fn printed(outcome: &Outcome) -> String {
    String::from_utf8(outcome.stdout.clone()).expect("la salida es UTF-8")
}

#[test]
fn verify_prints_one_line_per_validity_result_and_ends_with_zero() {
    let verifier = Answering::with(&["Firma valida", "Validación incompleta: sin sello"]);

    let outcome = verified(&["verify", "-i", "firmado.pdf"], &OneFile(A_PDF), &verifier);

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(
        printed(&outcome),
        "Firma valida\nValidación incompleta: sin sello\n"
    );
    assert!(outcome.stderr.is_empty(), "{:?}", outcome.stderr);
}

#[test]
fn an_invalid_signature_still_ends_with_zero_like_the_original() {
    let verifier = Answering::with(&["Firma no valida: existe un certificado de firma caducado"]);

    let outcome = verified(&["VERIFY", "-i", "firmado.pdf"], &OneFile(A_PDF), &verifier);

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(
        printed(&outcome),
        "Firma no valida: existe un certificado de firma caducado\n"
    );
}

#[test]
fn data_of_no_signature_format_print_their_result_without_asking_the_validator() {
    let verifier = Answering::with(&["Firma valida"]);

    let outcome = verified(
        &["verify", "-i", "foto.png"],
        &OneFile(b"\x89PNG\r\n"),
        &verifier,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED);
    assert_eq!(printed(&outcome), format!("{UNKNOWN_FORMAT}\n"));
    assert!(verifier.asked.borrow().is_empty());
}

#[test]
fn verify_asks_the_validator_of_the_format_it_detects() {
    let verifier = Answering::with(&["Firma valida"]);

    verified(&["verify", "-i", "firmado.pdf"], &OneFile(A_PDF), &verifier);

    assert_eq!(*verifier.asked.borrow(), [Format::Pades]);
}

#[test]
fn the_format_is_the_one_auto_detects_and_a_cms_signature_is_validated_as_cades() {
    assert_eq!(format_to_verify(A_PDF), Some(Format::Pades));
    assert_eq!(
        format_to_verify(b"<?xml version=\"1.0\"?><a/>"),
        Some(Format::Xades(XadesVariant::Enveloping))
    );
    assert_eq!(format_to_verify(A_CMS), Some(Format::Cades));
    assert_eq!(format_to_verify(b""), None);
}

#[test]
fn verify_without_an_input_file_is_refused() {
    let outcome = verified(&["verify"], &OneFile(A_PDF), &Answering::with(&[]));

    assert_eq!(outcome.exit_code, REFUSED);
    assert!(
        outcome.stderr.join("\n").contains("-i"),
        "{:?}",
        outcome.stderr
    );
}

#[test]
fn an_input_file_that_cannot_be_read_fails_naming_it() {
    let outcome = verified(
        &["verify", "-i", "no-esta.pdf"],
        &NoFile,
        &Answering::with(&[]),
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
    assert!(
        outcome.stderr.join("\n").contains("no-esta.pdf"),
        "{:?}",
        outcome.stderr
    );
}

#[test]
fn a_validator_that_cannot_answer_fails_instead_of_printing_a_result() {
    let outcome = verified(
        &["verify", "-i", "firmado.pdf"],
        &OneFile(A_PDF),
        &Answering::failing(),
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
    assert!(
        outcome.stderr.join("\n").contains("el isolate no arranca"),
        "{:?}",
        outcome.stderr
    );
}

#[test]
fn verify_in_xml_is_not_available_yet() {
    let outcome = verified(
        &["verify", "-i", "firmado.pdf", "-xml"],
        &OneFile(A_PDF),
        &Answering::with(&["Firma valida"]),
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
}

mod verbose;
