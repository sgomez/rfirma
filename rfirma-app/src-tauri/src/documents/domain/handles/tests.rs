use super::*;

#[test]
fn a_handle_is_thirty_two_hexadecimal_digits() {
    let handle = mint();

    assert_eq!(handle.len(), 32);
    assert!(handle
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn two_handles_are_never_the_same() {
    assert_ne!(mint(), mint());
}

#[test]
fn the_fallback_keeps_the_shape_and_the_difference() {
    let first = minted_without_the_system_csprng();
    let second = minted_without_the_system_csprng();

    assert_eq!(first.len(), 32);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    assert_ne!(first, second);
}

#[test]
fn what_was_minted_comes_back_by_its_handle() {
    let handles = Handles::new();

    let handle = handles.mint("contrato.pdf");

    assert_eq!(handles.get(&handle), Some("contrato.pdf"));
}

#[test]
fn a_handle_nobody_minted_is_simply_not_there() {
    let handles: Handles<&str> = Handles::new();

    assert_eq!(handles.get("00000000000000000000000000000000"), None);
    assert!(handles.is_empty());
}

#[test]
fn the_same_value_minted_twice_gets_two_handles_and_both_stay() {
    let handles = Handles::new();

    let first = handles.mint("contrato.pdf");
    let second = handles.mint("contrato.pdf");

    assert_ne!(first, second);
    assert_eq!(handles.len(), 2);
}

#[test]
fn replacing_forgets_every_handle_minted_before() {
    let handles = Handles::new();
    let before = handles.replace(["FIRMA"]);

    let after = handles.replace(["OTRO", "Y OTRO"]);

    assert_eq!(handles.len(), 2);
    assert_eq!(handles.get(&before[0]), None);
    assert_eq!(handles.get(&after[0]), Some("OTRO"));
    assert_eq!(handles.get(&after[1]), Some("Y OTRO"));
}

#[test]
fn the_last_handle_where_is_the_one_minted_latest_among_the_matching() {
    let handles = Handles::new();
    let first = handles.mint(("contrato.pdf", true));
    handles.mint(("contrato.pdf", false));
    let third = handles.mint(("contrato.pdf", true));
    handles.mint(("factura.pdf", true));

    assert_eq!(
        handles.last_where(|(name, wanted)| *name == "contrato.pdf" && *wanted),
        Some(third.clone())
    );
    assert_ne!(first, third);
    assert_eq!(handles.last_where(|(name, _)| *name == "nadie.pdf"), None);
}

#[test]
fn the_handle_carries_nothing_of_what_it_stands_for() {
    let handles = Handles::new();

    let handle = handles.mint("/usr/lib/softhsm/libsofthsm2.so rfirma-test FNMT 99999999R");

    assert_eq!(handle.len(), 32);
    assert!(handle
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    for leak in ["/", "usr", "softhsm", "rfirma-test", "FNMT", "99999999R"] {
        assert!(
            !handle.contains(leak),
            "el asa «{handle}» lleva «{leak}» dentro"
        );
    }
}
