use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, FixedOffset, Utc};

use super::*;
use crate::desktop::domain::command_line::WindowIntent;
use crate::desktop::domain::sign_arguments::Algorithm;
use crate::desktop::ports::{
    AskedSecret, LocalTimeZone, OfferedCertificate, SecretDescriptor, SignatureReading,
    WindowChoice, WindowOffer,
};
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::store::Store;
use crate::signing::domain::bridge::{BridgeError, Format, SignatureOperation};
use crate::signing::domain::DocumentSignatures;
use crate::site::domain::protocol::SiteFilter;

struct StoresWith {
    labels: Vec<&'static str>,
    opened: Cell<bool>,
}

impl StoresWith {
    fn labels(labels: &[&'static str]) -> Self {
        Self {
            labels: labels.to_vec(),
            opened: Cell::new(false),
        }
    }
}

impl CertificateStores for StoresWith {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        self.opened.set(true);
        Ok(self
            .labels
            .iter()
            .map(|label| {
                TokenCertificate::new(
                    CertificateRef::new(Store::module("/modulo.so"), "token", *label, None),
                    Vec::new(),
                )
            })
            .collect())
    }

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        (library == "/modulo.so").then(|| PathBuf::from(library))
    }
}

struct NoStoreOpens;

impl CertificateStores for NoStoreOpens {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Err(TokenError::new(
            Situation::ModuleNotFound,
            "no hay ningun modulo PKCS#11",
        ))
    }

    fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
        None
    }
}

struct ScriptedTerminal;

impl Terminal for ScriptedTerminal {
    fn is_interactive(&self) -> bool {
        false
    }

    fn secret(&self, asked: &AskedSecret<'_>) -> Result<ProtectedSecret, String> {
        panic!(
            "estas pruebas no piden el {:?} de {}",
            asked.name, asked.alias
        )
    }

    fn chosen(&self, offered: &[OfferedCertificate], _preselected: usize) -> Result<usize, String> {
        panic!(
            "estas pruebas no eligen entre {} certificados",
            offered.len()
        )
    }
}

struct ScriptedDescriptor;

impl SecretDescriptor for ScriptedDescriptor {
    fn read(&self, descriptor: u32) -> Result<ProtectedSecret, String> {
        panic!("estas pruebas no leen el descriptor {descriptor}")
    }
}

fn arguments_of(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

fn attended_by(
    words: &[&str],
    stores: &dyn CertificateStores,
    desktop: &RecordingDesktop,
) -> Outcome {
    attended_in(
        words,
        stores,
        desktop,
        &FilesInMemory::default(),
        &RecordingSigner::default(),
    )
}

fn attended_in(
    words: &[&str],
    stores: &dyn CertificateStores,
    desktop: &RecordingDesktop,
    files: &FilesInMemory,
    signer: &RecordingSigner,
) -> Outcome {
    let ports = CommandLinePorts {
        stores,
        terminal: &ScriptedTerminal,
        descriptor: &ScriptedDescriptor,
        desktop,
        filter: &Untouched,
        files,
        verifier: &Untouched,
        reader: &Untouched,
        time_zone: &Untouched,
        language: crate::signing::domain::Language::Spanish,
        signer,
        window: &Untouched,
    };
    attend(&arguments_of(words), &ports)
}

#[derive(Default)]
struct FilesInMemory {
    files: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
}

impl FilesInMemory {
    fn with(path: &str, bytes: &[u8]) -> Self {
        let files = Self::default();
        files
            .files
            .borrow_mut()
            .insert(PathBuf::from(path), bytes.to_vec());
        files
    }

    fn at(&self, path: &str) -> Option<Vec<u8>> {
        self.files.borrow().get(Path::new(path)).cloned()
    }
}

impl CommandLineFiles for FilesInMemory {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| "no existe".to_owned())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }
}

#[derive(Default)]
struct RecordingSigner {
    asked: RefCell<Vec<(PathBuf, String, SignatureFormat, Algorithm)>>,
    remembered: RefCell<Vec<String>>,
    parameters: RefCell<Vec<BTreeMap<String, String>>>,
    descriptors: RefCell<Vec<Option<u32>>>,
    window_secrets: RefCell<Vec<Option<Vec<u8>>>>,
    operations: RefCell<Vec<SignatureOperation>>,
    fails: bool,
    recalled: Option<CertificateRef>,
}

