//! El documento **tal y como entra**: por el portal, y nada más (ID-37,
//! ADR-0011, #22).
//!
//! Bajo el sandbox el diálogo de abrir no devuelve la ruta del usuario, sino
//! un enlace del portal de documentos:
//!
//! ```text
//! ruta dentro del sandbox: /run/user/1000/doc/1e8b83b9/original.pdf
//! directorio padre       : /run/user/1000/doc/1e8b83b9
//! contenido del padre    : original.pdf
//! ```
//!
//! Ese directorio padre **no es la carpeta del usuario**: contiene un solo
//! fichero, el que se acaba de conceder. De ahí las dos cosas que este tipo
//! existe para hacer imposibles:
//!
//! - **Escribir un hermano parece funcionar y no funciona.** Deja en la
//!   carpeta real un `.xdp-original-firmado.pdf-5OUkyi` que nunca se renombra,
//!   sin dar error. Por eso [`PortalDocument`] no tiene ningún método que
//!   devuelva un directorio: lo único que se puede sacar de él es un
//!   [`nombre`](PortalDocument::name) y una ruta **para leer**. Dónde cae lo
//!   firmado lo decide [`super::CheckedFolder`], que solo se construye sobre
//!   la carpeta de destino.
//! - **La ruta original no se puede averiguar**, y tampoco hace falta.
//!   `org.freedesktop.portal.Documents.Info` y `.Lookup` contestan
//!   `Not allowed in sandbox`, y el portal solo da la ruta real a un llamante
//!   `is_host`, que un flatpak nunca es. `--filesystem=home` **no lo
//!   arreglaría**: queda cerrado por no funcionar, no por seguridad. Medido en
//!   [`docs/research/flatpak-canal-unico.md`](../../../../../docs/research/flatpak-canal-unico.md),
//!   apartado 4.
//!
//! ## El identificador va con la ruta, no con el inodo
//!
//! El permiso que el portal concede se apunta contra la **ruta** del fichero
//! en el anfitrión. Sustituir el fichero por otro en la misma ruta —guardar
//! desde otro programa, que crea un inodo nuevo— conserva el permiso;
//! *mover* el fichero lo deja sin sujeto aunque el inodo siga vivo.
//!
//! Eso no es una curiosidad del portal: **es la implementación de la insignia
//! `No disponible`** de los recientes (ID-38). [`crate::memory::RecentDocument`]
//! identifica la fila por su ruta canónica y decide la insignia preguntando si
//! esa ruta responde ahora mismo, que es exactamente el mismo criterio. Las
//! pruebas de abajo lo dejan por escrito para que las dos mitades no se
//! separen.

use std::path::{Path, PathBuf};

/// Dónde monta el sandbox el portal de documentos. El `1000` del medio es el
/// uid, así que solo se comprueba el prefijo.
const PORTAL_ROOT: &str = "/run/user";

/// El directorio del portal de documentos dentro de ese prefijo.
const PORTAL_DIRECTORY: &str = "doc";

/// Un documento que ha entrado por el portal.
///
/// Fuera del sandbox —los instaladores nativos que todavía no existen— el
/// diálogo devuelve una ruta corriente y este tipo la acepta igual: entonces
/// [`PortalDocument::portal_id`] es `None` y nada más cambia. Lo que **no**
/// cambia en ningún canal es que de aquí no sale un directorio donde escribir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortalDocument {
    handle: PathBuf,
    name: String,
}

impl PortalDocument {
    /// El documento que el diálogo acaba de entregar.
    pub fn opened(handle: impl Into<PathBuf>) -> Self {
        let handle = handle.into();
        let name = handle
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self { handle, name }
    }

    /// El nombre del fichero, que es lo único del original que se reutiliza:
    /// de él sale el nombre del firmado (ADR-0011).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// La ruta **para leer los bytes**, y nada más.
    ///
    /// Se llama así a propósito. No es «la ruta del documento»: es el enlace
    /// que el portal ha concedido para esta sesión, y escribir en su directorio
    /// es el fallo silencioso del apartado 4 de la medición.
    pub fn reading_path(&self) -> &Path {
        &self.handle
    }

    /// El identificador que el portal le ha dado, si ha entrado por él.
    ///
    /// Devuelve una cadena y **no una ruta** por la misma razón que el resto
    /// del tipo: una ruta invitaría a escribir en ella. Vale para casar dos
    /// concesiones del mismo documento dentro de una sesión, no para localizar
    /// nada en el disco del usuario.
    pub fn portal_id(&self) -> Option<&str> {
        let directory = self.handle.parent()?;
        let identifier = directory.file_name()?.to_str()?;
        let root = directory.parent()?;
        if root.file_name()? != PORTAL_DIRECTORY || !root.starts_with(PORTAL_ROOT) {
            return None;
        }
        Some(identifier)
    }

    /// Si ha entrado por el portal del sandbox.
    pub fn came_through_the_portal(&self) -> bool {
        self.portal_id().is_some()
    }
}

/// Dónde se apunta a sí mismo un flatpak: el fichero que el propio `bwrap`
/// deja dentro del sandbox.
const SANDBOX_MARKER: &str = "/.flatpak-info";

/// **La única pregunta al entorno** de todo el destino (ID-184): si
/// Preferencias puede ofrecer «junto al original».
///
/// Se contesta **antes de que exista ningún documento**, que es cuando se
/// pinta la pantalla de ajustes, así que no puede depender de uno. Lo que sí
/// depende del documento —cuál es esa carpeta, o que no la haya— lo contesta
/// el documento mismo, y por eso aquí no hay ningún enum de acceso a ficheros
/// (ID-183).
///
/// Se resuelve como lo resuelven GTK, libportal y Firefox: mirando si existe
/// `/.flatpak-info`. Dentro del sandbox **todo** entra por el portal, así que
/// la carpeta del original no se conoce nunca y ofrecer la opción sería
/// ofrecer algo que no se puede cumplir; fuera, un documento de ruta directa
/// sí tiene carpeta, y uno del portal contesta que no la hay.
pub fn the_original_folder_can_be_offered() -> bool {
    !inside_a_sandbox(Path::new(SANDBOX_MARKER))
}

