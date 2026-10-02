//! `sign` pide el PIN al descriptor, a la TTY o al diálogo de escritorio.

#[path = "command_line/support.rs"]
mod support;

use support::*;

fn signed_on_the_card(home: &Path, terminal: &ScriptedTerminal) -> (Outcome, Roots, PathBuf) {
    signed_on_the_card_with_the_dialog(home, terminal, MockSecretPrompter::new())
}

fn signed_on_the_card_with_the_dialog(
    home: &Path,
    terminal: &ScriptedTerminal,
    dialog: MockSecretPrompter,
) -> (Outcome, Roots, PathBuf) {
    let mut roots = the_roots_under(home);
    roots.identity.prompter = Arc::new(dialog);
    let input = home.join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.join("firmado.pdf");
    let outcome = attended_with_the_roots(
        &[
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-alias",
            CARD_ACTIVE,
            "-store",
            &format!("pkcs11:{}", the_card_module().display()),
        ],
        &roots,
        terminal,
    );
    (outcome, roots, output)
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_on_the_test_token_asks_the_pin_on_the_tty_and_again_when_it_is_wrong() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let terminal = ScriptedTerminal::typing(&["0000", KIT_PASSWORD]);

    let (outcome, roots, output) = signed_on_the_card(home.path(), &terminal);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert_eq!(terminal.retries_asked(), vec![false, true]);
    assert!(outcome.stdout.is_empty());
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_on_the_test_token_fails_when_the_pin_is_not_typed() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let terminal = ScriptedTerminal::typing(&[]);

    let (outcome, roots, output) = signed_on_the_card(home.path(), &terminal);

    assert_eq!(outcome.exit_code, FAILED, "{:?}", outcome.stderr);
    assert_eq!(terminal.retries_asked(), vec![false]);
    assert!(outcome.stdout.is_empty());
    assert!(!output.exists());
    assert_eq!(roots.identity.remembered_certificate(), None);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_on_the_test_token_without_a_tty_asks_the_desktop_dialog_and_again_when_it_is_wrong() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let dialog = MockSecretPrompter::with_secrets(&["0000", KIT_PASSWORD]);

    let (outcome, roots, output) =
        signed_on_the_card_with_the_dialog(home.path(), &ScriptedTerminal::without_a_tty(), dialog);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_on_the_test_token_fails_when_the_desktop_dialog_is_cancelled() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();

    let (outcome, _roots, output) = signed_on_the_card_with_the_dialog(
        home.path(),
        &ScriptedTerminal::without_a_tty(),
        MockSecretPrompter::with_responses(vec![Err(SecretPromptError::Failed(
            "sin entorno gráfico disponible".to_owned(),
        ))]),
    );

    assert_eq!(outcome.exit_code, FAILED, "{:?}", outcome.stderr);
    assert!(outcome.stderr.concat().contains("-password-fd"));
    assert!(!output.exists());
}