impl DocumentSigner for RecordingSigner {
    fn sign(&self, request: &CommandLineSigning<'_>) -> Result<Vec<u8>, String> {
        self.descriptors.borrow_mut().push(request.password_fd);
        self.window_secrets.borrow_mut().push(
            request
                .typed_in_the_window
                .map(|secret| secret.as_bytes().to_vec()),
        );
        self.asked.borrow_mut().push((
            request.input.to_path_buf(),
            request.certificate.reference().label().to_owned(),
            request.format,
            request.algorithm,
        ));
        self.operations.borrow_mut().push(request.operation);
        self.parameters
            .borrow_mut()
            .push(request.parameters.clone());
        if self.fails {
            return Err("el token no firma".to_owned());
        }
        Ok(SIGNED.to_vec())
    }

    fn remember(&self, certificate: &TokenCertificate) {
        self.remembered
            .borrow_mut()
            .push(certificate.reference().label().to_owned());
    }

    fn remembered(&self) -> Option<CertificateRef> {
        self.recalled.clone()
    }
}

const A_PDF: &[u8] = b"%PDF-1.4 cuerpo";
const SIGNED: &[u8] = b"%PDF-1.4 firmado";

fn attended_with(words: &[&str], stores: &dyn CertificateStores) -> Outcome {
    attended_by(words, stores, &RecordingDesktop::default())
}

fn handed_over_with(desktop: &RecordingDesktop, words: &[&str]) -> Outcome {
    attended_by(words, &StoresWith::labels(&[]), desktop)
}

#[derive(Default)]
struct RecordingDesktop {
    delivered: RefCell<Vec<(PathBuf, WindowIntent)>>,
    fails: bool,
}

impl DesktopHandover for RecordingDesktop {
    fn hand_over(&self, file: &Path, intent: WindowIntent) -> Result<(), String> {
        if self.fails {
            return Err("sin ejecutable".to_owned());
        }
        self.delivered
            .borrow_mut()
            .push((file.to_path_buf(), intent));
        Ok(())
    }
}

impl CertificateFilter for Untouched {
    fn accepted(
        &self,
        _filter: &SiteFilter,
        _certificates: Vec<TokenCertificate>,
    ) -> Result<Vec<TokenCertificate>, String> {
        panic!("no debería filtrar certificados")
    }
}

/// El validador y la ventana que una orden que no los necesita no debería llegar a tocar.
struct Untouched;

impl GraphicalPicker for Untouched {
    fn has_a_display(&self) -> bool {
        panic!("no debería abrir la ventana de sede")
    }

    fn chosen(&self, document: &Path, _offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        panic!(
            "no debería elegir en la ventana para {}",
            document.display()
        )
    }
}

impl SignatureVerifier for Untouched {
    fn results_of(&self, _document: &[u8], format: Format) -> Result<Vec<String>, BridgeError> {
        panic!("no debería validar en {format}")
    }
}

impl SignatureReading for Untouched {
    fn signatures_in(&self, document: &[u8]) -> Result<DocumentSignatures, String> {
        panic!("no debería leer las firmas de {} bytes", document.len())
    }
}

impl LocalTimeZone for Untouched {
    fn offset_at(&self, instant: DateTime<Utc>) -> FixedOffset {
        panic!("no debería pasar a hora local {instant}")
    }
}

mod certgui;
mod certtui;
mod cosign;
mod filter_and_xml;
mod json;
mod listaliases;
mod listaliases_json;
#[path = "../../../../tests/command_line/schema.rs"]
mod schema;
mod sign_config;
mod sign_formats;
mod sign_json;

fn attended(words: &[&str]) -> Outcome {
    attended_with(words, &StoresWith::labels(&[]))
}

fn said(outcome: &Outcome) -> String {
    outcome.stderr.join("\n")
}

#[test]
fn countersign_and_batchsign_end_with_a_clear_refusal_and_a_nonzero_code() {
    for command in ["countersign", "batchsign"] {
        let outcome = attended(&[command, "-i", "a.pdf", "-o", "b.pdf"]);

        assert_ne!(outcome.exit_code, SUCCEEDED);
        assert!(outcome.stdout.is_empty());
        assert!(said(&outcome).contains(command), "{}", said(&outcome));
    }
}

#[test]
fn each_parameter_left_out_ends_with_a_refusal_that_names_it() {
    for parameter in ["-preurl", "-posturl", "-hformat", "-halgorithm", "-r"] {
        let outcome = attended(&["sign", "-i", "a.pdf", "-o", "b.pdf", parameter, "x"]);

        assert_eq!(outcome.exit_code, REFUSED, "{parameter}");
        assert!(outcome.stdout.is_empty());
        assert!(said(&outcome).contains(parameter), "{}", said(&outcome));
    }
}

