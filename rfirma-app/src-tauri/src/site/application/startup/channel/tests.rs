use std::sync::Mutex;

use super::*;
use crate::site::adapters::channel::Shutdown;

const PORTS: [u16; 3] = [51001, 51002, 51003];

fn a_channel(port: u16, closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> OpenChannel {
    let closed = std::sync::Arc::clone(closed);
    OpenChannel::new(port, Shutdown::of(move || crate::lock(&closed).push(port)))
}

fn closed_ports(closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> Vec<u16> {
    crate::lock(closed).clone()
}

#[test]
fn a_refusal_never_closes_the_channel_of_the_live_errand() {
    let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
    let held = HeldChannel::default();

    held.hold(a_channel(PORTS[0], &closed));
    held.hold_a_refusal(a_channel(PORTS[1], &closed));

    assert!(
        closed_ports(&closed).is_empty(),
        "el canal del trámite vivo sigue sirviendo: {:?}",
        closed_ports(&closed)
    );
}

#[test]
fn a_new_refusal_closes_the_refusal_it_replaces() {
    let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
    let held = HeldChannel::default();

    held.hold_a_refusal(a_channel(PORTS[0], &closed));
    held.hold_a_refusal(a_channel(PORTS[1], &closed));

    assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
}

#[test]
fn an_unheld_channel_is_not_serving() {
    let held = HeldChannel::default();

    assert!(!held.is_serving());
}

#[test]
fn only_the_channel_of_the_errand_counts_as_serving() {
    let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
    let held = HeldChannel::default();

    held.hold_a_refusal(a_channel(PORTS[0], &closed));
    assert!(!held.is_serving());

    held.hold(a_channel(PORTS[1], &closed));
    assert!(held.is_serving());
}

#[test]
fn a_new_serving_channel_closes_the_one_it_replaces() {
    let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
    let held = HeldChannel::default();

    held.hold(a_channel(PORTS[0], &closed));
    held.hold(a_channel(PORTS[1], &closed));

    assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
}
