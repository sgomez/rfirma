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
//!   inodos: rompen con las copias y con los sistemas de ficheros de red.
//! - **Una ruta que ya no responde no se purga en silencio.** La fila se marca
//!   [`ShownBadge::Unavailable`] y **sigue en la lista**: un PDF en un USB
//!   desmontado no está borrado, y decírselo al usuario es más útil que hacerlo
//!   desaparecer.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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

/// Un documento de la bandeja, con lo que hace falta para pintar la fila sin
/// abrirlo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentDocument {
    path: PathBuf,
    name: String,
    badge: Badge,
    modified: Option<u64>,
    last_used: u64,
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

    /// Cuándo se usó por última vez, en segundos desde la época. Es el criterio
    /// de desalojo.
    pub fn last_used(&self) -> u64 {
        self.last_used
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Recents {
    entries: Vec<RecentDocument>,
}

impl Recents {
    /// Anota un documento: pasa al frente, y si ya estaba —misma ruta
    /// canónica— sustituye a la entrada anterior en vez de duplicarla.
    ///
    /// Al firmar se anotan **dos**, el original y el firmado: son dos ficheros
    /// en el disco y fundirlos en una fila que «evoluciona» escondería cuál se
    /// va a mandar.
    pub fn record(&mut self, document: RecentDocument) {
        self.entries.retain(|entry| entry.path != document.path);
        self.entries.insert(0, document);
        self.entries.truncate(CAPACITY);
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
}
