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

fn a_pipe_carrying(secret: &str) -> std::io::PipeReader {
    use std::io::Write;

    let (reader, mut writer) = std::io::pipe().expect("deberia poder abrirse un pipe");
    writer
        .write_all(secret.as_bytes())
        .expect("el PIN deberia entrar en el pipe");
    reader
}

fn the_rfirma_binary_signing_on_the_card(home: &Path, pin_on_fd_3: &str) -> (Output, PathBuf) {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;

    let input = home.join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.join("firmado.pdf");
    let pipe = a_pipe_carrying(pin_on_fd_3);
    let pipe_fd = pipe.as_raw_fd();

    let mut rfirma = Command::new(env!("CARGO_BIN_EXE_rfirma"));
    if let Some(configuration) = the_softhsm_configuration() {
        rfirma.env("SOFTHSM2_CONF", configuration);
    }
    rfirma
        .arg("sign")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .args(["-alias", CARD_ACTIVE])
        .args(["-store", &format!("pkcs11:{}", the_card_module().display())])
        .args(["-password-fd", "3"])
        .stdin(std::process::Stdio::null())
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_DATA_HOME", home.join("data"));
    // SAFETY: dup2 es async-signal-safe y es lo único que corre entre el fork y el exec.
    unsafe {
        rfirma.pre_exec(move || {
            let duplicated = if pipe_fd == 3 {
                libc::fcntl(3, libc::F_SETFD, 0)
            } else {
                libc::dup2(pipe_fd, 3)
            };
            if duplicated == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let finished = rfirma.output().expect("el binario rfirma deberia lanzarse");
    (finished, output)
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_rfirma_binary_signs_on_the_test_token_with_the_pin_from_a_real_pipe() {
    let home = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    let (finished, output) = the_rfirma_binary_signing_on_the_card(home.path(), KIT_PASSWORD);

    let stderr = String::from_utf8_lossy(&finished.stderr);
    assert_eq!(finished.status.code(), Some(SUCCEEDED), "{stderr}");
    assert!(finished.stdout.is_empty(), "{stderr}");
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(
        verdict_of(&the_roots_under(home.path()), &signed),
        SignatureVerdict::Valid
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_rfirma_binary_fails_when_the_pin_from_the_pipe_is_wrong_and_does_not_ask_again() {
    let home = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    let (finished, output) = the_rfirma_binary_signing_on_the_card(home.path(), "0000\n");

    assert_eq!(finished.status.code(), Some(FAILED));
    assert!(!output.exists());
}
