use super::*;
use std::time::Duration;

fn a_document(directory: &Path, name: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, b"%PDF-1.7 de prueba").expect("deberia escribirse");
    path
}

fn seen(path: &Path) -> RecentDocument {
    RecentDocument::seen(
        path,
        Badge::Unsigned,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1),
    )
    .expect("deberia anotarse")
}

#[test]
fn a_recent_is_identified_by_its_canonical_path() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_document(directory.path(), "contrato.pdf");
    let detour = directory.path().join("./contrato.pdf");

    let entry = seen(&detour);

    assert!(entry.path().is_absolute());
    assert_eq!(
        entry.path(),
        fs::canonicalize(&document).expect("deberia canonicalizarse")
    );
    assert_eq!(entry.name(), "contrato.pdf");
}

#[test]
fn a_recent_caches_what_the_row_needs_so_the_tray_paints_without_opening_it() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_document(directory.path(), "nomina.pdf");

    let entry = RecentDocument::seen(
        &document,
        Badge::Signed,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
    )
    .expect("deberia anotarse");

    assert_eq!(entry.badge(), Badge::Signed);
    assert_eq!(entry.name(), "nomina.pdf");
    assert!(entry.modified().is_some());
    assert_eq!(entry.last_used(), 1_700_000_000);
}

#[test]
fn a_path_that_no_longer_answers_stays_in_the_list_with_the_unavailable_badge() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_document(directory.path(), "en-el-usb.pdf");
    let mut recents = Recents::default();
    recents.record(seen(&document));

    fs::remove_file(&document).expect("deberia borrarse");

    assert_eq!(recents.len(), 1, "no se purga en silencio");
    let entry = &recents.entries()[0];
    assert!(!entry.is_available());
    assert_eq!(entry.shown_badge(), ShownBadge::Unavailable);
    assert_eq!(entry.badge(), Badge::Unsigned, "lo cacheado no se toca");
}

#[test]
fn an_available_document_shows_its_cached_badge() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_document(directory.path(), "firmado.pdf");

    let entry = RecentDocument::seen(&document, Badge::Signed, SystemTime::now())
        .expect("deberia anotarse");

    assert_eq!(entry.shown_badge(), ShownBadge::Signed);
}

#[test]
fn the_tray_keeps_ten_and_evicts_the_least_recently_used() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let mut recents = Recents::default();
    let mut documents = Vec::new();
    for index in 0..CAPACITY + 2 {
        let document = a_document(directory.path(), &format!("documento-{index}.pdf"));
        recents.record(seen(&document));
        documents.push(document);
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
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let mut written = Recents::default();
    for index in 0..CAPACITY + 5 {
        let document = a_document(directory.path(), &format!("de-fuera-{index}.pdf"));
        written.entries.push(seen(&document));
    }
    let json = serde_json::to_string(&written).expect("deberia serializarse");

    let read: Recents = serde_json::from_str(&json).expect("deberia leerse");

    assert_eq!(
        read.len(),
        CAPACITY,
        "el limite es del tipo, no de `record`"
    );
    assert_eq!(read.entries()[0].name(), "de-fuera-0.pdf");
}

#[test]
fn recording_a_document_that_was_already_there_moves_it_to_the_front() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let first = a_document(directory.path(), "primero.pdf");
    let second = a_document(directory.path(), "segundo.pdf");
    let mut recents = Recents::default();
    recents.record(seen(&first));
    recents.record(seen(&second));

    recents.record(
        RecentDocument::seen(&first, Badge::Signed, SystemTime::now()).expect("deberia anotarse"),
    );

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
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let original = a_document(directory.path(), "contrato.pdf");
    let signed = a_document(directory.path(), "contrato_firmado.pdf");
    let mut recents = Recents::default();

    recents.record(seen(&original));
    recents.record(
        RecentDocument::seen(&signed, Badge::Signed, SystemTime::now()).expect("deberia anotarse"),
    );

    assert_eq!(recents.len(), 2);
    assert_eq!(recents.entries()[0].name(), "contrato_firmado.pdf");
}

#[test]
fn the_user_can_drop_one_row_or_empty_the_whole_list() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let first = a_document(directory.path(), "uno.pdf");
    let second = a_document(directory.path(), "dos.pdf");
    let mut recents = Recents::default();
    recents.record(seen(&first));
    recents.record(seen(&second));

    recents.forget(&fs::canonicalize(&first).expect("deberia canonicalizarse"));
    assert_eq!(recents.len(), 1);

    recents.clear();
    assert!(recents.is_empty());
}

#[test]
fn a_path_that_cannot_be_canonicalised_never_enters_the_list() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    let failure = RecentDocument::seen(
        &directory.path().join("no-existe.pdf"),
        Badge::Unsigned,
        SystemTime::now(),
    );

    assert!(failure.is_err());
}

#[test]
fn reads_a_v0_2_row_as_the_set_of_the_one_page_it_named() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_document(directory.path(), "contrato.pdf");
    let mut written = serde_json::to_value(vec![seen(&document)]).expect("deberia serializarse");
    written[0]["placement"] = serde_json::json!({
        "page": 3,
        "lower_left_x": 48.0,
        "lower_left_y": 179.0,
    });

    let read: Recents = serde_json::from_value(written).expect("deberia leerse");

    let placement = read.entries()[0]
        .placement()
        .expect("la v0.2 la habia colocado");
    assert_eq!(placement.pages, PageSet::only_page(3));
    assert_eq!(placement.lower_left_x, 48.0);
}

#[test]
fn discards_a_row_it_cannot_read_without_dragging_the_others() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let first = a_document(directory.path(), "primero.pdf");
    let second = a_document(directory.path(), "segundo.pdf");
    let mut written =
        serde_json::to_value(vec![seen(&first), seen(&second)]).expect("deberia serializarse");
    written[0]["placement"] = serde_json::json!({ "no": "esto no lo lee nadie" });

    let read: Recents = serde_json::from_value(written).expect("deberia leerse");

    assert_eq!(read.len(), 1);
    assert_eq!(read.entries()[0].name(), "segundo.pdf");
}

#[test]
fn remembers_the_page_set_of_each_document() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_document(directory.path(), "expediente.pdf");
    let mut recents = Recents::default();
    let noted = seen(&document);
    let path = noted.path().to_path_buf();
    recents.record(noted);
    recents.place(
        &path,
        Some(Placement {
            lower_left_x: 48.0,
            lower_left_y: 179.0,
            pages: PageSet::only([3, 7, 9]).expect("no esta vacio"),
        }),
    );

    let json = serde_json::to_string(&recents).expect("deberia serializarse");
    let read: Recents = serde_json::from_str(&json).expect("deberia leerse");

    assert_eq!(
        read.entries()[0].placement().map(|spot| spot.pages.clone()),
        PageSet::only([3, 7, 9])
    );
}
