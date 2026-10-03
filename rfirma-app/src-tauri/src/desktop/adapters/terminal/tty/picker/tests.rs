use ratatui::backend::TestBackend;
use std::io;

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;

use super::{height_for, run, told, Picker, Step};
use crate::desktop::ports::OfferedCertificate;

fn a_certificate(headline: &str, capacity: &str, stores: &[&str]) -> OfferedCertificate {
    OfferedCertificate {
        headline: headline.to_owned(),
        capacity: capacity.to_owned(),
        issuer: "AC FNMT Usuarios".to_owned(),
        expires: "2028-01-31".to_owned(),
        stores: stores.iter().map(|store| (*store).to_owned()).collect(),
    }
}

fn three() -> Vec<OfferedCertificate> {
    vec![
        a_certificate(
            "EMPRESA S.L. · B00000000",
            "Representante · JUAN PÉREZ LÓPEZ · 00000000T",
            &["Firefox", "tarjeta «DNIe»"],
        ),
        a_certificate(
            "PÉREZ LÓPEZ JUAN",
            "A título personal · 00000000T",
            &["Almacén de rFirma"],
        ),
        a_certificate("OTRA PERSONA", "A título personal · 11111111H", &["Chrome"]),
    ]
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn typed(picker: &mut Picker<'_>, text: &str) {
    for character in text.chars() {
        picker.on(key(KeyCode::Char(character)));
    }
}

fn screen(picker: &mut Picker<'_>, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal de prueba");
    terminal
        .draw(|frame| picker.draw(frame))
        .expect("el selector se pinta");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn enter_chooses_the_preselected_certificate() {
    let offered = three();
    let mut picker = Picker::new(&offered, 1);

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(1));
}

#[test]
fn the_arrows_move_the_selection_and_stop_at_the_ends() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);

    picker.on(key(KeyCode::Up));
    picker.on(key(KeyCode::Down));
    picker.on(key(KeyCode::Down));
    picker.on(key(KeyCode::Down));

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(2));
}

#[test]
fn a_preselection_past_the_end_falls_on_the_last_certificate() {
    let offered = three();
    let mut picker = Picker::new(&offered, 9);

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(2));
}

#[test]
fn typing_narrows_the_list_ignoring_case_and_accents() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);

    typed(&mut picker, "perez personal");

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(1));
}

#[test]
fn the_filter_also_finds_a_certificate_by_its_store() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);

    typed(&mut picker, "chrome");

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(2));
}

#[test]
fn the_filter_also_finds_a_certificate_by_its_expiry() {
    let mut offered = three();
    offered[2].expires = "2027-03-15".to_owned();
    let mut picker = Picker::new(&offered, 0);

    typed(&mut picker, "2027");

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(2));
}

#[test]
fn a_filter_that_keeps_the_selection_leaves_it_where_it_was() {
    let offered = three();
    let mut picker = Picker::new(&offered, 1);

    typed(&mut picker, "lopez");

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(1));
}

#[test]
fn backspace_widens_the_list_again() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);

    typed(&mut picker, "chromex");
    picker.on(key(KeyCode::Backspace));

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Chosen(2));
}

#[test]
fn enter_with_nothing_matching_chooses_nothing() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);

    typed(&mut picker, "nadie");

    assert_eq!(picker.on(key(KeyCode::Enter)), Step::Pending);
}

#[test]
fn escape_and_ctrl_c_cancel_the_choice() {
    let offered = three();

    assert_eq!(
        Picker::new(&offered, 0).on(key(KeyCode::Esc)),
        Step::Cancelled
    );
    assert_eq!(
        Picker::new(&offered, 0).on(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Step::Cancelled
    );
}

#[test]
fn each_certificate_shows_who_signs_in_what_capacity_and_every_store() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);

    let shown = screen(&mut picker, 100, height_for(offered.len()));

    assert!(shown.contains("> EMPRESA S.L. · B00000000"), "{shown}");
    assert!(
        shown.contains("    Representante · JUAN PÉREZ LÓPEZ · 00000000T"),
        "{shown}"
    );
    assert!(
        shown.contains("    AC FNMT Usuarios · caduca el 2028-01-31\n"),
        "{shown}"
    );
    assert!(
        shown.contains("    En Firefox, tarjeta «DNIe»\n"),
        "{shown}"
    );
    assert!(shown.contains("  PÉREZ LÓPEZ JUAN"), "{shown}");
}

