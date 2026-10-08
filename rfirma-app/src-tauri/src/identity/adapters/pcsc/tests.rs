use std::collections::VecDeque;
use std::ffi::CString;
use std::time::Duration;

use pcsc::State;

use super::{readers_in, ReaderStates, StatusSource, StatusWatch};
use crate::identity::domain::readers::Reader;
use crate::identity::ports::ReaderWatch;

fn state(name: &str, state: State) -> (CString, State) {
    (CString::new(name).unwrap(), state)
}

fn reader(name: &str, has_a_card: bool) -> Reader {
    Reader {
        name: name.into(),
        has_a_card,
    }
}

struct Script(VecDeque<Option<ReaderStates>>);

impl StatusSource for Script {
    fn wait_for_a_change(&mut self, _known: &[(CString, State)]) -> Option<ReaderStates> {
        self.0.pop_front().expect("the script ran out")
    }
}

fn watching(script: Vec<Option<ReaderStates>>) -> StatusWatch<Script> {
    StatusWatch::new(Script(script.into()), Duration::ZERO)
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
            reader("Lector A 00 00", false),
            reader("Lector B 01 00", true)
        ]
    );
}

#[test]
fn each_real_change_is_told_once_and_states_that_change_nothing_are_skipped() {
    let empty = state("Lector A", State::EMPTY);
    let present = state("Lector A", State::PRESENT);
    let mut watch = watching(vec![
        Some(vec![empty.clone()]),
        Some(vec![state("Lector A", State::EMPTY | State::CHANGED)]),
        Some(vec![present]),
        Some(vec![]),
    ]);

    assert_eq!(watch.next_change(), Some(vec![reader("Lector A", false)]));
    assert_eq!(watch.next_change(), Some(vec![reader("Lector A", true)]));
    assert_eq!(watch.next_change(), Some(vec![]));
}

#[test]
fn without_pcsc_no_readers_are_told_once_and_the_watch_waits_for_it_to_come_back() {
    let mut watch = watching(vec![
        None,
        None,
        None,
        Some(vec![state("Lector A", State::PRESENT)]),
    ]);

    assert_eq!(watch.next_change(), Some(vec![]));
    assert_eq!(watch.next_change(), Some(vec![reader("Lector A", true)]));
}

#[test]
fn losing_pcsc_after_having_readers_tells_there_are_none() {
    let mut watch = watching(vec![
        Some(vec![state("Lector A", State::EMPTY)]),
        None,
        Some(vec![state("Lector A", State::EMPTY)]),
    ]);

    assert_eq!(watch.next_change(), Some(vec![reader("Lector A", false)]));
    assert_eq!(watch.next_change(), Some(vec![]));
    assert_eq!(watch.next_change(), Some(vec![reader("Lector A", false)]));
}
