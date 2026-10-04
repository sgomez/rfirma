use super::*;
use crate::desktop::ports::NoCertificateToOffer;
use crate::identity::application::tests::{a_usable_certificate, an_expired_certificate};

/// Lo que la ventana doblada ha enseñado: el documento y los certificados ofrecidos o la falta.
#[derive(Debug, PartialEq, Eq)]
enum Shown {
    Certificates(PathBuf, Vec<String>),
    Nothing(PathBuf, NoCertificateToOffer),
}

/// El doble del elector gráfico: contesta lo que diga su guion y apunta lo que se le ha enseñado.
struct ScriptedWindow {
    display: bool,
    answer: Result<Option<(usize, Option<&'static str>)>, &'static str>,
    shown: RefCell<Vec<Shown>>,
}

impl ScriptedWindow {
    fn choosing(position: usize, pin: Option<&'static str>) -> Self {
        Self {
            display: true,
            answer: Ok(Some((position, pin))),
            shown: RefCell::new(Vec::new()),
        }
    }

    fn cancelling() -> Self {
        Self {
            answer: Ok(None),
            ..Self::choosing(0, None)
        }
    }

    fn failing() -> Self {
        Self {
            answer: Err("sin WebKit"),
            ..Self::choosing(0, None)
        }
    }

    fn without_a_display() -> Self {
        Self {
            display: false,
            ..Self::choosing(0, None)
        }
    }
}

impl GraphicalPicker for ScriptedWindow {
    fn has_a_display(&self) -> bool {
        self.display
    }

    fn chosen(&self, document: &Path, offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        let document = document.to_path_buf();
        let offered = match offer {
            WindowOffer::Certificates(certificates) => {
                let labels = certificates
                    .iter()
                    .map(|certificate| certificate.reference().label().to_owned())
                    .collect();
                self.shown
                    .borrow_mut()
                    .push(Shown::Certificates(document, labels));
                certificates.to_vec()
            }
            WindowOffer::Nothing(nothing) => {
                self.shown
                    .borrow_mut()
                    .push(Shown::Nothing(document, nothing));
                Vec::new()
            }
        };
        let Some((position, pin)) = self.answer.map_err(str::to_owned)? else {
            return Ok(WindowChoice::Cancelled);
        };
        Ok(WindowChoice::Chosen {
            certificate: Box::new(offered[position].clone()),
            secret: pin.map(ProtectedSecret::from),
        })
    }
}

struct StoresHolding(Vec<TokenCertificate>);

impl CertificateStores for StoresHolding {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.0.clone())
    }

    fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
        None
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

const CERTGUI: [&str; 6] = ["sign", "-i", "doc.pdf", "-o", "f.pdf", "-certgui"];

fn chosen_with(
    words: &[&str],
    certificates: Vec<TokenCertificate>,
    filter: &dyn CertificateFilter,
    window: &ScriptedWindow,
    signer: &RecordingSigner,
) -> Outcome {
    let files = FilesInMemory::with("doc.pdf", A_PDF);
    let ports = CommandLinePorts {
        stores: &StoresHolding(certificates),
        terminal: &ScriptedTerminal,
        descriptor: &ScriptedDescriptor,
        desktop: &RecordingDesktop::default(),
        filter,
        files: &files,
        verifier: &Untouched,
        reader: &Untouched,
        time_zone: &Untouched,
        language: crate::signing::domain::Language::Spanish,
        signer,
        window,
    };
    attend(&arguments_of(words), &ports)
}

fn chosen_among(
    certificates: Vec<TokenCertificate>,
    window: &ScriptedWindow,
    signer: &RecordingSigner,
) -> Outcome {
    chosen_with(&CERTGUI, certificates, &Untouched, window, signer)
}

fn the_document() -> PathBuf {
    PathBuf::from("doc.pdf")
}

