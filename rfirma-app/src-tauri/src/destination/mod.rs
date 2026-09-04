//! **Dónde cae el documento firmado**, y por dónde entra el original
//! (ID-36…ID-39, ADR-0011, #54).
//!
//! El recorrido no se negocia y no lo cambia el empaquetado: se firma y el
//! documento **cae solo**, sin diálogo por firma. Lo que cambia por canal es
//! *dónde* cae. Bajo el sandbox, la carpeta de documentos del usuario, que es
//! lo único que el manifiesto concede
//! (`--filesystem=xdg-documents`, `packaging/flatpak/me.sgomez.rfirma.yml`).
//!
//! Aquí no se escribe ningún fichero: este módulo **decide la ruta** y
//! comprueba que se puede usar. Quien firma y quien copia los bytes es la
//! orquestación trifásica (#60). La separación es la que hace que todo esto sea
//! grada A: son reglas, y las reglas se prueban en el carril rápido.
//!
//! ## Las tres trampas, y las tres fallan calladas
//!
//! 1. **«Junto al original» no es implementable** bajo el sandbox, y no hay
//!    ninguna ruta de código que lo intente. El portal solo da la ruta real a
//!    un llamante `is_host`, que un flatpak nunca es, y escribir al lado del
//!    fichero que entrega el portal deja un `.xdp-…` huérfano **sin dar
//!    error**. La regla no vive en un comentario suelto: vive en el tipo
//!    [`PortalDocument`], que no tiene ningún método que devuelva un
//!    directorio, y en que la única forma de obtener una ruta de salida es
//!    [`CheckedFolder::landing_for`], que cuelga de la carpeta de destino.
//! 2. **`--filesystem=home` no lo arregla.** Está cerrado por **no
//!    funcionar**, no por seguridad: tampoco devolvería la ruta real. Lo
//!    segundo invita a reabrirlo; lo primero, no.
//! 3. **La carpeta de destino no se crea nunca** (ID-38). Escribir en una
//!    carpeta declarada que no existe en el anfitrión contesta **OK**, el
//!    fichero se relee bien y en el anfitrión no hay nada; a la siguiente
//!    ejecución no queda ni rastro. Medido en el #27 y en
//!    [`docs/research/flatpak-canal-unico.md`](../../../../docs/research/flatpak-canal-unico.md),
//!    apartado 4. Por eso [`CheckedFolder::check`] solo mira, y por eso es un
//!    **tipo**: sin pasar por él no se puede nombrar un sitio donde escribir, y
//!    la comprobación cae antes de firmar, como manda el ADR-0010, y no al
//!    guardar.
//!
//! El permiso de directorio del portal se concede **con escritura y persiste**
//! —`~/.local/share/flatpak/db/documents`—, así que la carpeta se elige una vez
//! y luego se reutiliza: lo que se recuerda es
//! [`Configuration::destination`](crate::memory::Configuration::destination), y
//! `None` significa la carpeta de documentos, que resuelve
//! [`crate::paths::documents_folder`]. Nada de esto vuelve a preguntar.
//!
//! Este módulo **no importa [`crate::memory`]** y no debe volver a hacerlo
//! (ID-83): [`DestinationFolder`] es un concepto del destino y vive aquí, y
//! desenvolver la configuración para elegir entre ella y la carpeta de
//! documentos lo hace quien ya tiene la configuración delante,
//! [`crate::app::chosen_folder`]. La memoria sí importa de aquí —persiste una
//! carpeta de destino y un documento del portal—, que es la dirección correcta
//! (ID-81). Lo vigila `tests/module_directions.rs`.
//!
//! El identificador que el portal da a un documento **sigue a la ruta, no al
//! inodo**, que es exactamente la implementación de la insignia
//! `No disponible` de los recientes: está contado en [`portal`].

pub mod error;
pub mod naming;
pub mod portal;

pub use error::{DestinationError, Situation};
pub use naming::{numbered, signed_name, FIRST_NUMBER, MAX_NAMESAKES, SIGNED_SUFFIX};
pub use portal::{the_original_folder_can_be_offered, PortalDocument};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// La carpeta donde cae el documento firmado.
///
/// Bajo el sandbox la aplicación **escribe** en ella pero la única palabra que
/// tiene de ella es su último segmento, así que el ajuste enseña el
/// [`nombre`](DestinationFolder::name) y no la ruta (ADR-0011). Se guarda la
/// ruta entera porque es lo que hace falta para volver a escribir; enseñarla es
/// otra decisión, y es de la interfaz.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinationFolder {
    path: PathBuf,
}