#[test]
fn the_password_is_refused_in_any_position_with_a_message_naming_password_fd() {
    for words in [
        &["sign", "-password", "1234", "-i", "a.pdf"][..],
        &["listaliases", "-store", "pkcs11", "-password", "1234"][..],
        &["sign", "-help", "-password", "1234"][..],
    ] {
        let outcome = attended(words);

        assert_eq!(outcome.exit_code, REFUSED);
        assert!(outcome.stdout.is_empty());
        assert!(
            said(&outcome).contains("-password-fd"),
            "{}",
            said(&outcome)
        );
        assert!(!said(&outcome).contains("1234"), "nunca repite el secreto");
    }
}

#[test]
fn each_command_gives_its_syntax_on_stdout_with_help() {
    for command in ["sign", "COSIGN", "listaliases", "verify"] {
        let outcome = attended(&[command, "-help"]);

        assert_eq!(outcome.exit_code, SUCCEEDED);
        assert!(outcome.stderr.is_empty());
        let syntax = String::from_utf8(outcome.stdout).expect("UTF-8");
        assert!(
            syntax.contains(&format!("rfirma {}", command.to_lowercase())),
            "{syntax}"
        );
    }
}

#[test]
fn a_parameter_not_yet_available_fails_with_a_clear_message_and_an_empty_stdout() {
    let outcome = attended(&["cosign", "-i", "a.pdf", "-o", "b.pdf", "-certtui"]);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
    assert!(said(&outcome).contains("-certtui"), "{}", said(&outcome));
}

#[test]
fn invalid_sign_arguments_are_refused_before_anything_else_with_a_nonzero_code() {
    for words in [
        &["sign", "-i", "a.pdf", "-alias", "yo"][..],
        &[
            "cosign",
            "-i",
            "a.pdf",
            "-o",
            "b.pdf",
            "-alias",
            "yo",
            "-algorithm",
            "sha1",
        ][..],
        &[
            "sign", "-i", "a.pdf", "-o", "b.pdf", "-alias", "yo", "-format", "facturae",
        ][..],
        &["sign", "-i", "a.pdf", "-o", "b.pdf"][..],
        &[
            "sign", "-i", "a.pdf", "-o", "b.pdf", "-alias", "yo", "-certtui",
        ][..],
    ] {
        let outcome = attended(words);

        assert_eq!(outcome.exit_code, REFUSED, "{words:?}");
        assert!(outcome.stdout.is_empty());
    }
}

#[test]
fn sign_with_gui_hands_the_file_to_the_desktop_just_to_open_it() {
    let desktop = RecordingDesktop::default();

    let outcome = handed_over_with(&desktop, &["sign", "-gui", "-i", "doc.pdf"]);

    assert_eq!(outcome, Outcome::default());
    assert_eq!(
        *desktop.delivered.borrow(),
        vec![(PathBuf::from("doc.pdf"), WindowIntent::OpenTheDocument)]
    );
}

#[test]
fn verify_with_gui_hands_the_file_to_the_desktop_to_see_its_signatures() {
    for words in [
        ["verify", "-gui", "-i", "doc.pdf"],
        ["verify", "--gui", "-i", "doc.pdf"],
    ] {
        let desktop = RecordingDesktop::default();

        let outcome = handed_over_with(&desktop, &words);

        assert_eq!(outcome, Outcome::default(), "{words:?}");
        assert_eq!(
            *desktop.delivered.borrow(),
            vec![(PathBuf::from("doc.pdf"), WindowIntent::SeeItsSignatures)],
            "{words:?}"
        );
    }
}

#[test]
fn gui_without_an_input_is_refused_and_delivers_nothing() {
    let desktop = RecordingDesktop::default();

    let outcome = handed_over_with(&desktop, &["sign", "-gui"]);

    assert_eq!(outcome.exit_code, REFUSED);
    assert!(said(&outcome).contains("-i"), "{}", said(&outcome));
    assert!(desktop.delivered.borrow().is_empty());
}

#[test]
fn a_failed_delivery_ends_with_a_nonzero_code_and_the_reason() {
    let desktop = RecordingDesktop {
        fails: true,
        ..RecordingDesktop::default()
    };

    let outcome = handed_over_with(&desktop, &["verify", "-gui", "-i", "doc.pdf"]);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("sin ejecutable"));
}

fn signed_over(words: &[&str], files: &FilesInMemory, signer: &RecordingSigner) -> Outcome {
    attended_in(
        words,
        &StoresWith::labels(&["otro", "yo"]),
        &RecordingDesktop::default(),
        files,
        signer,
    )
}

