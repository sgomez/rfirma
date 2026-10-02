//! La línea de órdenes contra el token `rfirma-test` y el Almacén de rFirma: por su caso de uso con una terminal guionizada, y lanzando el binario `rfirma`.

#[path = "command_line/support.rs"]
mod support;

use support::*;

#[path = "command_line/config.rs"]
mod config;
#[path = "command_line/cosign.rs"]
mod cosign;
#[path = "command_line/formats.rs"]
mod formats;

#[test]
fn listaliases_lists_the_test_token_and_the_rfirma_store_one_alias_per_line() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();

    let outcome = attended_over(
        &["listaliases"],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let aliases = sorted_lines_of(&outcome.stdout);
    assert_eq!(aliases, every_alias_sorted(installed_alias));
    assert!(
        aliases.iter().any(|alias| alias == CARD_ACTIVE),
        "{aliases:?}"
    );
}

#[test]
fn listaliases_with_the_card_module_lists_only_the_token() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let store = format!("pkcs11:{}", the_card_module().display());

    let outcome = attended_over(
        &["listaliases", "-store", &store],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let mut aliases = the_card_aliases();
    aliases.sort();
    assert_eq!(sorted_lines_of(&outcome.stdout), aliases);
}

#[test]
fn listaliases_with_the_nss_family_lists_only_the_rfirma_store() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();

    let outcome = attended_over(
        &["listaliases", "-store", "mozilla"],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert_eq!(sorted_lines_of(&outcome.stdout), vec![installed_alias]);
}

#[test]
fn listaliases_with_a_module_that_is_not_discovered_loads_nothing_and_fails() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();

    let outcome = attended_over(
        &["listaliases", "-store", "pkcs11:/usr/lib/no-existe.so"],
        home.path(),
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.stdout.is_empty());
}

