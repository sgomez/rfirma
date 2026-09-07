use super::*;
use crate::signing::domain::{PageSet, Spot};
use std::time::Duration;

const FOLDER: &str = "/home/quien/Contratos";
const MODIFIED: Option<u64> = Some(1_699_000_000);

fn a_document(name: &str) -> PathBuf {
    PathBuf::from(FOLDER).join(name)
}

fn seen(path: &Path) -> RecentDocument<Spot> {
    RecentDocument::seen(
        path.to_path_buf(),
        MODIFIED,
        Badge::Unsigned,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1),
    )
}

#[test]
fn a_recent_is_identified_by_the_canonical_path_it_was_given() {
    let entry = seen(&a_document("contrato.pdf"));

    assert!(entry.path().is_absolute());
    assert_eq!(entry.path(), a_document("contrato.pdf"));
    assert_eq!(entry.name(), "contrato.pdf");
}

#[test]
fn a_recent_caches_what_the_row_needs_so_the_tray_paints_without_opening_it() {
    let entry = RecentDocument::<Spot>::seen(
        a_document("nomina.pdf"),
        MODIFIED,
        Badge::Signed,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
    );

    assert_eq!(entry.badge(), Badge::Signed);
    assert_eq!(entry.name(), "nomina.pdf");
    assert_eq!(entry.modified(), MODIFIED);
    assert_eq!(entry.last_used(), 1_700_000_000);
}

#[test]
fn a_path_that_no_longer_answers_stays_in_the_list_with_the_unavailable_badge() {
    let mut recents = Recents::<Spot>::default();
    recents.record(seen(&a_document("en-el-usb.pdf")));

    assert_eq!(recents.len(), 1, "no se purga en silencio");
    let entry = &recents.entries()[0];
    assert!(!entry.is_available());
    assert_eq!(entry.shown_badge(), ShownBadge::Unavailable);
    assert_eq!(entry.badge(), Badge::Unsigned, "lo cacheado no se toca");
}

#[test]
fn an_available_document_shows_its_cached_badge() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = directory.path().join("firmado.pdf");
    std::fs::write(&document, b"%PDF-1.7 de prueba").expect("deberia escribirse");

    let entry = RecentDocument::<Spot>::seen(document, MODIFIED, Badge::Signed, SystemTime::now());

    assert_eq!(entry.shown_badge(), ShownBadge::Signed);
}

#[test]
fn the_tray_keeps_ten_and_evicts_the_least_recently_used() {
    let mut recents = Recents::<Spot>::default();
    for index in 0..CAPACITY + 2 {
        recents.record(seen(&a_document(&format!("documento-{index}.pdf"))));
    }

    assert_eq!(recents.len(), CAPACITY);
    assert_eq!(recents.entries()[0].name(), "documento-11.pdf");
    let names: Vec<&str> = recents.entries().iter().map(RecentDocument::name).collect();
    assert!(
        !names.contains(&"documento-0.pdf"),
        "el mas viejo se desaloja"
    );
    assert!(!names.contains(&"documento-1.pdf"));
}

#[test]
fn a_support_with_more_than_ten_entries_is_cut_down_when_it_is_read() {
    let mut written = Recents::<Spot>::default();
    for index in 0..CAPACITY + 5 {
        written
            .entries
            .push(seen(&a_document(&format!("de-fuera-{index}.pdf"))));
    }
    let json = serde_json::to_string(&written).expect("deberia serializarse");

    let read: Recents<Spot> = serde_json::from_str(&json).expect("deberia leerse");

    assert_eq!(
        read.len(),
        CAPACITY,
        "el limite es del tipo, no de `record`"
    );
    assert_eq!(read.entries()[0].name(), "de-fuera-0.pdf");
}

#[test]
fn recording_a_document_that_was_already_there_moves_it_to_the_front() {
    let first = a_document("primero.pdf");
    let mut recents = Recents::<Spot>::default();
    recents.record(seen(&first));
    recents.record(seen(&a_document("segundo.pdf")));

    recents.record(RecentDocument::seen(
        first,
        MODIFIED,
        Badge::Signed,
        SystemTime::now(),
    ));

    assert_eq!(recents.len(), 2, "la misma ruta canonica no se duplica");
    assert_eq!(recents.entries()[0].name(), "primero.pdf");
    assert_eq!(
        recents.entries()[0].badge(),
        Badge::Signed,
        "la insignia se refresca"
    );
}

#[test]
fn signing_puts_two_rows_in_the_tray_and_not_one_that_evolves() {
    let mut recents = Recents::<Spot>::default();

    recents.record(seen(&a_document("contrato.pdf")));
    recents.record(RecentDocument::seen(
        a_document("contrato_firmado.pdf"),
        MODIFIED,
        Badge::Signed,
        SystemTime::now(),
    ));

    assert_eq!(recents.len(), 2);
    assert_eq!(recents.entries()[0].name(), "contrato_firmado.pdf");
}

#[test]
fn the_user_can_drop_one_row_or_empty_the_whole_list() {
    let first = a_document("uno.pdf");
    let mut recents = Recents::<Spot>::default();
    recents.record(seen(&first));
    recents.record(seen(&a_document("dos.pdf")));

    recents.forget(&first);
    assert_eq!(recents.len(), 1);

    recents.clear();
    assert!(recents.is_empty());
}

#[test]
fn reads_a_v0_2_row_as_the_set_of_the_one_page_it_named() {
    let mut written = serde_json::to_value(vec![seen(&a_document("contrato.pdf"))])
        .expect("deberia serializarse");
    written[0]["placement"] = serde_json::json!({
        "page": 3,
        "lower_left_x": 48.0,
        "lower_left_y": 179.0,
    });

    let read: Recents<Spot> = serde_json::from_value(written).expect("deberia leerse");

    let placement = read.entries()[0]
        .placement()
        .expect("la v0.2 la habia colocado");
    assert_eq!(placement.pages, PageSet::only_page(3));
    assert_eq!(placement.lower_left_x, 48.0);
}

#[test]
fn discards_a_row_it_cannot_read_without_dragging_the_others() {
    let mut written = serde_json::to_value(vec![
        seen(&a_document("primero.pdf")),
        seen(&a_document("segundo.pdf")),
    ])
    .expect("deberia serializarse");
    written[0]["placement"] = serde_json::json!({ "no": "esto no lo lee nadie" });

    let read: Recents<Spot> = serde_json::from_value(written).expect("deberia leerse");

    assert_eq!(read.len(), 1);
    assert_eq!(read.entries()[0].name(), "segundo.pdf");
}

#[test]
fn remembers_the_page_set_of_each_document() {
    let mut recents = Recents::<Spot>::default();
    let noted = seen(&a_document("expediente.pdf"));
    let path = noted.path().to_path_buf();
    recents.record(noted);
    recents.place(
        &path,
        Some(Spot {
            lower_left_x: 48.0,
            lower_left_y: 179.0,
            pages: PageSet::only([3, 7, 9]).expect("no esta vacio"),
        }),
    );

    let json = serde_json::to_string(&recents).expect("deberia serializarse");
    let read: Recents<Spot> = serde_json::from_str(&json).expect("deberia leerse");

    assert_eq!(
        read.entries()[0].placement().map(|spot| spot.pages.clone()),
        PageSet::only([3, 7, 9])
    );
}