#[test]
fn sign_with_an_alias_writes_exactly_in_the_output_and_says_so_only_on_stderr() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner::default();

    let outcome = signed_over(
        &["sign", "-i", "doc.pdf", "-o", "firmado.pdf", "-alias", "yo"],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert!(outcome.stdout.is_empty());
    assert!(said(&outcome).contains("firmado.pdf"), "{}", said(&outcome));
    assert_eq!(files.at("firmado.pdf").as_deref(), Some(SIGNED));
    assert_eq!(
        *signer.asked.borrow(),
        vec![(
            PathBuf::from("doc.pdf"),
            "yo".to_owned(),
            SignatureFormat::Pades,
            Algorithm::Sha512
        )]
    );
    assert_eq!(*signer.remembered.borrow(), vec!["yo".to_owned()]);
}

#[test]
fn sign_with_pades_and_another_algorithm_asks_for_exactly_that() {
    for (word, algorithm) in [("sha256", Algorithm::Sha256), ("sha384", Algorithm::Sha384)] {
        let files = FilesInMemory::with("doc.pdf", A_PDF);
        let signer = RecordingSigner::default();

        let outcome = signed_over(
            &[
                "sign",
                "-i",
                "doc.pdf",
                "-o",
                "f.pdf",
                "-alias",
                "yo",
                "-format",
                "pades",
                "-algorithm",
                word,
            ],
            &files,
            &signer,
        );

        assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
        let asked = signer.asked.borrow();
        assert_eq!(
            (asked[0].2, asked[0].3),
            (SignatureFormat::Pades, algorithm)
        );
    }
}

#[test]
fn sign_overwrites_what_was_already_at_the_output() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    files
        .write(Path::new("firmado.pdf"), b"lo de antes")
        .expect("en memoria se escribe");

    let outcome = signed_over(
        &["sign", "-i", "doc.pdf", "-o", "firmado.pdf", "-alias", "yo"],
        &files,
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(files.at("firmado.pdf").as_deref(), Some(SIGNED));
}

#[test]
fn sign_with_an_alias_no_store_has_fails_without_signing_or_writing() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner::default();

    let outcome = signed_over(
        &["sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "nadie"],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("nadie"), "{}", said(&outcome));
    assert!(signer.asked.borrow().is_empty());
    assert_eq!(files.at("f.pdf"), None);
}

#[test]
fn a_signature_that_fails_writes_nothing_and_remembers_nothing() {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let signer = RecordingSigner {
        fails: true,
        ..RecordingSigner::default()
    };

    let outcome = signed_over(
        &["sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo"],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(
        said(&outcome).contains("el token no firma"),
        "{}",
        said(&outcome)
    );
    assert_eq!(files.at("f.pdf"), None);
    assert!(signer.remembered.borrow().is_empty());
}

#[test]
fn an_input_that_cannot_be_read_fails_without_signing() {
    let signer = RecordingSigner::default();

    let outcome = signed_over(
        &["sign", "-i", "no-esta.pdf", "-o", "f.pdf", "-alias", "yo"],
        &FilesInMemory::default(),
        &signer,
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("no-esta.pdf"), "{}", said(&outcome));
    assert!(signer.asked.borrow().is_empty());
}

#[test]
fn sign_hands_the_password_descriptor_to_the_signer_without_reading_it() {
    let signer = RecordingSigner::default();

    let outcome = attended_in(
        &[
            "sign",
            "-i",
            "doc.pdf",
            "-o",
            "f.pdf",
            "-alias",
            "yo",
            "-password-fd",
            "3",
        ],
        &StoresWith::labels(&["yo"]),
        &RecordingDesktop::default(),
        &FilesInMemory::with("doc.pdf", A_PDF),
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert_eq!(*signer.descriptors.borrow(), vec![Some(3)]);
}

#[test]
fn verbose_is_unknown_outside_verify() {
    for words in [
        &["sign", "-i", "a.pdf", "-o", "b.pdf", "--alias", "yo", "-v"][..],
        &[
            "sign",
            "-i",
            "a.pdf",
            "-o",
            "b.pdf",
            "--alias",
            "yo",
            "--verbose",
        ][..],
        &[
            "cosign", "-i", "a.pdf", "-o", "b.pdf", "--alias", "yo", "-v",
        ][..],
        &[
            "cosign",
            "-i",
            "a.pdf",
            "-o",
            "b.pdf",
            "--alias",
            "yo",
            "--verbose",
        ][..],
        &["listaliases", "-v"][..],
        &["listaliases", "--verbose"][..],
        &["listaliases", "-vv"][..],
    ] {
        let outcome = attended(words);

        assert_eq!(outcome.exit_code, REFUSED, "{words:?}");
        assert!(outcome.stdout.is_empty(), "{words:?}");
        assert!(
            said(&outcome).contains("no se reconoce"),
            "{words:?}: {}",
            said(&outcome)
        );
    }
}