#[test]
fn certgui_signs_with_the_certificate_and_the_pin_chosen_in_the_window() {
    let window = ScriptedWindow::choosing(1, Some("1234"));
    let signer = RecordingSigner::default();

    let outcome = chosen_among(
        vec![a_usable_certificate("uno"), a_usable_certificate("dos")],
        &window,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(
        *window.shown.borrow(),
        vec![Shown::Certificates(
            the_document(),
            vec!["uno".to_owned(), "dos".to_owned()]
        )]
    );
    assert_eq!(signer.asked.borrow()[0].1, "dos");
    assert_eq!(
        *signer.window_secrets.borrow(),
        vec![Some(b"1234".to_vec())]
    );
    assert_eq!(*signer.remembered.borrow(), vec!["dos".to_owned()]);
}

#[test]
fn certgui_json_names_the_chosen_certificate_by_alias_with_its_common_fields() {
    let mut words = CERTGUI.to_vec();
    words.push("-json");

    let outcome = chosen_with(
        &words,
        vec![a_usable_certificate("uno"), a_usable_certificate("dos")],
        &Untouched,
        &ScriptedWindow::choosing(1, Some("1234")),
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    let signed = super::schema::conforming_json("sign", &outcome.stdout);
    assert_eq!(signed["certificate"]["alias"], "dos");
    assert!(signed["certificate"]["notAfter"].is_string());
}

#[test]
fn certgui_with_a_store_that_needs_no_pin_signs_without_a_secret_from_the_window() {
    let window = ScriptedWindow::choosing(0, None);
    let signer = RecordingSigner::default();

    let outcome = chosen_among(vec![a_usable_certificate("uno")], &window, &signer);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(*signer.window_secrets.borrow(), vec![None]);
}

#[test]
fn certgui_offers_neither_the_expired_nor_what_the_filter_leaves_out() {
    let window = ScriptedWindow::choosing(0, Some("1234"));
    let signer = RecordingSigner::default();

    let outcome = chosen_with(
        &[
            "sign",
            "-i",
            "doc.pdf",
            "-o",
            "f.pdf",
            "-certgui",
            "-filter",
            "nonexpired:",
        ],
        vec![
            a_usable_certificate("uno"),
            an_expired_certificate("caducado"),
            a_usable_certificate("dos"),
        ],
        &AcceptingLabels(vec!["dos", "caducado"]),
        &window,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(
        *window.shown.borrow(),
        vec![Shown::Certificates(the_document(), vec!["dos".to_owned()])]
    );
}

#[test]
fn certgui_cancelled_in_the_window_fails_without_signing_or_remembering() {
    let window = ScriptedWindow::cancelling();
    let signer = RecordingSigner::default();

    let outcome = chosen_among(vec![a_usable_certificate("uno")], &window, &signer);

    assert_ne!(outcome.exit_code, SUCCEEDED);
    assert!(outcome.stdout.is_empty());
    assert!(said(&outcome).contains("cancelado"), "{}", said(&outcome));
    assert!(signer.asked.borrow().is_empty());
    assert!(signer.remembered.borrow().is_empty());
}

#[test]
fn certgui_without_a_graphical_environment_fails_clearly_before_opening_anything() {
    let window = ScriptedWindow::without_a_display();
    let signer = RecordingSigner::default();

    let outcome = chosen_among(vec![a_usable_certificate("uno")], &window, &signer);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(
        said(&outcome).contains("entorno gráfico"),
        "{}",
        said(&outcome)
    );
    assert!(said(&outcome).contains("-certtui"), "{}", said(&outcome));
    assert!(window.shown.borrow().is_empty());
    assert!(signer.asked.borrow().is_empty());
}

#[test]
fn certgui_whose_window_cannot_open_fails_with_the_reason() {
    let window = ScriptedWindow::failing();
    let signer = RecordingSigner::default();

    let outcome = chosen_among(vec![a_usable_certificate("uno")], &window, &signer);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(said(&outcome).contains("sin WebKit"), "{}", said(&outcome));
    assert!(signer.asked.borrow().is_empty());
}

#[test]
fn certgui_with_no_certificate_opens_the_window_saying_why_and_then_fails() {
    for (certificates, filter, why) in [
        (Vec::new(), None, NoCertificateToOffer::None),
        (
            vec![an_expired_certificate("a"), an_expired_certificate("b")],
            None,
            NoCertificateToOffer::AllExpired { owned: 2 },
        ),
        (
            vec![
                a_usable_certificate("a"),
                a_usable_certificate("b"),
                a_usable_certificate("c"),
            ],
            Some("nonexpired:"),
            NoCertificateToOffer::Excluded { owned: 3 },
        ),
    ] {
        let window = ScriptedWindow::cancelling();
        let signer = RecordingSigner::default();
        let mut words = CERTGUI.to_vec();
        words.extend(
            filter
                .map(|expression| ["-filter", expression])
                .iter()
                .flatten(),
        );

        let outcome = chosen_with(
            &words,
            certificates,
            &AcceptingLabels(Vec::new()),
            &window,
            &signer,
        );

        assert_eq!(outcome.exit_code, FAILED, "{why:?}");
        assert!(said(&outcome).contains("vigente"), "{}", said(&outcome));
        assert_eq!(
            *window.shown.borrow(),
            vec![Shown::Nothing(the_document(), why)]
        );
        assert!(signer.asked.borrow().is_empty());
    }
}

#[test]
fn certgui_excludes_alias_and_certtui_but_not_a_filter() {
    for extra in [&["-alias", "yo"][..], &["-certtui"][..]] {
        let mut words = CERTGUI.to_vec();
        words.extend_from_slice(extra);

        let outcome = attended(&words);

        assert_eq!(outcome.exit_code, REFUSED, "{extra:?}");
    }
}
