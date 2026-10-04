use std::path::PathBuf;

use super::*;

const A_LAUNCH: &str = "afirma://websocket?ports=51000,51001&v=4&idsession=8jAkPZfRw2mQxN4TbYuL";

fn invoked_with_the_words(words: &[&str]) -> Invocation {
    Invocation {
        command_line: std::iter::once("rfirma.com")
            .chain(words.iter().copied())
            .map(str::to_owned)
            .collect(),
        folder: PathBuf::from("/"),
    }
}

#[test]
fn a_terminal_command_is_attended_by_the_console() {
    for words in [
        &["sign", "-i", "a.pdf", "-o", "b.pdf"][..],
        &["cosign", "-i", "a.pdf"],
        &["ListAliases"],
        &["verify", "-i", "a.pdf"],
        &["sign", "--help"],
        &["sign"],
    ] {
        assert_eq!(
            console_entry_of(invoked_with_the_words(words)),
            ConsoleEntry::AttendsTheCommand,
            "{words:?}"
        );
    }
}

#[test]
fn the_help_and_the_version_are_printed_by_the_console() {
    for flag in ["--help", "-h", "--version", "-version"] {
        assert_eq!(
            console_entry_of(invoked_with_the_words(&[flag])),
            ConsoleEntry::PrintsTheInformativeText,
            "{flag}"
        );
    }
}

#[test]
fn no_arguments_are_handed_over_to_the_window() {
    assert_eq!(
        console_entry_of(invoked_with_the_words(&[])),
        ConsoleEntry::HandsOverToTheWindow
    );
}

#[test]
fn a_file_is_handed_over_to_the_window() {
    assert_eq!(
        console_entry_of(invoked_with_the_words(&["contrato.pdf"])),
        ConsoleEntry::HandsOverToTheWindow
    );
}

#[test]
fn an_afirma_url_is_handed_over_to_the_window_even_with_a_command() {
    for words in [&[A_LAUNCH][..], &["sign", A_LAUNCH]] {
        assert_eq!(
            console_entry_of(invoked_with_the_words(words)),
            ConsoleEntry::HandsOverToTheWindow,
            "{words:?}"
        );
    }
}

#[test]
fn a_foreign_url_is_handed_over_to_the_window() {
    assert_eq!(
        console_entry_of(invoked_with_the_words(&["https://example.org/a.pdf"])),
        ConsoleEntry::HandsOverToTheWindow
    );
}
