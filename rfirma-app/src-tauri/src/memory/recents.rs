//! Gestión del historial de documentos recientes con metadatos cacheados (ADR-0010, ADR-0011).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::signing::PageSet;

/// Capacidad máxima del historial de documentos recientes.
pub const CAPACITY: usize = 10;

/// Estado de firma persistido en caché para un documento reciente.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Badge {
    /// Documento con al menos una firma.
    Signed,
    /// Documento sin firmas.
    Unsigned,
}

/// Estado de firma visualizado en la interfaz, incluyendo disponibilidad actual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShownBadge {
    /// Documento con al menos una firma.
    Signed,
    /// Documento sin firmas.
    Unsigned,
    /// El fichero ya no está accesible en la ruta registrada.
    Unavailable,
}

/// Posición y páginas configuradas para la firma visible en un documento (ADR-0006).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Placement {
    /// Esquina inferior izquierda, eje X, en espacio de usuario PDF.
    pub lower_left_x: f64,
    /// Esquina inferior izquierda, eje Y, en espacio de usuario PDF.
    pub lower_left_y: f64,
    /// Páginas en las que estampar la firma visible.
    pub pages: PageSet,
}

impl<'de> Deserialize<'de> for Placement {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Stored {
            lower_left_x: f64,
            lower_left_y: f64,
            pages: Option<PageSet>,
            page: Option<u32>,
        }

        let stored = Stored::deserialize(deserializer)?;
        let pages = stored
            .pages
            .or_else(|| stored.page.map(PageSet::only_page))
            .ok_or_else(|| serde::de::Error::missing_field("pages"))?;
        Ok(Self {
            lower_left_x: stored.lower_left_x,
            lower_left_y: stored.lower_left_y,
            pages,
        })
    }
}

/// Metadatos cacheados de un documento reciente.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentDocument {
    path: PathBuf,
    name: String,
    badge: Badge,
    modified: Option<u64>,
    last_used: u64,
    #[serde(default)]
    placement: Option<Placement>,
}

impl RecentDocument {
    /// Construye una entrada reciente a partir de una ruta verificada.
    pub fn seen(path: &Path, badge: Badge, at: SystemTime) -> std::io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(seconds_since_epoch);
        Ok(Self {
            name: file_name(&path),
            path,
            badge,
            modified,
            last_used: seconds_since_epoch(at).unwrap_or_default(),
            placement: None,
        })
    }

    /// Ruta canónica del documento.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nombre de fichero para visualización.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Insignia de firma en caché.
    pub fn badge(&self) -> Badge {
        self.badge
    }

    /// Fecha de última modificación registrada en segundos desde la época UNIX.
    pub fn modified(&self) -> Option<u64> {
        self.modified
    }

    /// Fecha de último acceso registrada en segundos desde la época UNIX.
    pub fn last_used(&self) -> u64 {
        self.last_used
    }

    /// Posición del recuadro visible configurada en este documento.
    pub fn placement(&self) -> Option<&Placement> {
        self.placement.as_ref()
    }

    /// Asigna o elimina la posición del recuadro visible para este documento.
    pub fn place(&mut self, placement: Option<Placement>) {
        self.placement = placement;
    }

    /// Comprueba si el fichero existe actualmente en la ruta registrada.
    pub fn is_available(&self) -> bool {
        self.path.exists()
    }

    /// Estado de firma calculado para visualización.
    pub fn shown_badge(&self) -> ShownBadge {
        if !self.is_available() {
            return ShownBadge::Unavailable;
        }
        match self.badge {
            Badge::Signed => ShownBadge::Signed,
            Badge::Unsigned => ShownBadge::Unsigned,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn seconds_since_epoch(instant: SystemTime) -> Option<u64> {
    instant
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

/// Colección acotada de documentos recientes ordenados por fecha de uso.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Recents {
    entries: Vec<RecentDocument>,
}

impl<'de> Deserialize<'de> for Recents {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = Vec::<serde_json::Value>::deserialize(deserializer)?;
        let mut entries: Vec<RecentDocument> = rows
            .into_iter()
            .filter_map(|row| serde_json::from_value(row).ok())
            .collect();
        entries.truncate(CAPACITY);
        Ok(Self { entries })
    }
}

impl Recents {
    /// Registra un documento reciente colocándolo en cabeza y conservando posición previa.
    pub fn record(&mut self, mut document: RecentDocument) {
        let remembered = self
            .entries
            .iter()
            .find(|entry| entry.path == document.path)
            .and_then(|entry| entry.placement.clone());
        if document.placement.is_none() {
            document.placement = remembered;
        }
        self.entries.retain(|entry| entry.path != document.path);
        self.entries.insert(0, document);
        self.entries.truncate(CAPACITY);
    }

    /// Obtiene la entrada correspondiente a una ruta si existe.
    pub fn entry(&self, path: &Path) -> Option<&RecentDocument> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Actualiza la posición de recuadro asociada a una ruta registrada.
    pub fn place(&mut self, path: &Path, placement: Option<Placement>) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) {
            entry.place(placement);
        }
    }

    /// Elimina las posiciones de recuadro configuradas en todos los recientes.
    pub fn forget_placements(&mut self) {
        for entry in &mut self.entries {
            entry.place(None);
        }
    }

    /// Lista ordenada de documentos recientes.
    pub fn entries(&self) -> &[RecentDocument] {
        &self.entries
    }

    /// Elimina una ruta concreta del historial de recientes.
    pub fn forget(&mut self, path: &Path) {
        self.entries.retain(|entry| entry.path != path);
    }

    /// Vacía todas las entradas del historial.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Número de entradas en el historial.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Comprueba si el historial está vacío.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
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
            RecentDocument::seen(&first, Badge::Signed, SystemTime::now())
                .expect("deberia anotarse"),
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
            RecentDocument::seen(&signed, Badge::Signed, SystemTime::now())
                .expect("deberia anotarse"),
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
        let mut written =
            serde_json::to_value(vec![seen(&document)]).expect("deberia serializarse");
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
}
