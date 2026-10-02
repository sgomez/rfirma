use super::*;

fn attended(words: &[&str]) -> Outcome {
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    attend(&arguments)
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
fn a_command_not_yet_available_fails_with_a_clear_message_and_an_empty_stdout() {
    for command in ["sign", "cosign", "listaliases", "verify"] {
        let outcome = attended(&[command, "-i", "a.pdf", "-o", "b.pdf"]);

        assert_eq!(outcome.exit_code, FAILED);
        assert!(outcome.stdout.is_empty());
        assert!(said(&outcome).contains(command), "{}", said(&outcome));
    }
}
