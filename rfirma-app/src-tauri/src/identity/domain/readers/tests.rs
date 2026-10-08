use super::*;

fn reader(name: &str, has_a_card: bool) -> Reader {
    Reader {
        name: name.to_owned(),
        has_a_card,
    }
}

#[test]
fn with_no_reader_the_status_is_no_reader() {
    assert_eq!(
        status_of(&[], Listing::Done(Some(ReadyCard::Dnie))),
        ReaderStatus::NoReader
    );
}

#[test]
fn an_empty_reader_is_a_reader_with_no_card() {
    assert_eq!(
        status_of(&[reader("A", false)], Listing::InProgress),
        ReaderStatus::NoCard
    );
}

#[test]
fn a_card_being_listed_is_reading_and_then_ready_or_unreadable() {
    let readers = [reader("A", true)];

    assert_eq!(
        status_of(&readers, Listing::InProgress),
        ReaderStatus::Reading
    );
    assert_eq!(
        status_of(&readers, Listing::Done(Some(ReadyCard::Other))),
        ReaderStatus::Ready(ReadyCard::Other)
    );
    assert_eq!(
        status_of(&readers, Listing::Done(None)),
        ReaderStatus::Unreadable
    );
}

#[test]
fn the_most_advanced_status_sums_up_several_readers() {
    use ReaderStatus::{NoCard, NoReader, Reading, Ready, Unreadable};
    let cases = [
        (vec![NoCard, Reading, Ready(ReadyCard::Dnie)], Reading),
        (
            vec![Unreadable, Ready(ReadyCard::Other), NoCard],
            Ready(ReadyCard::Other),
        ),
        (vec![NoCard, Unreadable], Unreadable),
        (vec![NoCard, NoCard], NoCard),
        (
            vec![Ready(ReadyCard::Other), Ready(ReadyCard::Dnie)],
            Ready(ReadyCard::Dnie),
        ),
        (vec![], NoReader),
    ];

    for (statuses, expected) in cases {
        assert_eq!(
            ReaderStatus::most_advanced(statuses.clone()),
            expected,
            "{statuses:?}"
        );
    }
}

#[test]
fn an_empty_reader_next_to_a_card_being_read_still_reads() {
    assert_eq!(
        status_of(
            &[reader("A", false), reader("B", true)],
            Listing::InProgress
        ),
        ReaderStatus::Reading
    );
}

#[test]
fn the_dnie_is_recognised_by_its_issuer_whatever_the_number_of_its_ca() {
    for number in ["001", "004", "006", "010"] {
        let issuer = format!("CN=AC DNIE {number},OU=DNIE,O=DIRECCION GENERAL DE LA POLICIA,C=ES");
        assert!(is_issued_for_a_dnie(&issuer), "{issuer}");
    }
}

#[test]
fn an_issuer_that_lacks_any_part_of_the_dnie_shape_is_not_a_dnie() {
    for issuer in [
        "CN=AC DNIE 004,OU=DNIE,O=DIRECCION GENERAL DE LA POLICIA,C=PT",
        "CN=AC DNIE 004,OU=OTRA,O=DIRECCION GENERAL DE LA POLICIA,C=ES",
        "CN=AC DNIE 004,OU=DNIE,O=OTRA ENTIDAD,C=ES",
        "CN=AC FNMT Usuarios,OU=Ceres,O=FNMT-RCM,C=ES",
        "",
    ] {
        assert!(!is_issued_for_a_dnie(issuer), "{issuer}");
    }
}
