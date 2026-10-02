use super::super::certtui::store_of;
use super::*;
use crate::identity::application::tests::{a_usable_certificate, an_expired_certificate};
use crate::identity::domain::store::StoreClass;

struct StoresHolding {
    certificates: Vec<TokenCertificate>,
    opened: Cell<bool>,
}

impl StoresHolding {
    fn these(certificates: Vec<TokenCertificate>) -> Self {
        Self {
            certificates,
            opened: Cell::new(false),
        }
    }
}

impl CertificateStores for StoresHolding {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        self.opened.set(true);
        Ok(self.certificates.clone())
    }

    fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
        None
    }
}

/// La terminal que elige lo que diga su guion y apunta lo que se le ha enseñado.
struct ChoosingTerminal {
    interactive: bool,
    answer: Result<Option<usize>, &'static str>,
    shown: RefCell<Vec<(Vec<OfferedCertificate>, usize)>>,
}

impl ChoosingTerminal {
    fn taking(answer: Option<usize>) -> Self {
        Self {
            interactive: true,
            answer: Ok(answer),
            shown: RefCell::new(Vec::new()),
        }
    }

    fn cancelling() -> Self {
        Self {
            answer: Err("cancelado"),
            ..Self::taking(None)
        }
    }

    fn without_a_tty() -> Self {
        Self {
            interactive: false,
            ..Self::taking(None)
        }
    }
}

impl Terminal for ChoosingTerminal {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn secret(&self, asked: &AskedSecret<'_>) -> Result<ProtectedSecret, String> {
        panic!("el firmante de prueba no pide el PIN de {}", asked.alias)
    }

    fn chosen(&self, offered: &[OfferedCertificate], preselected: usize) -> Result<usize, String> {
        self.shown
            .borrow_mut()
            .push((offered.to_vec(), preselected));
        self.answer
            .map(|answer| answer.unwrap_or(preselected))
            .map_err(str::to_owned)
    }
}

/// El motor de filtros que deja pasar solo los certificados de esas etiquetas.
struct AcceptingLabels(Vec<&'static str>);

impl CertificateFilter for AcceptingLabels {
    fn accepted(
        &self,
        _filter: &SiteFilter,
        certificates: Vec<TokenCertificate>,
    ) -> Result<Vec<TokenCertificate>, String> {
        Ok(certificates
            .into_iter()
            .filter(|certificate| self.0.contains(&certificate.reference().label()))
            .collect())
    }
}

const CERTTUI: [&str; 6] = ["sign", "-i", "doc.pdf", "-o", "f.pdf", "-certtui"];

fn chosen_with(
    words: &[&str],
    stores: &StoresHolding,
    terminal: &ChoosingTerminal,
    filter: &dyn CertificateFilter,
    signer: &RecordingSigner,
) -> Outcome {
    let ports = CommandLinePorts {
        stores,
        terminal,
        descriptor: &ScriptedDescriptor,
        desktop: &RecordingDesktop::default(),
        filter,
        files: &FilesInMemory::with("doc.pdf", A_PDF),
        verifier: &Untouched,
        signer,
    };
    attend(&arguments_of(words), &ports)
}

fn chosen_among(
    stores: &StoresHolding,
    terminal: &ChoosingTerminal,
    signer: &RecordingSigner,
) -> Outcome {
    chosen_with(&CERTTUI, stores, terminal, &Untouched, signer)
}

#[test]
fn certtui_lists_only_the_usable_certificates_with_holder_issuer_expiry_and_store() {
    let stores = StoresHolding::these(vec![
        a_usable_certificate("uno"),
        an_expired_certificate("caducado"),
        a_usable_certificate("dos"),
    ]);
    let terminal = ChoosingTerminal::taking(Some(1));
    let signer = RecordingSigner::default();

    let outcome = chosen_among(&stores, &terminal, &signer);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    let shown = terminal.shown.borrow();
    let (offered, preselected) = &shown[0];
    assert_eq!(offered.len(), 2);
    assert_eq!(*preselected, 0);
    for certificate in offered {
        assert!(!certificate.holder.is_empty());
        assert!(!certificate.issuer.is_empty());
        assert_eq!(certificate.expires.len(), "2026-10-02".len());
        assert_eq!(certificate.store, "tarjeta «rfirma-test»");
    }
    assert_eq!(signer.asked.borrow()[0].1, "dos");
    assert_eq!(*signer.remembered.borrow(), vec!["dos".to_owned()]);
}

#[test]
fn certtui_preselects_the_remembered_certificate() {
    let second = a_usable_certificate("dos");
    let stores = StoresHolding::these(vec![a_usable_certificate("uno"), second.clone()]);
    let terminal = ChoosingTerminal::taking(None);
    let signer = RecordingSigner {
        recalled: Some(second.reference().clone()),
        ..RecordingSigner::default()
    };

    let outcome = chosen_among(&stores, &terminal, &signer);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(terminal.shown.borrow()[0].1, 1);
    assert_eq!(signer.asked.borrow()[0].1, "dos");
}

#[test]
fn certtui_without_a_tty_fails_before_opening_any_store() {
    let stores = StoresHolding::these(vec![a_usable_certificate("uno")]);
    let terminal = ChoosingTerminal::without_a_tty();
    let signer = RecordingSigner::default();

    let outcome = chosen_among(&stores, &terminal, &signer);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("-certtui"), "{}", said(&outcome));
    assert!(!stores.opened.get());
    assert!(terminal.shown.borrow().is_empty());
    assert!(signer.asked.borrow().is_empty());
}