impl DestinationFolder {
    /// La carpeta que hay en `path`.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// La ruta, para escribir en ella.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// El último segmento, que es lo que ve el usuario. Vacío si la ruta no
    /// tiene ninguno —una raíz—, y entonces la interfaz enseña la ruta.
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }
}

/// Una carpeta de destino que **existe de verdad**.
///
/// No se puede construir de otra forma que comprobándola, y no hay ninguna
/// función de este módulo que devuelva una ruta de salida sin tener una de
/// estas delante. Eso es lo que impide que el ID-38 se olvide: no hace falta
/// acordarse de comprobar, hace falta tener el tipo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedFolder {
    path: PathBuf,
}

impl CheckedFolder {
    /// Comprueba la carpeta. **Nunca la crea**, ni siquiera si el error es
    /// justo que no está (ID-38).
    ///
    /// Dentro del sandbox crearla habría contestado OK y no habría dejado nada
    /// en el anfitrión, así que «arreglarlo» aquí sería fabricar el fallo
    /// silencioso en lugar de evitarlo. Si no está, es que no está de verdad:
    /// flatpak solo monta lo que ya existe.
    pub fn check(folder: &DestinationFolder) -> Result<Self, DestinationError> {
        Self::at(folder.path())
    }

    /// La misma comprobación sobre una ruta suelta.
    pub fn at(path: impl AsRef<Path>) -> Result<Self, DestinationError> {
        let path = path.as_ref();
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Ok(Self {
                path: path.to_path_buf(),
            }),
            Ok(_) => Err(DestinationError::about(Situation::NotAFolder, path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(DestinationError::about(Situation::FolderMissing, path))
            }
            Err(error) => Err(DestinationError::caused_by(
                Situation::FolderUnreadable,
                path,
                &error,
            )),
        }
    }

