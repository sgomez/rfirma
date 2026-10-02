use std::io::{self, Read, Write};

use super::{key_of, menu, picked, typed_line, Key, CHOOSE};

struct BrokenTerminal;

impl Read for BrokenTerminal {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("EIO"))
    }
}

impl Write for BrokenTerminal {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("EIO"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_line_ends_at_the_newline_and_leaves_the_rest_unread() {
    let mut input: &[u8] = b"1234\nnext";

    let secret = typed_line(&mut input).unwrap();

    assert_eq!(secret.as_bytes(), b"1234");
    assert_eq!(input, b"next");
}

#[test]
fn a_carriage_return_before_the_newline_is_not_part_of_the_secret() {
    let mut input: &[u8] = b"1234\r\n";

    assert_eq!(typed_line(&mut input).unwrap().as_bytes(), b"1234");
}

#[test]
fn an_empty_line_is_an_empty_secret() {
    let mut input: &[u8] = b"\n";

    assert!(typed_line(&mut input).unwrap().is_empty());
}

#[test]
fn a_terminal_closed_before_the_newline_is_refused() {
    let mut input: &[u8] = b"12";

    assert_eq!(
        typed_line(&mut input).unwrap_err(),
        "no se ha tecleado nada"
    );
}

#[test]
fn a_terminal_that_fails_to_read_names_the_system_error() {
    let error = typed_line(&mut BrokenTerminal).unwrap_err();

    assert!(error.contains("EIO"), "{error}");
}

fn lines() -> Vec<String> {
    vec!["uno".to_owned(), "dos".to_owned(), "tres".to_owned()]
}

fn picked_after(keys: &[u8], preselected: usize) -> (Result<usize, String>, String) {
    let mut shown = Vec::new();
    let chosen = picked(&mut &keys[..], &mut shown, &lines(), preselected);
    (
        chosen,
        String::from_utf8(shown).expect("la lista sale en UTF-8"),
    )
}

#[test]
fn enter_chooses_the_preselected_line() {
    let (chosen, shown) = picked_after(b"\r", 1);

    assert_eq!(chosen, Ok(1));
    assert!(shown.contains("\x1b[7m> dos"), "{shown:?}");
}

#[test]
fn the_arrows_move_the_selection_and_stop_at_the_ends() {
    assert_eq!(picked_after(b"\x1b[B\x1b[B\x1b[B\n", 0).0, Ok(2));
    assert_eq!(picked_after(b"\x1b[A\x1bOA\x1b[A\n", 2).0, Ok(0));
    assert_eq!(picked_after(b"jjk\n", 0).0, Ok(1));
}

#[test]
fn each_move_redraws_the_list_over_itself() {
    let (_, shown) = picked_after(b"j\n", 0);

    assert!(shown.contains("\x1b[3A"), "{shown:?}");
    assert!(shown.ends_with(&menu(&lines(), 1)), "{shown:?}");
}

#[test]
fn keys_it_does_not_know_leave_the_selection_alone() {
    assert_eq!(picked_after(b"x\x1b[C\x1bxz\n", 1).0, Ok(1));
}

#[test]
fn q_and_ctrl_c_cancel_the_choice() {
    for keys in [&b"q"[..], &b"\x03"[..], &b"\x04"[..]] {
        assert!(picked_after(keys, 0).0.is_err());
    }
}

#[test]
fn a_terminal_closed_before_choosing_is_refused() {
    let (chosen, _) = picked_after(b"j", 0);

    assert_eq!(
        chosen,
        Err("la terminal se ha cerrado sin elegir".to_owned())
    );
}

#[test]
fn a_preselection_past_the_end_falls_on_the_last_line_and_an_empty_list_is_refused() {
    assert_eq!(picked_after(b"\n", 9).0, Ok(2));
    assert!(picked(&mut &b"\n"[..], &mut Vec::new(), &[], 0).is_err());
}

#[test]
fn a_terminal_that_fails_to_read_or_write_names_the_system_error() {
    assert!(key_of(&mut BrokenTerminal)
        .expect_err("deberia fallar")
        .contains("EIO"));
    assert!(picked(&mut &b"\n"[..], &mut BrokenTerminal, &lines(), 0)
        .expect_err("deberia fallar")
        .contains("EIO"));
}

#[test]
fn the_list_is_announced_with_how_to_use_it() {
    assert!(CHOOSE.contains('↑') && CHOOSE.contains("Intro"));
    assert_eq!(key_of(&mut &b"\x1bOB"[..]), Ok(Key::Down));
}