/// La misma pregunta sobre una marca cualquiera, que es lo que la hace
/// comprobable sin un flatpak montado.
fn inside_a_sandbox(marker: &Path) -> bool {
    marker.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;

    use crate::memory::{Badge, RecentDocument, ShownBadge};

    /// **Grada A**: rutas como cadenas para el portal, y ficheros de verdad en
    /// un directorio temporal para la parte que pregunta al disco. El
    /// comportamiento del portal en sí solo se ve dentro del sandbox y se
    /// comprueba en el sub-issue del flatpak (#62).
    const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/original.pdf";

    #[test]
    fn a_document_from_the_portal_yields_its_name_and_its_identifier() {
        let document = PortalDocument::opened(A_PORTAL_HANDLE);

        assert_eq!(document.name(), "original.pdf");
        assert_eq!(document.portal_id(), Some("1e8b83b9"));
        assert!(document.came_through_the_portal());
    }

    #[test]
    fn a_path_outside_the_portal_has_no_identifier_and_is_still_readable() {
        let document = PortalDocument::opened("/home/quien/Documentos/original.pdf");

        assert_eq!(document.name(), "original.pdf");
        assert_eq!(document.portal_id(), None);
        assert!(!document.came_through_the_portal());
        assert_eq!(
            document.reading_path(),
            Path::new("/home/quien/Documentos/original.pdf")
        );
    }

    /// Un directorio que se llama `doc` en cualquier otro sitio no convierte
    /// su contenido en un documento del portal.
    #[test]
    fn a_folder_named_doc_elsewhere_is_not_the_portal() {
        let document = PortalDocument::opened("/home/quien/doc/1e8b83b9/original.pdf");

        assert_eq!(document.portal_id(), None);
    }

    /// La pregunta al entorno, con la marca del flatpak puesta: dentro del
    /// sandbox no hay carpeta original que ofrecer, porque todo entra por el
    /// portal (ID-184).
    #[test]
    fn inside_the_sandbox_the_original_folder_cannot_be_offered() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let marker = directory.path().join(".flatpak-info");
        fs::write(&marker, b"[Application]\n").expect("deberia escribirse");

        assert!(inside_a_sandbox(&marker));
    }

    /// Y sin la marca, que es el `.deb` y el `.rpm`: la opción se ofrece, y
    /// cada documento contesta luego si tiene carpeta o no.
    #[test]
    fn outside_the_sandbox_the_original_folder_can_be_offered() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        assert!(!inside_a_sandbox(&directory.path().join(".flatpak-info")));
    }

    /// La pregunta de verdad se contesta sobre `/.flatpak-info`, y este equipo
    /// no es un sandbox: lo que se fija aquí es que las dos mitades son la
    /// misma pregunta, no el valor de una de ellas.
    #[test]
    fn the_question_asked_to_the_environment_is_the_marker_of_the_sandbox() {
        assert_eq!(
            the_original_folder_can_be_offered(),
            !inside_a_sandbox(Path::new(SANDBOX_MARKER))
        );
    }

    #[test]
    fn the_document_that_came_in_never_offers_a_folder_to_write_into() {
        let document = PortalDocument::opened(A_PORTAL_HANDLE);

        // Lo que se puede sacar de aqui son dos cadenas y una ruta de lectura.
        // Si algun dia aparece un metodo que devuelva el directorio padre, el
        // `.xdp-…` huerfano vuelve con el.
        assert_eq!(document.name(), "original.pdf");
        assert_eq!(document.portal_id(), Some("1e8b83b9"));
        assert_eq!(document.reading_path(), Path::new(A_PORTAL_HANDLE));
    }

    /// El identificador del portal sigue a la **ruta**: sustituir el fichero
    /// por otro en el mismo sitio no lo pierde. La fila de recientes hace lo
    /// mismo, y esta prueba es la que ata las dos mitades (ID-38).
    #[test]
    fn a_new_file_at_the_same_path_keeps_the_row_available() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let path = directory.path().join("contrato.pdf");
        fs::write(&path, b"%PDF-1.7 primero").expect("deberia escribirse");
        let entry = RecentDocument::seen(&path, Badge::Unsigned, SystemTime::now())
            .expect("deberia anotarse");

        fs::remove_file(&path).expect("deberia borrarse");
        fs::write(&path, b"%PDF-1.7 otro inodo").expect("deberia escribirse");

        assert!(entry.is_available(), "el permiso va con la ruta");
        assert_eq!(entry.shown_badge(), ShownBadge::Unsigned);
    }

    /// Y al revés: mover el fichero deja la ruta sin sujeto aunque el inodo
    /// siga vivo, y eso es lo que enciende `No disponible`.
    #[test]
    fn moving_the_file_away_shows_the_unavailable_badge_though_the_inode_lives_on() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let path = directory.path().join("contrato.pdf");
        fs::write(&path, b"%PDF-1.7 de prueba").expect("deberia escribirse");
        let entry = RecentDocument::seen(&path, Badge::Signed, SystemTime::now())
            .expect("deberia anotarse");

        let elsewhere = directory.path().join("archivado.pdf");
        fs::rename(&path, &elsewhere).expect("deberia moverse");

        assert!(elsewhere.exists(), "el inodo sigue vivo");
        assert!(!entry.is_available());
        assert_eq!(entry.shown_badge(), ShownBadge::Unavailable);
    }
}
