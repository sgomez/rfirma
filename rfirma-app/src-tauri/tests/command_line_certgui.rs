//! `sign -certgui` firma con el certificado y el PIN que devuelve la ventana de sede, aquí un doble del elector gráfico.

#[path = "command_line/support.rs"]
mod support;

use support::*;

/// El elector gráfico de las pruebas: elige el certificado activo del kit y devuelve ese PIN.
struct ChoosingTheActiveCertificate {
    pin: &'static str,
    offered: RefCell<Vec<String>>,
}

impl ChoosingTheActiveCertificate {
    fn typing(pin: &'static str) -> Self {
        Self {
            pin,
            offered: RefCell::new(Vec::new()),
        }
    }
}

impl GraphicalPicker for ChoosingTheActiveCertificate {
    fn has_a_display(&self) -> bool {
        true
    }

    fn chosen(&self, _document: &Path, offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        let WindowOffer::Certificates(certificates) = offer else {
            return Ok(WindowChoice::Cancelled);
        };
        self.offered.borrow_mut().extend(
            certificates
                .iter()
                .map(|certificate| certificate.reference().label().to_owned()),
        );
        let active = certificates
            .iter()
            .find(|certificate| certificate.reference().label() == CARD_ACTIVE)
            .ok_or("el certificado activo del kit deberia ofrecerse")?;
        Ok(WindowChoice::Chosen {
            certificate: active.clone(),
            secret: Some(ProtectedSecret::from(self.pin)),
        })
    }
}

fn signed_in_the_window(
    home: &Path,
    roots: &Roots,
    window: &ChoosingTheActiveCertificate,
) -> (Outcome, PathBuf) {
    let input = home.join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.join("firmado.pdf");
    let store = format!("pkcs11:{}", the_card_module().display());
    let words = [
        "sign",
        "-i",
        input.to_str().expect("ruta UTF-8"),
        "-o",
        output.to_str().expect("ruta UTF-8"),
        "-certgui",
        "-store",
        &store,
    ];
    let outcome =
        attended_with_the_window(&words, roots, &ScriptedTerminal::without_a_tty(), window);
    (outcome, output)
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn certgui_signs_with_the_certificate_and_the_pin_chosen_in_the_window() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let window = ChoosingTheActiveCertificate::typing(KIT_PASSWORD);

    let (outcome, output) = signed_in_the_window(home.path(), &roots, &window);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert!(window.offered.borrow().contains(&CARD_ACTIVE.to_owned()));
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn certgui_with_a_wrong_pin_from_the_window_fails_without_writing() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let window = ChoosingTheActiveCertificate::typing("0000");

    let (outcome, output) = signed_in_the_window(home.path(), &roots, &window);

    assert_eq!(outcome.exit_code, FAILED, "{:?}", outcome.stderr);
    assert!(!output.exists());
}