#[test]
fn the_rfirma_binary_lists_aliases_without_a_window_and_with_a_clean_stdout() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();

    let mut rfirma = Command::new(env!("CARGO_BIN_EXE_rfirma"));
    if let Some(configuration) = the_softhsm_configuration() {
        rfirma.env("SOFTHSM2_CONF", configuration);
    }
    let output = rfirma
        .arg("listaliases")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("RFIRMA_PKCS11_MODULE", the_card_module())
        .env("RUST_LOG", "trace")
        .output()
        .expect("el binario rfirma deberia lanzarse");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(SUCCEEDED), "{stderr}");
    assert_eq!(
        sorted_lines_of(&output.stdout),
        every_alias_sorted(installed_alias),
        "{stderr}"
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_with_an_alias_of_the_rfirma_store_signs_a_valid_pades_exactly_at_the_output() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");

    for (output, extra) in [
        ("firmado.pdf", &[][..]),
        (
            "firmado-sha256.pdf",
            &["-format", "pades", "-algorithm", "sha256"][..],
        ),
    ] {
        let output = home.path().join(output);
        std::fs::write(&output, b"lo que hubiera antes").expect("deberia escribirse");
        let mut words = vec![
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-alias",
            &installed_alias,
        ];
        words.extend_from_slice(extra);

        let outcome = attended_with_the_roots(&words, &roots, &ScriptedTerminal::without_a_tty());

        assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
        assert!(outcome.stdout.is_empty());
        assert!(!outcome.stderr.is_empty());
        let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
        assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
    }
    assert_eq!(
        roots
            .identity
            .remembered_certificate()
            .map(|reference| reference.label().to_owned()),
        Some(installed_alias)
    );
    let state = roots
        .signing
        .memory
        .state()
        .expect("el estado deberia leerse")
        .into_value();
    assert!(state.recents.is_empty(), "no se apunta en los recientes");
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_with_a_filter_picks_the_certificate_by_its_data_and_fails_with_none_or_several() {
    let (home, _alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.path().join("firmado.pdf");
    let sign_with = |filter: &str, store: &[&str]| {
        let mut words = vec![
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-filter",
            filter,
        ];
        words.extend_from_slice(store);
        attended_with_the_roots(&words, &roots, &ScriptedTerminal::without_a_tty())
    };

    let one = sign_with("subject.contains:99999999R", &["-store", "mozilla"]);
    assert_eq!(one.exit_code, SUCCEEDED, "{:?}", one.stderr);
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);

    std::fs::remove_file(&output).expect("deberia borrarse");
    let none = sign_with("subject.contains:NO-EXISTE-NADIE-ASI", &[]);
    assert_eq!(none.exit_code, FAILED);
    let several = sign_with("subject.contains:99999999R", &[]);
    assert_eq!(several.exit_code, FAILED, "{:?}", several.stderr);
    assert!(!output.exists());
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_with_xml_and_no_output_answers_with_the_signature_in_base64_on_stdout() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let words = [
        "sign",
        "-i",
        input.to_str().expect("ruta UTF-8"),
        "-alias",
        &installed_alias,
        "-xml",
    ];

    let outcome = attended_with_the_roots(&words, &roots, &ScriptedTerminal::without_a_tty());

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let xml = String::from_utf8(outcome.stdout).expect("stdout en UTF-8");
    let signature = xml
        .split("<sign>")
        .nth(1)
        .and_then(|rest| rest.split("</sign>").next())
        .expect("la respuesta lleva <sign>");
    let signed = base64::engine::general_purpose::STANDARD
        .decode(signature)
        .expect("la firma va en Base64");
    assert_eq!(verdict_of(&roots, &signed), SignatureVerdict::Valid);
    assert!(xml.starts_with("<afirma><result>true</result>"), "{xml}");
}

#[test]
fn a_signature_the_bridge_cannot_make_fails_on_stderr_and_leaves_no_output_or_memory() {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let mut roots = the_roots_under(home.path());
    roots.signing.isolate =
        Isolate::start_with(|| Err(BridgeError::Failed("sin puente en esta prueba".to_owned())));
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.path().join("firmado.pdf");

    let outcome = attended_with_the_roots(
        &[
            "sign",
            "-i",
            input.to_str().expect("ruta UTF-8"),
            "-o",
            output.to_str().expect("ruta UTF-8"),
            "-alias",
            &installed_alias,
        ],
        &roots,
        &ScriptedTerminal::without_a_tty(),
    );

    assert_eq!(outcome.exit_code, FAILED, "{:?}", outcome.stderr);
    assert!(
        outcome.stderr.concat().contains("sin puente"),
        "{:?}",
        outcome.stderr
    );
    assert!(outcome.stdout.is_empty());
    assert!(!output.exists());
    assert_eq!(roots.identity.remembered_certificate(), None);
}

#[test]
fn the_rfirma_binary_attends_sign_as_a_terminal_command_and_not_with_a_window() {
    let (home, _installed_alias) = a_home_with_an_installed_certificate();
    let input = home.path().join("documento.pdf");
    std::fs::write(&input, a_one_page_pdf()).expect("el PDF deberia escribirse");
    let output = home.path().join("firmado.pdf");

    let mut rfirma = Command::new(env!("CARGO_BIN_EXE_rfirma"));
    if let Some(configuration) = the_softhsm_configuration() {
        rfirma.env("SOFTHSM2_CONF", configuration);
    }
    let finished = rfirma
        .arg("sign")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .args(["-alias", "nadie-con-este-alias"])
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("RFIRMA_PKCS11_MODULE", the_card_module())
        .output()
        .expect("el binario rfirma deberia lanzarse");

    let stderr = String::from_utf8_lossy(&finished.stderr);
    assert_eq!(finished.status.code(), Some(FAILED), "{stderr}");
    assert!(stderr.contains("nadie-con-este-alias"), "{stderr}");
    assert!(finished.stdout.is_empty(), "{stderr}");
    assert!(!output.exists());
}
