//! `sign -certtui` elige el certificado en la terminal entre los vigentes y pide después el PIN.

#[path = "command_line/support.rs"]
mod support;

use support::*;

fn signed_choosing(
    home: &Path,
    roots: &Roots,
    terminal: &ScriptedTerminal,
    selection: &[&str],
) -> (Outcome, PathBuf) {
    let input = home.join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.join("firmado.pdf");
    let _ = std::fs::remove_file(&output);
    let mut words = vec![
        "sign",
        "-i",
        input.to_str().expect("ruta UTF-8"),
        "-o",
        output.to_str().expect("ruta UTF-8"),
    ];
    words.extend_from_slice(selection);
    (attended_with_the_roots(&words, roots, terminal), output)
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn certtui_preselects_the_remembered_certificate_and_lists_only_usable_ones_with_their_store() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let (remembered, _) = signed_choosing(
        home.path(),
        &roots,
        &ScriptedTerminal::without_a_tty(),
        &["-alias", &installed_alias],
    );
    assert_eq!(remembered.exit_code, SUCCEEDED, "{:?}", remembered.stderr);
    let terminal = ScriptedTerminal::typing(&[]);

    let (outcome, output) = signed_choosing(home.path(), &roots, &terminal, &["-certtui"]);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let shown = terminal.lists_shown();
    let (offered, preselected) = &shown[0];
    assert_eq!(offered[*preselected].store, "Almacén de rFirma");
    assert!(offered
        .iter()
        .any(|certificate| certificate.store.starts_with("tarjeta")));
    assert!(offered
        .iter()
        .all(|certificate| !certificate.expires.is_empty()));
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn certtui_within_the_card_store_asks_the_pin_on_the_tty_after_choosing() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let store = format!("pkcs11:{}", the_card_module().display());
    let probe = ScriptedTerminal::typing(&[]).choosing(Some(usize::MAX));
    let _ = signed_choosing(home.path(), &roots, &probe, &["-certtui", "-store", &store]);
    let listed = &probe.lists_shown()[0].0;
    assert!(listed
        .iter()
        .all(|certificate| certificate.store.starts_with("tarjeta")));
    let active = listed
        .iter()
        .position(|certificate| certificate.holder.contains("99999999R"))
        .expect("el certificado activo del kit deberia estar en la lista");
    let terminal = ScriptedTerminal::typing(&[KIT_PASSWORD]).choosing(Some(active));

    let (outcome, output) = signed_choosing(
        home.path(),
        &roots,
        &terminal,
        &["-certtui", "-store", &store],
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert_eq!(terminal.retries_asked(), vec![false]);
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
}