#[test]
fn the_prompt_shows_what_is_typed_and_how_to_use_the_list() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);
    typed(&mut picker, "emp");

    let shown = screen(&mut picker, 100, height_for(offered.len()));

    assert!(shown.starts_with("Certificado: emp"), "{shown}");
    assert!(shown.contains("Esc cancela"), "{shown}");
}

#[test]
fn nothing_matching_is_said_instead_of_an_empty_list() {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);
    typed(&mut picker, "nadie");

    let shown = screen(&mut picker, 100, height_for(offered.len()));

    assert!(
        shown.contains("Ningún certificado coincide con «nadie»"),
        "{shown}"
    );
}

#[test]
fn a_long_list_scrolls_to_keep_the_selection_in_sight() {
    let offered: Vec<OfferedCertificate> = (0..12)
        .map(|n| {
            a_certificate(
                &format!("TITULAR {n:02}"),
                "A título personal · X",
                &["Chrome"],
            )
        })
        .collect();
    let mut picker = Picker::new(&offered, 11);

    let shown = screen(&mut picker, 80, height_for(offered.len()));

    assert!(shown.contains("> TITULAR 11"), "{shown}");
    assert!(!shown.contains("TITULAR 00"), "{shown}");
}

fn ran(events: Vec<io::Result<Event>>) -> (Result<Step, String>, String) {
    let offered = three();
    let mut picker = Picker::new(&offered, 0);
    let mut terminal = Terminal::new(TestBackend::new(60, 13)).expect("terminal de prueba");
    let mut events = events.into_iter();
    let step = run(&mut terminal, &mut picker, || {
        events.next().expect("quedan teclas en el guion")
    });
    let left = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    (step, left.trim().to_owned())
}

#[test]
fn the_list_ignores_released_keys_and_other_events() {
    let (step, _) = ran(vec![
        Ok(Event::Resize(60, 13)),
        Ok(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Down,
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ))),
        Ok(Event::Key(key(KeyCode::Down))),
        Ok(Event::Key(key(KeyCode::Enter))),
    ]);

    assert_eq!(step, Ok(Step::Chosen(1)));
}

#[test]
fn the_list_leaves_the_terminal_blank_when_done() {
    let (step, left) = ran(vec![Ok(Event::Key(key(KeyCode::Esc)))]);

    assert_eq!(step, Ok(Step::Cancelled));
    assert_eq!(left, "");
}

#[test]
fn a_terminal_that_stops_answering_ends_the_choice() {
    let (step, _) = ran(vec![Err(io::Error::other("cerrada"))]);

    assert_eq!(
        step,
        Err("no se puede usar la terminal (cerrada)".to_owned())
    );
}

#[test]
fn the_chosen_certificate_is_left_written_under_the_prompt() {
    let mut out = Vec::new();

    let chosen = told(Step::Chosen(1), &three(), &mut out);

    assert_eq!(chosen, Ok(1));
    assert_eq!(
        String::from_utf8(out).expect("texto"),
        "Certificado: PÉREZ LÓPEZ JUAN\n"
    );
}

#[test]
fn a_cancelled_choice_writes_nothing() {
    let mut out = Vec::new();

    let chosen = told(Step::Cancelled, &three(), &mut out);

    assert_eq!(chosen, Err("se ha cancelado la elección".to_owned()));
    assert!(out.is_empty());
}