#[test]
fn certtui_with_no_usable_certificate_fails_without_asking() {
    let stores = StoresHolding::these(vec![an_expired_certificate("caducado")]);
    let terminal = ChoosingTerminal::taking(None);
    let signer = RecordingSigner::default();

    let outcome = chosen_among(&stores, &terminal, &signer);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("vigente"), "{}", said(&outcome));
    assert!(terminal.shown.borrow().is_empty());
}

#[test]
fn certtui_cancelled_fails_without_signing_or_remembering() {
    let stores = StoresHolding::these(vec![a_usable_certificate("uno")]);
    let terminal = ChoosingTerminal::cancelling();
    let signer = RecordingSigner::default();

    let outcome = chosen_among(&stores, &terminal, &signer);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("cancelado"), "{}", said(&outcome));
    assert!(signer.asked.borrow().is_empty());
    assert!(signer.remembered.borrow().is_empty());
}

#[test]
fn certtui_answering_outside_the_list_fails_without_signing() {
    let stores = StoresHolding::these(vec![a_usable_certificate("uno")]);
    let terminal = ChoosingTerminal::taking(Some(5));
    let signer = RecordingSigner::default();

    let outcome = chosen_among(&stores, &terminal, &signer);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(signer.asked.borrow().is_empty());
}

#[test]
fn certtui_with_a_filter_lists_only_what_the_filter_accepts() {
    let stores = StoresHolding::these(vec![
        a_usable_certificate("uno"),
        a_usable_certificate("dos"),
    ]);
    let terminal = ChoosingTerminal::taking(None);
    let signer = RecordingSigner::default();
    let mut words = CERTTUI.to_vec();
    words.extend(["-filter", "nonexpired:"]);

    let outcome = chosen_with(
        &words,
        &stores,
        &terminal,
        &AcceptingLabels(vec!["dos"]),
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(terminal.shown.borrow()[0].0.len(), 1);
    assert_eq!(signer.asked.borrow()[0].1, "dos");
}

#[test]
fn certtui_with_a_filter_naming_no_criterion_fails_before_opening_any_store() {
    let stores = StoresHolding::these(vec![a_usable_certificate("uno")]);
    let terminal = ChoosingTerminal::taking(None);
    let mut words = CERTTUI.to_vec();
    words.extend(["-filter", "inventado:x"]);

    let outcome = chosen_with(
        &words,
        &stores,
        &terminal,
        &Untouched,
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(!stores.opened.get());
}

#[test]
fn certtui_with_the_nss_family_leaves_the_card_out_of_the_list() {
    let stores = StoresHolding::these(vec![a_usable_certificate("uno")]);
    let terminal = ChoosingTerminal::taking(None);
    let mut words = CERTTUI.to_vec();
    words.extend(["-store", "mozilla"]);

    let outcome = chosen_with(
        &words,
        &stores,
        &terminal,
        &Untouched,
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(terminal.shown.borrow().is_empty());
}

#[test]
fn each_store_class_is_named_in_the_list() {
    let reference = CertificateRef::new(Store::module("/modulo.so"), "token", "yo", None);

    for (class, name) in [
        (StoreClass::Card, "tarjeta «token»"),
        (StoreClass::Installed, "Almacén de rFirma"),
        (StoreClass::Firefox, "Firefox"),
        (StoreClass::Chrome, "Chrome"),
        (StoreClass::Nssdb, "NSS del sistema"),
        (StoreClass::Windows, "Windows"),
    ] {
        assert_eq!(store_of(class, &reference), name);
    }
}