    /// La ruta, para escribir en ella.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// El último segmento, que es **lo único que se le enseña al usuario**
    /// (ADR-0011): bajo el sandbox la aplicación escribe en la carpeta pero no
    /// tiene más palabra de ella que su nombre, y enseñar la ruta donde se
    /// puede y el nombre donde no es la misma incoherencia en pequeño.
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }

    /// Dónde cae `document` una vez firmado, con el conflicto de nombres ya
    /// resuelto.
    ///
    /// La numeración la pone la aplicación porque **sin diálogo por firma no
    /// hay ningún «ya existe, ¿reemplazar?» del sistema que avise**: sin ella,
    /// la segunda firma machacaría a la primera en silencio.
    ///
    /// Del original solo se usa el **nombre**. La carpeta es siempre esta, y
    /// por eso «junto al original» no tiene por dónde colarse.
    pub fn landing_for(&self, document: &PortalDocument) -> Result<PathBuf, DestinationError> {
        let name = signed_name(document.name());
        let first = self.path.join(&name);
        if !first.exists() {
            return Ok(first);
        }
        for number in FIRST_NUMBER..=MAX_NAMESAKES {
            let candidate = self.path.join(numbered(&name, number));
            if !candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(DestinationError::about(Situation::NoFreeName, &first))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// **Grada A**: directorios de verdad en un temporal, que es lo que hace
    /// falta para que «existe» y «no existe» signifiquen algo. Lo que el
    /// portal hace *dentro* del sandbox —contestar OK a una escritura que no
    /// deja nada— no se puede reproducir fuera de él: se comprueba en el
    /// sub-issue del flatpak (#62) y aquí queda escrito en
    /// [`the_folder_is_never_created_here`], con el enlace a la medición.
    fn a_folder() -> tempfile::TempDir {
        tempfile::tempdir().expect("deberia haber directorio temporal")
    }

    fn a_document() -> PortalDocument {
        PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf")
    }

    #[test]
    fn a_folder_that_is_there_can_be_checked_and_shows_its_name() {
        let root = a_folder();
        let path = root.path().join("Documentos");
        fs::create_dir(&path).expect("deberia crearse");

        let checked = CheckedFolder::at(&path).expect("deberia comprobarse");

        assert_eq!(checked.path(), path);
        assert_eq!(checked.name(), "Documentos");
    }

    /// La trampa del ID-38, por escrito.
    ///
    /// Dentro del sandbox, escribir en una carpeta **declarada** en
    /// `--filesystem` que no existe en el anfitrión contesta OK, el fichero se
    /// relee bien y en el anfitrión no hay nada; a la siguiente ejecución no
    /// queda ni rastro. Medido en el #27 y recogido en
    /// `docs/research/flatpak-canal-unico.md`, apartado 4. Fuera del sandbox
    /// eso no se puede reproducir, así que lo que esta prueba sujeta es la
    /// **única** defensa que tenemos contra ello: que la aplicación no crea la
    /// carpeta ni cuando el error es justo que falta.
    #[test]
    fn the_folder_is_never_created_here() {
        let root = a_folder();
        let missing = root.path().join("Documentos");

        let failure = CheckedFolder::at(&missing).expect_err("no deberia comprobarse");

        assert_eq!(failure.situation(), Situation::FolderMissing);
        assert!(failure.detail().contains("Documentos"));
        assert!(
            !missing.exists(),
            "comprobar una carpeta que falta no puede crearla: dentro del sandbox \
             eso contestaria OK y no dejaria nada en el anfitrion (#27)"
        );
    }

    #[test]
    fn a_file_where_the_folder_should_be_is_not_a_destination() {
        let root = a_folder();
        let path = root.path().join("Documentos");
        fs::write(&path, b"no soy una carpeta").expect("deberia escribirse");

        let failure = CheckedFolder::at(&path).expect_err("no deberia comprobarse");

        assert_eq!(failure.situation(), Situation::NotAFolder);
    }

    #[test]
    fn the_signed_document_lands_in_the_destination_folder_with_no_dialogue() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");

        let landing = checked
            .landing_for(&a_document())
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado.pdf"));
    }

    /// El original vive en `/run/user/1000/doc/…`, y lo firmado **no** cae ahí:
    /// escribir un hermano del fichero del portal deja un `.xdp-…` huérfano sin
    /// dar error (ID-37).
    #[test]
    fn the_landing_never_falls_next_to_the_original() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        let document = a_document();

        let landing = checked.landing_for(&document).expect("deberia haber sitio");

        assert_eq!(landing.parent(), Some(checked.path()));
        assert_ne!(landing.parent(), document.reading_path().parent());
    }

    #[test]
    fn a_second_signature_is_numbered_instead_of_overwriting_the_first() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        fs::write(root.path().join("contrato-firmado.pdf"), b"la primera")
            .expect("deberia escribirse");

        let landing = checked
            .landing_for(&a_document())
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado-2.pdf"));
        assert_eq!(
            fs::read(root.path().join("contrato-firmado.pdf")).expect("deberia leerse"),
            b"la primera",
            "la primera firma sigue donde estaba"
        );
    }

    #[test]
    fn cosigning_the_signed_document_does_not_stack_a_second_suffix() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        let already_signed = PortalDocument::opened("/run/user/1000/doc/aa/contrato-firmado.pdf");

        let landing = checked
            .landing_for(&already_signed)
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado.pdf"));
    }

    /// La tercera vuelta, que es donde el sufijo se apilaba: lo que entra ya
    /// lleva el número de la segunda, y lo que sale es el siguiente número, no
    /// un segundo `-firmado`.
    #[test]
    fn the_third_cosignature_keeps_counting_instead_of_stacking() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");
        fs::write(root.path().join("contrato-firmado.pdf"), b"la primera")
            .expect("deberia escribirse");
        fs::write(root.path().join("contrato-firmado-2.pdf"), b"la segunda")
            .expect("deberia escribirse");
        let signed_twice = PortalDocument::opened("/run/user/1000/doc/aa/contrato-firmado-2.pdf");

        let landing = checked
            .landing_for(&signed_twice)
            .expect("deberia haber sitio");

        assert_eq!(landing, root.path().join("contrato-firmado-3.pdf"));
    }

    #[test]
    fn deciding_where_it_lands_writes_nothing() {
        let root = a_folder();
        let checked = CheckedFolder::at(root.path()).expect("deberia comprobarse");

        let landing = checked
            .landing_for(&a_document())
            .expect("deberia haber sitio");

        assert!(
            !landing.exists(),
            "quien escribe es la orquestacion, no esto"
        );
    }

    #[test]
    fn the_destination_shows_its_name_and_not_its_path() {
        let folder = DestinationFolder::at("/home/quien/Documentos/Firmados");

        assert_eq!(folder.name(), "Firmados");
    }
}
