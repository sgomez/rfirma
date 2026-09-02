//! **La bandeja, del disco a la ventana y de vuelta** (ID-75, ADR-0010).
//!
//! Las reglas de la bandeja —capacidad diez, deduplicación por ruta canónica,
//! insignia `No disponible`— ya estaban en [`crate::memory::recents`]. Lo que
//! hay aquí es lo que faltaba: **quién la lee y quién la escribe**, y las dos
//! traducciones que eso exige.
//!
//! # La fila se guarda por ruta y cruza por identificador
//!
//! La bandeja deduplica por la ruta canónica, que **solo Rust conoce** y que no
//! sale de este proceso (ADR-0011). Lo que cruza a la ventana es el
//! identificador opaco que acuñó [`OpenedDocuments`] (ID-62), así que listar
//! implica darle uno a cada fila: el de la concesión del portal que ya haya, y
//! si no lo hay, uno recién acuñado contra la ruta guardada. Sin eso, la fila
//! se pintaría pero no se podría abrir.
//!
//! # El recuadro se guarda partido y cruza entero
//!
//! El ID-74 reparte lo que se recuerda de la firma visible: el **tamaño** es
//! global y la **página y la posición** son de cada documento. La ventana, en
//! cambio, pinta un rectángulo. Juntar las dos mitades al salir y volver a
//! partirlas al entrar es de aquí, y de ningún otro sitio: hacerlo en
//! TypeScript sería tener el reparto del ID-74 en los dos lados.
//!
//! # `available` no se persiste
//!
//! Se recalcula en cada listado contra el disco de ahora mismo. Una ruta que no
//! responde da `available: false` —la ventana la pinta `No disponible`— y la
//! fila **revive** cuando la ruta reaparece. Nadie la purga por su cuenta: eso
//! solo lo hace [`forget`].

use std::path::Path;
use std::time::SystemTime;

use crate::commands::views::{Failure, PlacementView, RecentDocumentView};
use crate::destination::PortalDocument;
use crate::memory::{
    Badge, BoxSize, Configuration, Memory, OpenedDocuments, Placement, RecentDocument, State,
};

/// **Caso de uso.** La bandeja entera, la más reciente primero.
///
/// No abre ni un PDF: se pinta con lo cacheado (ADR-0010). Lo único que toca el
/// disco es comprobar si cada ruta sigue respondiendo.
pub fn listed_rows(memory: &Memory, opened: &OpenedDocuments) -> Vec<RecentDocumentView> {
    let state = loaded_state(memory);
    let size = state
        .visible_signature
        .as_ref()
        .map(|remembered| remembered.size)
        .unwrap_or_default();
    state
        .recents
        .entries()
        .iter()
        .map(|entry| told_as_row(entry, size, opened))
        .collect()
}

/// **Caso de uso.** Anota el documento abierto en la bandeja y devuelve su fila
/// ya lista para pintar.
///
/// Devuelve la fila y no nada porque es donde la ventana recupera **lo que ya
/// se sabía de ese documento**: su insignia cacheada y dónde había caído su
/// recuadro. El identificador que sale es el mismo que entró, así que la fila
/// activa sigue siendo la misma para la ventana.
///
/// La insignia que se escribe es la que la fila ya tuviera, y `Sin firmar` si
/// es nueva: **`Firmado` solo lo escribe [`super::signing::finish`]** (ID-76).
/// Un PDF que ya venga con firmas entra `Sin firmar` a propósito —contar las
/// firmas de un PDF ajeno es de v1.0—, y reabrir uno que rFirma firmó no le
/// quita la suya.
pub fn record(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    id: &str,
    placement: Option<PlacementView>,
) -> Result<RecentDocumentView, Failure> {
    let document = opened
        .get(id)
        .ok_or_else(|| Failure::new("documentUnreadable", format!("no hay documento «{id}»")))?;
    let path = document.reading_path().to_path_buf();
    let mut state = loaded_state(memory);
    let badge = state
        .recents
        .entry(&path)
        .map_or(Badge::Unsigned, RecentDocument::badge);
    let noted = RecentDocument::seen(&path, badge, SystemTime::now())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    let canonical = noted.path().to_path_buf();
    state.recents.record(noted);
    if let Some(placement) = placement {
        let (spot, size) = split(placement);
        state.recents.place(&canonical, Some(spot));
        remember_the_size(&mut state, size);
    }
    memory.remember_state(configuration, &state)?;
    let size = state
        .visible_signature
        .as_ref()
        .map(|remembered| remembered.size)
        .unwrap_or_default();
    let entry = state
        .recents
        .entry(&canonical)
        .expect("la fila acaba de anotarse");
    Ok(RecentDocumentView {
        id: id.to_owned(),
        ..told_as_row(entry, size, opened)
    })
}

