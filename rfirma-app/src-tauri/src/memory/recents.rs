//! Los documentos recientes: **diez**, por ruta canónica, con metadatos
//! cacheados (ID-33, ADR-0010).
//!
//! Tres decisiones que parecen detalles y no lo son:
//!
//! - **Se cachean metadatos, no solo rutas.** Sin caché habría que parsear diez
//!   PDFs antes de pintar la bandeja, porque la insignia `Firmado`/`Sin firmar`
//!   no se deduce de la ruta. Se revalida solo el documento que se selecciona,
//!   comparando el `mtime`.
//! - **Se identifican por su ruta absoluta canónica.** Nada de hashes ni
//!   inodos: rompen con las copias y con los sistemas de ficheros de red. Es
//!   además el mismo criterio que usa el portal de flatpak, cuyo permiso va
//!   con la **ruta** y no con el inodo (ID-38, ADR-0011): las dos mitades de
//!   esa coincidencia están atadas por las pruebas de
//!   [`crate::destination::portal`].
//! - **Una ruta que ya no responde no se purga en silencio.** La fila se marca
//!   [`ShownBadge::Unavailable`] y **sigue en la lista**: un PDF en un USB
//!   desmontado no está borrado, y decírselo al usuario es más útil que hacerlo
//!   desaparecer.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::signing::PageSet;

/// Cuántos se recuerdan. La bandeja no tiene buscador; si algún día hace falta
/// uno, este límite estaba mal.
pub const CAPACITY: usize = 10;

/// La insignia **guardada**. Solo puede ser una de estas dos: se conoce
/// abriendo el documento, y por eso se cachea.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Badge {
    /// Ya lleva al menos una firma.
    Signed,
    /// Todavía no lleva ninguna.
    Unsigned,
}

/// La insignia que **se pinta**, que es la guardada más el tercer valor que no
/// se guarda nunca porque depende del disco de ahora mismo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShownBadge {
    /// Ya lleva al menos una firma.
    Signed,
    /// Todavía no lleva ninguna.
    Unsigned,
    /// La ruta ya no responde. La fila se atenúa y ofrece quitarla; nadie la
    /// purga por su cuenta.
    Unavailable,
}

/// **Dónde cayó el recuadro en este documento**: el conjunto de páginas y la
/// esquina inferior izquierda, en espacio de usuario PDF (ID-74, ID-95).
///
/// Es la mitad **por documento** de lo que se recuerda de la firma visible. La
/// otra mitad —el interruptor, las cinco casillas, el motivo y el **tamaño**
/// del recuadro— es global y vive en
/// [`VisibleSignatureMemory`](super::state::VisibleSignatureMemory): reponer
/// sobre un documento nuevo una posición elegida para otro es justo lo que
/// rechaza el ID-22, mientras que el tamaño sí se hereda porque no depende de
/// la página.
///
/// **El conjunto de páginas también es por documento**: «las páginas 3, 7 y 9»
/// no significa nada en otro PDF.
///
/// No lleva el tamaño **a propósito**: dos sitios donde guardar el mismo ancho
/// es un sitio donde divergen.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Placement {
    /// Esquina inferior izquierda, eje X, en espacio de usuario PDF.
    pub lower_left_x: f64,
    /// Esquina inferior izquierda, eje Y, en espacio de usuario PDF.
    pub lower_left_y: f64,
    /// En qué páginas se estampa.
    pub pages: PageSet,
}

impl<'de> Deserialize<'de> for Placement {
    /// **Lee también las filas que dejó v0.2**, que guardaban `page` en vez de
    /// `pages` (ID-95).
    ///
    /// `{ page: 3 }` significaba exactamente `{ pages: [3] }`, así que se lee
    /// como tal y **no se versiona el formato**: una versión para un campo que
    /// se traduce solo sería una versión que hay que subir la próxima vez.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Stored {
            lower_left_x: f64,
            lower_left_y: f64,
            /// v0.3.
            pages: Option<PageSet>,
            /// v0.2: una sola página.
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

/// Un documento de la bandeja, con lo que hace falta para pintar la fila sin
/// abrirlo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentDocument {
    path: PathBuf,
    name: String,
    badge: Badge,
    modified: Option<u64>,
    last_used: u64,
    /// Dónde cayó el recuadro en **este** documento. `None` mientras nadie lo
    /// haya colocado, y también cuando «Recordar la última configuración de
    /// firma visible» está apagado: apagado significa **no guardarla**.
    #[serde(default)]
    placement: Option<Placement>,
}

impl RecentDocument {
    /// El documento que se acaba de usar, leyendo del disco lo que se cachea.
    ///
    /// **Canonicaliza la ruta**, y por eso puede fallar: se construye cuando el
    /// documento se abre o se firma, o sea cuando está ahí. Una ruta que no se
    /// puede canonicalizar no entra en la lista, porque no se sabría contra qué
    /// comparar la siguiente.
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

    /// La ruta canónica, que es lo que identifica la fila.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// El nombre del fichero, cacheado para pintar la fila.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// La insignia cacheada.
    pub fn badge(&self) -> Badge {
        self.badge
    }

