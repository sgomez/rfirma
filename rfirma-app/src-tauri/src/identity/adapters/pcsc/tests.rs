use std::ffi::CString;

use pcsc::State;

use super::readers_in;
use crate::identity::domain::readers::Reader;

fn state(name: &str, state: State) -> (CString, State) {
    (CString::new(name).unwrap(), state)
}

#[test]
fn readers_are_told_apart_by_the_card_they_hold_and_the_pnp_pseudo_reader_is_not_one() {
    let states = [
        state("\\\\?PnP?\\Notification", State::CHANGED),
        state("Lector A 00 00", State::CHANGED | State::EMPTY),
        state(
            "Lector B 01 00",
            State::CHANGED | State::PRESENT | State::INUSE,
        ),
        state(
            "Lector C 02 00",
            State::CHANGED | State::UNKNOWN | State::IGNORE,
        ),
    ];

    assert_eq!(
        readers_in(&states),
        vec![
            Reader {
                name: "Lector A 00 00".into(),
                has_a_card: false
            },
            Reader {
                name: "Lector B 01 00".into(),
                has_a_card: true
            },
        ]
    );
}
