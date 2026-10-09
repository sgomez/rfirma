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

fn in_windows(provider: &str, der: &[u8]) -> TokenCertificate {
    use crate::identity::domain::certificate::CertificateRef;
    use crate::identity::domain::store::Store;

    TokenCertificate::new(
        CertificateRef::new(
            Store::module("cng:CurrentUser/MY"),
            "CurrentUser\\MY",
            "x",
            der.to_vec(),
        )
        .with_key_provider(provider),
        der.to_vec(),
    )
}

#[test]
fn a_windows_copy_of_a_card_that_is_not_in_the_reader_is_left_out_and_the_rest_stay() {
    let on_the_card = in_windows(
        "Microsoft Smart Card Key Storage Provider",
        b"en la tarjeta",
    );
    let card_gone = in_windows(
        "Microsoft Smart Card Key Storage Provider",
        b"tarjeta fuera",
    );
    let in_software = in_windows("Microsoft Software Key Storage Provider", b"en el disco");

    let present = with_the_cards_present(
        vec![on_the_card, card_gone, in_software],
        &[b"en la tarjeta".to_vec()],
    );

    let ders: Vec<&[u8]> = present.iter().map(TokenCertificate::der).collect();
    assert_eq!(ders, vec![&b"en la tarjeta"[..], &b"en el disco"[..]]);
}