    /// El `mtime` cacheado, en segundos desde la época. Es contra este valor
    /// contra el que se revalida el documento al seleccionarlo.
    pub fn modified(&self) -> Option<u64> {
        self.modified
    }

    /// Cuándo se usó por última vez, en segundos desde la época.
    ///
    /// Es **dato para pintar la fila**, no el criterio de desalojo: el desalojo
    /// es por posición en la lista, y la posición la fija [`Recents::record`]
    /// insertando al frente. Hoy las dos cosas coinciden porque `record` es la
    /// única mutación; si algún día algo inserta con un `last_used` viejo —una
    /// fusión de dos bandejas, una migración—, el que manda seguirá siendo el
    /// orden, y entonces habrá que decidir cuál de los dos es la verdad.
    pub fn last_used(&self) -> u64 {
        self.last_used
    }

    /// Dónde cayó el recuadro en este documento, si alguien lo colocó.
    pub fn placement(&self) -> Option<&Placement> {
        self.placement.as_ref()
    }

    /// Coloca —o descoloca— el recuadro de este documento.
    pub fn place(&mut self, placement: Option<Placement>) {
        self.placement = placement;
    }

    /// Si la ruta responde ahora mismo.
    pub fn is_available(&self) -> bool {
        self.path.exists()
    }

    /// La insignia que se pinta: la cacheada, o `No disponible` si la ruta ya
    /// no responde.
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

/// La bandeja: como mucho [`CAPACITY`] documentos, el más reciente primero.
///
/// El límite es una invariante del tipo y no una propiedad de [`Recents::record`]:
/// también se aplica **al deserializar**, para que un `state.json` con quince
/// entradas —editado a mano, escrito por una rFirma futura con otro límite,
/// fusionado por un sincronizador— no pinte quince filas hasta el siguiente
/// `record`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Recents {
    entries: Vec<RecentDocument>,
}

impl<'de> Deserialize<'de> for Recents {
    /// **Una fila que no se sepa leer se descarta, y las demás siguen**
    /// (ID-95): la bandeja es actividad, no datos del usuario, y perder las
    /// diez porque una traía un campo raro es perder nueve por nada.
    ///
    /// Por eso las filas se leen de una en una desde su JSON en vez de
    /// deserializar el vector entero: un `Vec<RecentDocument>` falla entero al
    /// primer elemento que no encaje.
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
    /// Anota un documento: pasa al frente, y si ya estaba —misma ruta
    /// canónica— sustituye a la entrada anterior en vez de duplicarla.
    ///
    /// Al firmar se anotan **dos**, el original y el firmado: son dos ficheros
    /// en el disco y fundirlos en una fila que «evoluciona» escondería cuál se
    /// va a mandar.
    /// Una fila que vuelve a anotarse **conserva su recuadro**: la posición es
    /// del documento, no de la apertura, y el identificador opaco cambia en
    /// cada concesión del portal (ID-62). Sin esto, reabrir el mismo contrato
    /// borraría dónde había caído su recuadro, que es lo contrario del ID-74.
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

    /// La fila de una ruta, si está.
    pub fn entry(&self, path: &Path) -> Option<&RecentDocument> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Coloca el recuadro de una fila. Si la ruta no está, no hace nada: la
    /// fila la crea [`Recents::record`] al abrir el documento.
    pub fn place(&mut self, path: &Path, placement: Option<Placement>) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) {
            entry.place(placement);
        }
    }

    /// Descoloca **todos** los recuadros, dejando las filas donde están.
    ///
    /// Es lo que hace «Recordar la última configuración de firma visible» al
    /// apagarse: la bandeja es actividad y la sigue guardando «Recordar mi
    /// actividad», pero dónde cayó el recuadro es firma visible y apagado
    /// significa no guardarlo (ID-74).
    pub fn forget_placements(&mut self) {
        for entry in &mut self.entries {
            entry.place(None);
        }
    }

    /// Los documentos, del más reciente al más antiguo.
    pub fn entries(&self) -> &[RecentDocument] {
        &self.entries
    }

    /// Quita una fila concreta. Es lo que ofrece la fila `No disponible`, y es
    /// el usuario quien lo pide.
    pub fn forget(&mut self, path: &Path) {
        self.entries.retain(|entry| entry.path != path);
    }

    /// «Vaciar la lista»: hoy no, mañana sí. No apaga ningún interruptor.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Cuántos hay.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Si no hay ninguno.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// **Grada A**: ficheros de verdad en un directorio temporal, que es lo que
    /// hace falta para canonicalizar una ruta y para que deje de existir.
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
            // Sin pasar por `record`: es lo que hace quien edita el fichero a
            // mano o una rFirma con otro limite.
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

    /// **ID-95**: la fila que v0.2 guardó con `page` se lee como el conjunto de
    /// esa única página, que es exactamente lo que significaba. **No hay
    /// migración ni versión de formato**: el campo se traduce al leerlo.
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

    /// La otra mitad del ID-95: una fila ilegible **se descarta sola**, y las
    /// demás llegan. Perder las diez porque una traía un campo que nadie sabe
    /// leer es perder nueve por nada.
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

    /// El conjunto de páginas es **de este documento** (ID-95), así que viaja
    /// en la fila y vuelve entero.
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
