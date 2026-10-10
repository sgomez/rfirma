#![cfg(not(windows))]

use std::path::Path;
use std::time::{Duration, Instant};

use fake_pkcs11::FakeCard;

use super::*;

const HANG_LIMIT: Duration = Duration::from_millis(300);
const HEALTHY_LIMIT: Duration = Duration::from_secs(120);

#[test]
fn a_module_that_loads_says_the_manufacturer_and_version_its_library_declares() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    assert_eq!(
        probe(&card.module(), HEALTHY_LIMIT),
        ModuleProbe::Loads(LibraryInfo {
            manufacturer: "rfirma".to_owned(),
            version: "0.1".to_owned(),
        })
    );
}

#[test]
fn a_library_that_does_not_exist_does_not_load() {
    assert_eq!(
        probe(Path::new("/nonexistent/libnothing.so"), HANG_LIMIT),
        ModuleProbe::DoesNotLoad
    );
}

#[test]
fn a_module_that_hangs_on_initialize_is_not_responding_once_the_limit_passes() {
    let card = FakeCard::new()
        .and_then(FakeCard::hanging_on_initialize)
        .expect("la tarjeta falsa deberia montarse");
    let started = Instant::now();

    assert_eq!(
        probe(&card.module(), HANG_LIMIT),
        ModuleProbe::NotResponding
    );
    assert!(started.elapsed() < HANG_LIMIT * 10);
}

#[test]
fn the_probe_calls_no_slot_token_session_or_object_function() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    probe(&card.module(), HEALTHY_LIMIT);

    let functions: Vec<String> = card
        .calls()
        .iter()
        .filter_map(|call| call.split_whitespace().next().map(str::to_owned))
        .collect();
    assert_eq!(
        functions,
        [
            "C_GetFunctionList",
            "C_Initialize",
            "C_GetInfo",
            "C_Finalize"
        ]
    );
}
