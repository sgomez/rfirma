use super::*;

#[test]
fn the_password_refusal_on_windows_explains_who_asks_for_the_pin_without_naming_linux_things() {
    let refusal = command_of(
        &arguments(&["sign", "-password", "1234"]),
        Platform::Windows,
    )
    .expect_err("se rechaza");

    assert_eq!(refusal, Refusal::PasswordInArgvOnWindows);
    let text = refusal.to_string();
    assert!(text.contains("Windows"), "{text}");
    assert!(text.contains("--certgui"), "{text}");
    assert!(
        !text.contains("password-fd") && !text.contains("secret-tool"),
        "{text}"
    );
    assert!(!text.contains("1234"));
}

#[test]
fn the_windows_syntax_of_sign_and_cosign_leaves_out_certtui_and_password_fd() {
    for command in [Command::Sign, Command::Cosign] {
        let syntax = command.syntax(Platform::Windows).expect("tiene sintaxis");

        assert!(
            !syntax.contains("certtui") && !syntax.contains("password-fd"),
            "{syntax}"
        );
        assert!(syntax.contains("--certgui"), "{syntax}");
    }
}

fn arguments(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn every_command_is_recognised_whatever_the_case() {
    for (word, command) in [
        ("sign", Command::Sign),
        ("COSIGN", Command::Cosign),
        ("ListAliases", Command::ListAliases),
        ("Verify", Command::Verify),
        ("countersign", Command::CounterSign),
        ("BatchSign", Command::BatchSign),
    ] {
        assert_eq!(Command::named(word), Some(command), "{word}");
    }
}

#[test]
fn a_document_or_a_parameter_is_not_a_command() {
    for word in ["contrato.pdf", "-i", "signature", ""] {
        assert_eq!(Command::named(word), None, "{word}");
    }
}

#[test]
fn the_four_attended_commands_have_their_syntax() {
    for command in [
        Command::Sign,
        Command::Cosign,
        Command::ListAliases,
        Command::Verify,
    ] {
        let syntax = command.syntax(Platform::Linux).expect("tiene sintaxis");
        assert!(
            syntax.contains(&format!("rfirma {}", command.name())),
            "{syntax}"
        );
    }
}

#[test]
fn countersign_and_batchsign_are_refused_as_left_out() {
    for word in ["countersign", "BATCHSIGN"] {
        let refusal = command_of(&arguments(&[word, "-i", "a.pdf"]), Platform::Linux)
            .expect_err("se rechaza");

        assert!(matches!(refusal, Refusal::CommandLeftOut(_)), "{refusal:?}");
        assert!(refusal.to_string().contains(&word.to_lowercase()));
    }
}

#[test]
fn the_password_is_refused_in_any_position_naming_password_fd() {
    for words in [
        &["sign", "-password", "1234", "-i", "a.pdf"][..],
        &["sign", "-i", "a.pdf", "-o", "b.pdf", "-PASSWORD", "1234"][..],
        &["-password", "1234", "sign"][..],
        &["countersign", "-password", "1234"][..],
    ] {
        let refusal = command_of(&arguments(words), Platform::Linux).expect_err("se rechaza");

        assert_eq!(refusal, Refusal::PasswordInArgv);
        assert!(refusal.to_string().contains(PASSWORD_FD));
        assert!(
            !refusal.to_string().contains("1234"),
            "nunca repite el secreto"
        );
    }
}

#[test]
fn the_password_descriptor_is_not_the_password() {
    assert_eq!(
        command_of(&arguments(&["sign", "-password-fd", "3"]), Platform::Linux),
        Ok(Command::Sign)
    );
}

#[test]
fn a_word_that_is_not_a_command_is_refused_as_unknown() {
    assert_eq!(
        command_of(&arguments(&["firmar"]), Platform::Linux),
        Err(Refusal::UnknownCommand("firmar".to_owned()))
    );
    assert_eq!(command_of(&[], Platform::Linux), Err(Refusal::NoCommand));
}

#[test]
fn every_parameter_left_out_is_refused_by_its_name() {
    for parameter in PARAMETERS_LEFT_OUT {
        let refusal = parameter_left_out(&arguments(&["sign", "-i", "a.pdf", parameter, "x"]))
            .expect("se rechaza");

        assert_eq!(refusal, Refusal::ParameterLeftOut(parameter));
        assert!(refusal.to_string().contains(parameter));
    }
}

#[test]
fn the_attended_parameters_are_not_left_out() {
    let attended = arguments(&[
        "sign", "-i", "a.pdf", "-o", "b.pdf", "-alias", "yo", "-store", "pkcs11",
    ]);

    assert_eq!(parameter_left_out(&attended), None);
}

#[test]
fn the_help_is_asked_for_with_any_of_its_three_flags() {
    for flag in ["--help", "-help", "-h"] {
        assert!(is_a_help_flag(flag), "{flag}");
    }
    assert!(!is_a_help_flag("help"));
}

#[test]
fn the_value_of_a_parameter_is_the_argument_that_follows_it() {
    let words = arguments(&["verify", "-i", "firmado.pdf", "-xml"]);

    assert_eq!(value_of(&words, INPUT), Some("firmado.pdf"));
}

#[test]
fn a_parameter_that_is_missing_or_last_has_no_value() {
    assert_eq!(value_of(&arguments(&["verify"]), INPUT), None);
    assert_eq!(value_of(&arguments(&["verify", "-i"]), INPUT), None);
}

#[test]
fn a_known_option_in_double_dash_form_is_normalised_and_values_are_left_alone() {
    let normalised = normalised(&arguments(&[
        "sign",
        "-i",
        "--xml",
        "--alias",
        "--format",
        "--i",
        "--desconocida",
        "--password",
    ]));

    assert_eq!(
        normalised,
        arguments(&[
            "sign",
            "-i",
            "-xml",
            "-alias",
            "-format",
            "--i",
            "--desconocida",
            "-password"
        ])
    );
}

#[test]
fn the_documented_form_has_two_dashes_only_for_long_options() {
    assert_eq!(documented("-store"), "--store");
    assert_eq!(documented("-i"), "-i");
}

#[test]
fn verbose_is_asked_for_with_v_or_with_its_long_form() {
    let verbosity_of = |words: &[&str]| verbosity(&normalised(&arguments(words)));

    assert_eq!(verbosity_of(&["verify", "-i", "a.pdf"]), 0);
    assert_eq!(verbosity_of(&["verify", "-v", "-i", "a.pdf"]), 1);
    assert_eq!(verbosity_of(&["verify", "--verbose"]), 1);
    assert_eq!(verbosity_of(&["verify", "-verbose"]), 1);
}

#[test]
fn every_appearance_adds_a_level_and_from_two_on_it_is_the_same() {
    let verbosity_of = |words: &[&str]| verbosity(&normalised(&arguments(words)));

    assert_eq!(verbosity_of(&["verify", "-vv"]), 2);
    assert_eq!(verbosity_of(&["verify", "-v", "-v"]), 2);
    assert_eq!(verbosity_of(&["verify", "--verbose", "--verbose"]), 2);
    assert!(verbosity_of(&["verify", "-vvv"]) >= 2);
}

#[test]
fn the_manual_documents_every_command_and_flag_of_the_declared_syntax() {
    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packaging/repo/site/src/content/docs/manual/linea-de-ordenes.md"),
    )
    .expect("la página «Línea de órdenes» del manual");

    let mut missing = Vec::new();
    for command in COMMANDS {
        let syntaxes: Vec<&str> = [Platform::Linux, Platform::Windows]
            .into_iter()
            .filter_map(|platform| command.syntax(platform))
            .collect();
        if syntaxes.is_empty() {
            continue;
        }
        if !mentions(&page, command.name()) {
            missing.push(format!("la orden `{}`", command.name()));
        }
        for syntax in syntaxes {
            for flag in flags_of(syntax) {
                let entry = format!("el flag `{flag}` de `{}`", command.name());
                if !mentions(&page, &flag) && !missing.contains(&entry) {
                    missing.push(entry);
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "falta en la página «Línea de órdenes» del manual: {}",
        missing.join(", ")
    );
}

fn flags_of(syntax: &str) -> Vec<String> {
    let mut flags: Vec<String> = syntax
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| "[]()|,.;".contains(c)))
        .filter(|word| word.starts_with('-') && word.len() > 1)
        .map(str::to_owned)
        .collect();
    flags.sort();
    flags.dedup();
    flags
}

fn mentions(page: &str, word: &str) -> bool {
    let is_part_of_a_word = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    page.match_indices(word).any(|(at, _)| {
        let before = page[..at].chars().next_back();
        let after = page[at + word.len()..].chars().next();
        !before.is_some_and(is_part_of_a_word) && !after.is_some_and(is_part_of_a_word)
    })
}