/// **Caso de uso.** Quita una fila de la bandeja.
///
/// Es lo único que la saca: una ruta que no responde **sigue en la lista** y es
/// el usuario quien decide quitarla. Vaciar la lista entera es otra orden,
/// `forget_activity`, porque se lleva también el certificado.
pub fn forget(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    id: &str,
) -> Result<(), Failure> {
    let document = opened
        .get(id)
        .ok_or_else(|| Failure::new("documentUnreadable", format!("no hay documento «{id}»")))?;
    let mut state = loaded_state(memory);
    state
        .recents
        .forget(&canonical_or_raw(document.reading_path()));
    memory.remember_state(configuration, &state)?;
    Ok(())
}

/// La ruta canónica de `path`, y `path` tal cual si no se puede canonicalizar.
///
/// Las filas se guardan **siempre** por ruta canónica ([`RecentDocument::seen`]),
/// así que buscar una por la ruta cruda no la encuentra en cuanto el camino pasa
/// por un enlace simbólico. Canonicalizar falla cuando la ruta ya no responde, y
/// ahí la cruda es lo único que hay: es justo la fila `No disponible` que el
/// usuario quiere quitar, y compararla cruda es lo que la encuentra si se guardó
/// sin enlaces por medio.
fn canonical_or_raw(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Anota el documento **recién firmado** con la insignia `Firmado`.
///
/// Es el único sitio de toda la aplicación donde se escribe esa insignia, y lo
/// llama [`super::signing::finish`] después de que el PDF haya caído
/// (ID-76). Un fallo al escribir el estado **no tumba la firma**: el documento
/// ya está en el disco, y perder su fila es una molestia, no un error de firma.
pub fn note_signed(memory: &Memory, configuration: &Configuration, landing: &Path) {
    let Ok(noted) = RecentDocument::seen(landing, Badge::Signed, SystemTime::now()) else {
        return;
    };
    let mut state = loaded_state(memory);
    state.recents.record(noted);
    let _ = memory.remember_state(configuration, &state);
}

/// El estado guardado, o el de un primer arranque si no se pudo leer.
///
/// Un `state.json` ilegible ya se apartó al leerlo ([`crate::memory::store`]) y
/// no puede impedir que la bandeja se pinte: lo que se pierde es lo que había
/// dentro, no la sesión.
fn loaded_state(memory: &Memory) -> State {
    memory
        .state()
        .map(crate::memory::Loaded::into_value)
        .unwrap_or_default()
}

/// La fila tal como la ventana la recibe, con `available` recalculado y el
/// recuadro ya entero.
fn told_as_row(
    entry: &RecentDocument,
    size: BoxSize,
    opened: &OpenedDocuments,
) -> RecentDocumentView {
    RecentDocumentView {
        id: identifier_for(entry.path(), opened),
        name: entry.name().to_owned(),
        badge: entry.badge(),
        modified: entry.modified(),
        last_used: entry.last_used(),
        available: entry.is_available(),
        placement: entry.placement().map(|spot| joined(spot, size)),
    }
}

/// El identificador con el que esa ruta cruza: el de la concesión que ya haya,
/// y si no, uno recién acuñado.
///
/// Reusarlo importa: la ventana tiene delante el identificador que le dio
/// `open_document`, y acuñar otro para la misma fila la dejaría sin reconocer
/// cuál de la bandeja es la que está firmando.
fn identifier_for(path: &Path, opened: &OpenedDocuments) -> String {
    opened
        .last_id_of(path)
        .unwrap_or_else(|| opened.remember(PortalDocument::opened(path.to_path_buf())))
}

/// Las dos mitades juntas: la esquina del documento y el tamaño global.
fn joined(spot: Placement, size: BoxSize) -> PlacementView {
    PlacementView {
        page: spot.page,
        rect: [
            spot.lower_left_x,
            spot.lower_left_y,
            spot.lower_left_x + size.width,
            spot.lower_left_y + size.height,
        ],
    }
}

/// Y el reparto de vuelta: la esquina es del documento, el tamaño es de todos.
fn split(placement: PlacementView) -> (Placement, BoxSize) {
    let [x0, y0, x1, y1] = placement.rect;
    (
        Placement {
            page: placement.page,
            lower_left_x: x0,
            lower_left_y: y0,
        },
        BoxSize {
            width: x1 - x0,
            height: y1 - y0,
        },
    )
}

/// Guarda el tamaño en lo global sin tocar el resto de lo que hubiera.
fn remember_the_size(state: &mut State, size: BoxSize) {
    state.visible_signature.get_or_insert_default().size = size;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fixtures::a_memory;
    use crate::memory::Loaded;
    use std::fs;
    use std::path::PathBuf;

    /// **Grada A**: ficheros de verdad en un directorio temporal. Ni token, ni
    /// librería nativa, ni red.
    fn a_pdf(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = directory.join(name);
        fs::write(&path, bytes).expect("deberia escribirse");
        path
    }

    fn an_opened_pdf(directory: &Path, name: &str, opened: &OpenedDocuments) -> (PathBuf, String) {
        let path = a_pdf(directory, name, b"%PDF-1.7 de prueba");
        let id = opened.remember(PortalDocument::opened(path.clone()));
        (path, id)
    }

    fn a_placement(page: u32) -> PlacementView {
        PlacementView {
            page,
            rect: [72.0, 500.0, 272.0, 600.0],
        }
    }

    #[test]
    fn the_tray_survives_being_read_again_with_its_names_badges_and_order() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        let (_, second) = an_opened_pdf(directory.path(), "nomina.pdf", &opened);

        record(&memory, &configuration, &opened, &first, None).expect("deberia anotarse");
        record(&memory, &configuration, &opened, &second, None).expect("deberia anotarse");

        // Otra sesión: otro registro de documentos abiertos, la misma memoria.
        let next_session = OpenedDocuments::new();
        let rows = listed_rows(&memory, &next_session);

        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["nomina.pdf", "contrato.pdf"]);
        assert!(rows.iter().all(|row| row.badge == Badge::Unsigned));
        assert!(rows.iter().all(|row| row.available));
    }

    #[test]
    fn a_path_that_no_longer_answers_is_unavailable_and_revives_when_it_comes_back() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &configuration, &opened, &id, None).expect("deberia anotarse");

        fs::remove_file(&path).expect("deberia borrarse");
        let gone = listed_rows(&memory, &opened);
        fs::write(&path, b"%PDF-1.7 de vuelta").expect("deberia volver");
        let back = listed_rows(&memory, &opened);

        assert_eq!(gone.len(), 1, "nadie la purga por su cuenta");
        assert!(!gone[0].available);
        assert!(back[0].available, "la fila revive cuando la ruta reaparece");
    }

    #[test]
    fn availability_is_never_written_to_the_disk() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let opened = OpenedDocuments::new();
        let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);

        record(&memory, &Configuration::default(), &opened, &id, None).expect("deberia anotarse");

        let written = fs::read_to_string(memory.state_file().path()).expect("deberia leerse");
        assert!(
            !written.contains("available"),
            "«available» es un hecho del disco de ahora mismo y se recalcula al listar: {written}"
        );
    }

    #[test]
    fn only_forget_takes_a_row_out() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        let (_, second) = an_opened_pdf(directory.path(), "nomina.pdf", &opened);
        record(&memory, &configuration, &opened, &first, None).expect("deberia anotarse");
        record(&memory, &configuration, &opened, &second, None).expect("deberia anotarse");

        forget(&memory, &configuration, &opened, &first).expect("deberia olvidarse");

        let names: Vec<String> = listed_rows(&memory, &opened)
            .into_iter()
            .map(|row| row.name)
            .collect();
        assert_eq!(names, vec!["nomina.pdf".to_owned()]);
    }

    /// La fila se guarda por ruta canónica, así que quitarla comparando la ruta
    /// cruda que abrió la ventana era un no-op silencioso en cuanto el camino
    /// pasaba por un enlace simbólico.
    #[test]
    fn a_row_opened_through_a_symlink_is_still_the_row_that_forget_takes_out() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let real = directory.path().join("real");
        fs::create_dir(&real).expect("deberia crearse");
        let linked = directory.path().join("enlace");
        std::os::unix::fs::symlink(&real, &linked).expect("deberia enlazarse");
        a_pdf(&real, "contrato.pdf", b"%PDF-1.7 de prueba");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let id = opened.remember(PortalDocument::opened(linked.join("contrato.pdf")));
        record(&memory, &configuration, &opened, &id, None).expect("deberia anotarse");

        forget(&memory, &configuration, &opened, &id).expect("deberia olvidarse");

        assert!(listed_rows(&memory, &opened).is_empty());
    }

    #[test]
    fn a_document_that_was_open_before_gets_its_page_and_position_back() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &configuration, &opened, &id, Some(a_placement(3)))
            .expect("deberia anotarse");

        // Otra apertura del mismo fichero: identificador nuevo (ID-62), misma
        // fila.
        let again = opened.remember(PortalDocument::opened(path));
        let row = record(&memory, &configuration, &opened, &again, None).expect("deberia anotarse");

        assert_eq!(row.placement, Some(a_placement(3)));
    }

    #[test]
    fn a_brand_new_document_does_not_inherit_the_position_of_another_one() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(
            &memory,
            &configuration,
            &opened,
            &first,
            Some(a_placement(3)),
        )
        .expect("deberia anotarse");

        let (_, second) = an_opened_pdf(directory.path(), "nomina.pdf", &opened);
        let row =
            record(&memory, &configuration, &opened, &second, None).expect("deberia anotarse");

        assert_eq!(row.placement, None, "eso es lo que rechaza el ID-22");
    }

    #[test]
    fn with_the_visible_signature_switch_off_the_box_starts_at_its_default_every_time() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration {
            remember_visible_signature: false,
            ..Configuration::default()
        };
        let opened = OpenedDocuments::new();
        let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &configuration, &opened, &id, Some(a_placement(3)))
            .expect("deberia anotarse");

        let again = opened.remember(PortalDocument::opened(path));
        let row = record(&memory, &configuration, &opened, &again, None).expect("deberia anotarse");

        assert_eq!(row.placement, None);
        let state = memory
            .state()
            .map(Loaded::into_value)
            .expect("deberia leerse");
        assert!(state.visible_signature.is_none(), "lo global tampoco");
    }

    #[test]
    fn a_pdf_that_already_carries_signatures_still_enters_as_unsigned() {
        // Contar las firmas de un PDF ajeno es la ficha 14 y es de v1.0
        // (ID-76): lo que entra es lo que se sabe, y de un PDF que rFirma no
        // ha firmado se sabe que no lo ha firmado.
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let opened = OpenedDocuments::new();
        let path = a_pdf(
            directory.path(),
            "ya-firmado.pdf",
            b"%PDF-1.7\n/ByteRange [0 1000 2000 3000]\n/SubFilter /ETSI.CAdES.detached\n",
        );
        let id = opened.remember(PortalDocument::opened(path));

        let row = record(&memory, &Configuration::default(), &opened, &id, None)
            .expect("deberia anotarse");

        assert_eq!(row.badge, Badge::Unsigned);
    }

    #[test]
    fn the_signed_document_is_the_only_row_that_gets_the_signed_badge() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &configuration, &opened, &id, None).expect("deberia anotarse");
        let landing = a_pdf(
            directory.path(),
            "contrato_firmado.pdf",
            b"%PDF-1.7 firmado",
        );

        note_signed(&memory, &configuration, &landing);

        let rows = listed_rows(&memory, &opened);
        let signed: Vec<&str> = rows
            .iter()
            .filter(|row| row.badge == Badge::Signed)
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(signed, vec!["contrato_firmado.pdf"]);
        assert_eq!(rows.len(), 2, "el original y el firmado son dos ficheros");
    }

    #[test]
    fn reopening_a_document_that_rfirma_signed_does_not_take_its_badge_away() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let landing = a_pdf(
            directory.path(),
            "contrato_firmado.pdf",
            b"%PDF-1.7 firmado",
        );
        note_signed(&memory, &configuration, &landing);

        let id = opened.remember(PortalDocument::opened(landing));
        let row = record(&memory, &configuration, &opened, &id, None).expect("deberia anotarse");

        assert_eq!(row.badge, Badge::Signed);
    }

    #[test]
    fn no_row_carries_the_path_the_backend_dedupes_by() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let opened = OpenedDocuments::new();
        let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &Configuration::default(), &opened, &id, None).expect("deberia anotarse");

        let rows = listed_rows(&memory, &opened);

        let told = serde_json::to_string(&rows).expect("deberia serializarse");
        assert!(
            !told.contains(&path.to_string_lossy().into_owned()),
            "lo que cruza es el identificador opaco y nada mas: {told}"
        );
        assert!(!told.contains(&directory.path().to_string_lossy().into_owned()));
    }

    #[test]
    fn a_listed_row_can_be_read_because_it_carries_a_usable_identifier() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let opened = OpenedDocuments::new();
        let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &Configuration::default(), &opened, &id, None).expect("deberia anotarse");

        // Sesión nueva: el registro de abiertos empieza vacío y la fila viene
        // del disco.
        let next_session = OpenedDocuments::new();
        let rows = listed_rows(&memory, &next_session);

        let bytes = super::super::documents::bytes_of(&next_session, &rows[0].id)
            .expect("la fila listada tiene que poder abrirse");
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn the_row_of_the_document_in_front_keeps_the_identifier_the_window_already_has() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let opened = OpenedDocuments::new();
        let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(&memory, &Configuration::default(), &opened, &id, None).expect("deberia anotarse");

        let rows = listed_rows(&memory, &opened);

        assert_eq!(rows[0].id, id);
    }

    #[test]
    fn the_size_is_global_and_the_position_is_of_each_document() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(directory.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
        record(
            &memory,
            &configuration,
            &opened,
            &first,
            Some(a_placement(1)),
        )
        .expect("deberia anotarse");

        let state = memory
            .state()
            .map(Loaded::into_value)
            .expect("deberia leerse");
        let global = state.visible_signature.expect("el tamano es global");
        assert_eq!(global.size.width, 200.0);
        assert_eq!(global.size.height, 100.0);
        let spot = state.recents.entries()[0]
            .placement()
            .expect("la posicion es de este documento");
        assert_eq!((spot.page, spot.lower_left_x), (1, 72.0));
    }
}
