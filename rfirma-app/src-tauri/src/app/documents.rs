//! **Por dónde entra el documento y dónde cae el firmado.**
//!
//! Entra de dos maneras —elegido en el diálogo o soltado en la ventana— y las
//! dos acaban igual: el documento queda apuntado en
//! [`OpenedDocuments`](crate::memory::OpenedDocuments) y lo que cruza es su
//! identificador opaco. Sale de una sola: la carpeta de destino, sin diálogo
//! (ID-36, ADR-0011).

use std::path::{Path, PathBuf};

use crate::commands::views::{
    DestinationView, DroppedDocumentView, Failure, OpenedDocumentView, SignedDocumentView,
};
use crate::destination::{CheckedFolder, DestinationFolder, PortalDocument};
use crate::memory::{Configuration, Memory, OpenedDocuments};
use crate::signing::Refusal;

/// **Caso de uso.** Apunta el documento que el diálogo acaba de conceder y lo
/// cuenta como la ventana lo entiende.
///
/// De paso apunta de dónde salió, cuando se puede saber: ver
/// [`remember_the_folder`].
pub fn note_opened(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    handle: PathBuf,
) -> OpenedDocumentView {
    let document = PortalDocument::opened(handle);
    remember_the_folder(memory, configuration, &document);
    told_as_opened(document, opened)
}

/// **Caso de uso.** Los bytes del documento abierto, contra su identificador.
pub fn bytes_of(opened: &OpenedDocuments, id: &str) -> Result<Vec<u8>, Failure> {
    let document = opened_document(opened, id)?;
    std::fs::read(document.reading_path())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))
}

/// **Caso de uso.** Decide qué hacer con lo que se ha soltado y lo apunta si se
/// puede abrir.
///
/// Es el adaptador entre [`crate::dropped::first_pdf`], que es quien decide, y
/// lo que la ventana entiende. Devuelve `None` cuando no se ha soltado nada:
/// entonces no hay nada que contar y no se emite ningún evento.
pub fn dropped_document(
    paths: &[PathBuf],
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    match crate::dropped::first_pdf(paths) {
        crate::dropped::Dropped::Nothing => None,
        crate::dropped::Dropped::Opened { path, ignored } => Some(DroppedDocumentView {
            document: Some(told_as_opened(PortalDocument::opened(path), opened)),
            failure: None,
            ignored,
        }),
        crate::dropped::Dropped::NotAPdf { ignored } => Some(DroppedDocumentView {
            document: None,
            failure: Some(Failure::from(Refusal::NotAPdf)),
            ignored,
        }),
        // El aviso que el ID-68 exige: no es «ha fallado» a secas, es una
        // situación propia cuyo texto dice qué hacer —usar el botón de abrir,
        // que sí pasa por el portal—. Por qué existe este caso y desde qué
        // carpetas ocurre está medido en
        // `docs/research/arrastre-bajo-el-sandbox.md`.
        crate::dropped::Dropped::Unreadable { detail, ignored } => Some(DroppedDocumentView {
            document: None,
            failure: Some(Failure::new("droppedFileUnreadable", detail)),
            ignored,
        }),
    }
}

/// **Caso de uso.** Deja caer el documento firmado en la carpeta de destino,
/// **sin diálogo** (ID-36, ADR-0011).
///
/// Lo único que se elige es la carpeta, y se eligió una vez. El nombre lo
/// resuelve [`CheckedFolder::landing_for`], que numera los homónimos: sin
/// diálogo por firma no hay ningún «ya existe, ¿reemplazar?» que avise, así que
/// sin esa numeración la segunda firma machacaría a la primera en silencio.
pub fn deliver(
    configuration: &Configuration,
    documents_folder: &Path,
    document: &PortalDocument,
    signed: &[u8],
) -> Result<(PathBuf, SignedDocumentView), Failure> {
    let chosen = super::chosen_folder(configuration, documents_folder.to_path_buf());
    // La carpeta se comprueba y **no se crea nunca** (ID-38): bajo el sandbox
    // crearla contesta OK y no deja nada en el anfitrión.
    let folder = CheckedFolder::check(&chosen)?;
    let landing = folder.landing_for(document)?;
    std::fs::write(&landing, signed)
        .map_err(|error| Failure::new("folderUnwritable", error.to_string()))?;
    // La ruta sale **hacia dentro**, no hacia la ventana: la necesita
    // [`crate::app::recents::note_signed`] para anotar la fila del firmado, y
    // lo que cruza sigue siendo [`SignedDocumentView`], dos nombres y ninguna
    // ruta (ADR-0011).
    // El tamaño sale de aquí y no de volver a mirar el fichero (ID-77): estos
    // son los bytes que se acaban de escribir, y son los que el resumen cuenta.
    let told = told_as(&landing, &folder, signed.len() as u64);
    Ok((landing, told))
}

/// **Caso de uso.** Dónde va a caer el documento que hay delante, **antes** de
/// firmarlo y sin escribir nada (ID-63, ID-67).
///
/// Es [`deliver`] menos la escritura: la misma carpeta elegida, la misma
/// comprobación —[`CheckedFolder::check`], que **no crea nada** (ID-38)— y el
/// mismo nombre que compone [`CheckedFolder::landing_for`], homónimos
/// numerados incluidos. Lo que el pie del panel enseña es lo que va a ocurrir,
/// no una promesa parecida.
///
/// La carpeta que no está o no se deja escribir **no es un fallo aquí**: es un
/// destino que se cuenta como no escribible, con el botón de firmar todavía
/// vivo y un `Cambiar` al lado (ADR-0011). Por eso no devuelve `Result`.
pub fn where_it_lands(
    configuration: &Configuration,
    documents_folder: &Path,
    document: &PortalDocument,
) -> DestinationView {
    let chosen = super::chosen_folder(configuration, documents_folder.to_path_buf());
    let Ok(folder) = CheckedFolder::check(&chosen) else {
        return DestinationView {
            folder: chosen.name().to_owned(),
            name: None,
            writable: false,
        };
    };
    let name = folder
        .landing_for(document)
        .ok()
        .and_then(|landing| file_name_of(&landing));
    DestinationView {
        folder: folder.name().to_owned(),
        name,
        writable: true,
    }
}

/// Cómo se cuenta un documento firmado: **dos nombres, un tamaño y ninguna
/// ruta** (ADR-0011).
///
/// El tamaño entra por parámetro y no se lee del disco: quien llama acaba de
/// escribir esos bytes y ya sabe cuántos son (ID-77).
pub fn told_as(landing: &Path, folder: &CheckedFolder, size_bytes: u64) -> SignedDocumentView {
    SignedDocumentView {
        name: file_name_of(landing).unwrap_or_default(),
        folder: folder.name().to_owned(),
        size_bytes,
    }
}

/// El último segmento de una ruta, que es lo único de ella que cruza.
fn file_name_of(landing: &Path) -> Option<String> {
    landing
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

/// Apunta el documento en el registro de la sesión y lo cuenta con su
/// identificador opaco.
fn told_as_opened(document: PortalDocument, opened: &OpenedDocuments) -> OpenedDocumentView {
    let name = document.name().to_owned();
    let modified = modified_seconds(&document);
    let path = real_path_of(&document).map(|path| path.to_string_lossy().into_owned());
    OpenedDocumentView {
        id: opened.remember(document),
        name,
        modified,
        path,
    }
}

/// Dónde se abre el diálogo de abrir: **la última carpeta usada**, y si no se
/// sabe, la de destino.
///
/// Las dos mitades de la frase importan, porque no todos los canales saben lo
/// mismo:
///
/// - **Fuera del sandbox** —deb, rpm, Windows, macOS— el diálogo devuelve una
///   ruta de verdad, así que la carpeta de la que salió el documento se sabe y
///   se recuerda. Lo hace [`remembered_folder`], y vive en el estado, no en la
///   configuración: la acumula la aplicación sola.
/// - **Bajo el sandbox no se puede saber**, y no hay forma de arreglarlo con
///   más código: lo que el portal devuelve es
///   `/run/user/1000/doc/<id>/nombre.pdf`, cuyo directorio padre tiene un solo
///   fichero dentro y no es ninguna carpeta del usuario; preguntar por la real
///   —`org.freedesktop.portal.Documents.Info` y `.Lookup`— contesta
///   `Not allowed in sandbox`, y `--filesystem=home` tampoco la devolvería.
///   Medido en `docs/research/flatpak-canal-unico.md`, apartado 4.
///
/// El respaldo para ese caso es la **carpeta de destino**, la de Preferencias:
/// la única carpeta del usuario que la aplicación conoce y nombra en el
/// flatpak. Resuelve lo que se quería de verdad —no empezar cada vez en la
/// lista de «Recientes» del sistema— y además deja a la vista lo ya firmado.
///
/// Esto **no es una ruta donde escribir** y no puede llegar a serlo: lo único
/// que la recibe es `set_directory`, y la única forma de nombrar un sitio
/// donde cae un fichero sigue siendo [`CheckedFolder::landing_for`]
/// (ADR-0011).
///
/// Devuelve `None` si no queda ninguna de las dos, y entonces el diálogo se
/// abre donde el sistema quiera: [`CheckedFolder`] solo mira, **nunca crea**
/// (ID-38).
pub fn starting_folder(
    memory: &Memory,
    configuration: &Configuration,
    documents_folder: &Path,
) -> Option<PathBuf> {
    if let Some(remembered) = remembered_folder(memory) {
        return Some(remembered);
    }
    let folder = super::chosen_folder(configuration, documents_folder.to_path_buf());
    CheckedFolder::check(&folder)
        .ok()
        .map(|checked| checked.path().to_path_buf())
}

/// La última carpeta apuntada, **si sigue estando ahí**.
///
/// Se comprueba porque una carpeta que se borró o que estaba en un disco que
/// ya no está montado no es un punto de partida: es un diálogo que se abre en
/// un sitio que no existe. Bajo el sandbox esto es siempre `None`, y también
/// con «Recordar mi actividad» apagado, porque entonces no hay fichero de
/// estado que leer.
pub fn remembered_folder(memory: &Memory) -> Option<PathBuf> {
    memory
        .state()
        .ok()?
        .into_value()
        .last_open_folder
        .filter(|folder| folder.is_dir())
}

/// Apunta de dónde salió el documento, **cuando se puede saber**.
///
/// Es lo mejor posible en cada canal y a propósito: donde el diálogo devuelve
/// una ruta de verdad, la próxima vez se abre justo ahí; donde devuelve un
/// enlace del portal, [`folder_it_came_from`] contesta `None` y no se apunta
/// nada. Un fallo al escribir el estado **no impide abrir el documento**:
/// recordar la carpeta es una comodidad, y perderla no puede costar el
/// recorrido.
pub fn remember_the_folder(
    memory: &Memory,
    configuration: &Configuration,
    document: &PortalDocument,
) {
    let Some(folder) = folder_it_came_from(document) else {
        return;
    };
    let Ok(loaded) = memory.state() else {
        return;
    };
    let mut state = loaded.into_value();
    if state.last_open_folder.as_deref() == Some(folder) {
        return;
    }
    state.last_open_folder = Some(folder.to_path_buf());
    let _ = memory.remember_state(configuration, &state);
}

/// La carpeta de la que salió el documento, o `None` si entró por el portal.
///
/// El `None` **no es una precaución, es la verdad**: el directorio padre de un
/// enlace del portal contiene ese solo fichero y no dice nada de dónde está el
/// original. Apuntarlo abriría el diálogo la próxima vez en un directorio del
/// sandbox que para entonces ni existe.
///
/// Vive aquí y no en [`PortalDocument`] para no darle a ese tipo un método que
/// devuelva un directorio: el sitio donde cae lo firmado lo decide
/// [`CheckedFolder`] y nadie más, y esta carpeta es un dato sobre el original,
/// no un destino (ADR-0011).
pub fn folder_it_came_from(document: &PortalDocument) -> Option<&Path> {
    if document.came_through_the_portal() {
        return None;
    }
    document.reading_path().parent()
}

/// **Caso de uso.** Qué significa «junto al original» para **este** documento,
/// o `None` si no significa nada (ID-183).
///
/// La capacidad es del documento y no del entorno, así que aquí no se
/// pregunta por el canal ni hay ningún enum que lo clasifique: un documento
/// sin identificador de portal *es* un documento de ruta directa, y «junto al
/// original» es la carpeta en la que está. Uno que entró por el portal
/// contesta que no hay carpeta original, y eso vale igual en un `.deb` —que
/// también puede recibir una ruta del portal— que dentro del flatpak.
///
/// Si Preferencias llega a ofrecer la opción es otra pregunta, la única que se
/// le hace al entorno: [`crate::destination::the_original_folder_can_be_offered`].
pub fn next_to_the_original(document: &PortalDocument) -> Option<DestinationFolder> {
    folder_it_came_from(document).map(DestinationFolder::at)
}

/// **Caso de uso.** La ruta real del documento, cuando se conoce (ID-185).
///
/// Fuera del sandbox se enseña, como hace cualquier aplicación de escritorio:
/// el argumento de privacidad no se sostiene —el gestor de ficheros la enseña
/// todo el día— y lo que sí se sostiene es la corrección. Bajo el portal la
/// ruta **no se conoce**, así que lo que sale es `None` y no el enlace de
/// `/run/user/…`, que sería devolver una mentira.
pub fn real_path_of(document: &PortalDocument) -> Option<&Path> {
    if document.came_through_the_portal() {
        return None;
    }
    Some(document.reading_path())
}

/// El documento que se abrió con ese identificador.
///
/// Que no esté apuntado no es un fallo del programa: se cuenta como un
/// documento que no se puede leer, que es lo que la ventana sabe enseñar.
pub fn opened_document(opened: &OpenedDocuments, id: &str) -> Result<PortalDocument, Failure> {
    opened.get(id).ok_or_else(|| {
        Failure::new(
            "documentUnreadable",
            "el documento ya no esta abierto en esta sesion",
        )
    })
}

/// El `mtime` del documento, en segundos desde la época.
fn modified_seconds(document: &PortalDocument) -> Option<u64> {
    std::fs::metadata(document.reading_path())
        .and_then(|metadata| metadata.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{
        bytes_of, deliver, dropped_document, folder_it_came_from, next_to_the_original,
        note_opened, real_path_of, remember_the_folder, starting_folder, told_as, where_it_lands,
    };
    use crate::app::fixtures::a_memory;
    use crate::destination::{CheckedFolder, PortalDocument};
    use crate::memory::{Configuration, OpenedDocuments};

    /// Una configuración con esa carpeta de destino elegida.
    fn with_destination(folder: &std::path::Path) -> Configuration {
        Configuration {
            destination: Some(crate::destination::DestinationFolder::at(folder)),
            ..Configuration::default()
        }
    }

    /// Lo que el diálogo concede acaba apuntado y contado como la ventana lo
    /// entiende: un identificador, un nombre y un `mtime`. Los bytes se piden
    /// después contra ese identificador, nunca contra una ruta (ID-60, ID-66).
    #[test]
    fn what_the_dialog_granted_is_noted_and_read_back_by_its_identifier() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let pdf = home.path().join("contrato.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n").expect("deberia escribirse el temporal");
        let opened = OpenedDocuments::new();

        let view = note_opened(
            &a_memory(home.path()),
            &Configuration::default(),
            &opened,
            pdf,
        );

        assert_eq!(view.name, "contrato.pdf");
        assert_eq!(view.id.len(), 32);
        assert!(view.modified.is_some(), "el mtime lo lee el backend");
        assert_eq!(
            bytes_of(&opened, &view.id).expect("se leen contra el identificador"),
            b"%PDF-1.4\n"
        );
    }

    /// Un identificador que no es de esta sesión no es un fallo del programa:
    /// se cuenta como un documento que no se puede leer, que es lo que la
    /// ventana sabe enseñar.
    #[test]
    fn a_document_that_is_not_open_in_this_session_cannot_be_read() {
        let failure = bytes_of(&OpenedDocuments::new(), "0").expect_err("no esta abierto");

        assert_eq!(failure.situation, "documentUnreadable");
    }

    /// El identificador cruza; la ruta que hay detrás se queda en el registro.
    #[test]
    fn the_identifier_crosses_and_the_reading_path_stays_behind() {
        let opened = OpenedDocuments::new();
        let handle = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

        let id = opened.remember(PortalDocument::opened(handle));

        assert!(
            !id.contains("1e8b83b9"),
            "el identificador no lleva el del portal: {id}"
        );
        assert!(!id.contains("contrato"), "ni el nombre: {id}");
        assert_eq!(
            opened
                .get(&id)
                .map(|document| document.reading_path().to_owned()),
            Some(std::path::PathBuf::from(handle)),
            "y el backend sí sabe por dónde leerlo"
        );
    }

    /// Soltar un PDF legible acaba igual que elegirlo por el diálogo: un
    /// documento apuntado, con su identificador opaco y su nombre.
    #[test]
    fn a_dropped_pdf_crosses_as_an_opened_document() {
        let opened = OpenedDocuments::new();
        let pdf = std::env::temp_dir().join("rfirma-commands-soltado.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n").expect("se puede escribir en el temporal");

        let view = dropped_document(&[pdf], &opened).expect("algo se ha soltado");

        let document = view.document.expect("y se ha abierto");
        assert_eq!(document.name, "rfirma-commands-soltado.pdf");
        assert_eq!(document.id.len(), 32);
        assert_eq!(view.failure, None);
        assert_eq!(view.ignored, 0);
        assert_eq!(opened.len(), 1);
    }

    /// Y lo que no es un PDF no apunta nada: se cuenta con la misma situación
    /// con la que se rechaza al firmar, que ya está en el catálogo.
    #[test]
    fn dropping_something_that_is_not_a_pdf_opens_nothing_and_says_so() {
        let opened = OpenedDocuments::new();
        let other = std::env::temp_dir().join("rfirma-commands-soltado.ods");

        let view = dropped_document(&[other], &opened).expect("algo se ha soltado");

        assert!(view.document.is_none());
        assert_eq!(
            view.failure.map(|failure| failure.situation),
            Some("notAPdf".to_owned())
        );
        assert!(opened.is_empty(), "no se apunta lo que no se abre");
    }

    /// El aviso del ID-68 tiene **situación propia**: `documentUnreadable`
    /// dice «comprueba que sigue donde estaba», y aquí el fichero está donde
    /// estaba —lo que falta es la concesión—, así que ese texto mandaría a
    /// mirar lo que no es. El suyo dice qué hacer: usar el botón de abrir.
    #[test]
    fn a_dropped_file_the_sandbox_cannot_read_names_its_own_situation() {
        let opened = OpenedDocuments::new();
        let unreachable = std::env::temp_dir().join("rfirma-commands-no-existe/contrato.pdf");

        let view = dropped_document(&[unreachable], &opened).expect("algo se ha soltado");

        let failure = view.failure.expect("se cuenta como un fallo con nombre");
        assert_eq!(failure.situation, "droppedFileUnreadable");
        assert!(!failure.detail.is_empty(), "con su detalle crudo (ID-29)");
    }

    /// Soltar nada no es un suceso que contar, así que no se emite nada.
    #[test]
    fn dropping_no_files_at_all_says_nothing() {
        assert_eq!(dropped_document(&[], &OpenedDocuments::new()), None);
    }

    #[test]
    fn a_signed_document_is_named_by_its_file_and_its_folder_and_nothing_else() {
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let checked = CheckedFolder::at(folder.path()).expect("existe");
        let landing = folder.path().join("contrato-firmado.pdf");

        let view = told_as(&landing, &checked, 2_400_000);

        assert_eq!(view.name, "contrato-firmado.pdf");
        assert_eq!(view.size_bytes, 2_400_000);
        assert_eq!(
            view.folder,
            folder.path().file_name().and_then(|n| n.to_str()).unwrap()
        );
        // Ni el nombre ni la carpeta llevan un separador: si lo llevaran, sería
        // una ruta del anfitrión saliendo por la orden (ADR-0011).
        assert!(!view.name.contains('/'));
        assert!(!view.folder.contains('/'));
    }

    /// El diálogo de abrir arranca donde caen los firmados, que es la única
    /// carpeta que la aplicación conoce bajo el sandbox.
    #[test]
    fn the_open_dialog_starts_in_the_destination_folder() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let chosen = documents.path().join("Firmados");
        std::fs::create_dir(&chosen).expect("deberia crearse la carpeta de prueba");

        assert_eq!(
            starting_folder(
                &a_memory(documents.path()),
                &with_destination(&chosen),
                documents.path()
            ),
            Some(chosen)
        );
    }

    /// Sin destino elegido manda la carpeta de documentos, igual que al
    /// guardar: las dos puntas del recorrido miran al mismo sitio.
    #[test]
    fn without_a_chosen_destination_it_starts_in_the_documents_folder() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");

        assert_eq!(
            starting_folder(
                &a_memory(documents.path()),
                &Configuration::default(),
                documents.path()
            ),
            Some(documents.path().to_path_buf())
        );
    }

    /// La carpeta **no se crea nunca** (ID-38): si no está, el diálogo se abre
    /// donde el sistema quiera y ya está. Fabricarla aquí sería justo el fallo
    /// silencioso que midió el #27.
    #[test]
    fn a_missing_folder_neither_gets_created_nor_stops_the_dialog() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let absent = documents.path().join("Firmados");

        assert_eq!(
            starting_folder(
                &a_memory(documents.path()),
                &with_destination(&absent),
                documents.path()
            ),
            None
        );
        assert!(!absent.exists(), "la carpeta no se puede haber creado");
    }

    /// Fuera del sandbox el diálogo devuelve una ruta de verdad, y entonces la
    /// carpeta de la que salió el documento **sí** se sabe.
    #[test]
    fn outside_the_sandbox_the_folder_the_document_came_from_is_the_real_one() {
        let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

        assert_eq!(
            folder_it_came_from(&document),
            Some(std::path::Path::new("/home/quien/Contratos"))
        );
    }

    /// Y bajo el sandbox no se sabe. El padre del enlace del portal tiene ese
    /// solo fichero dentro y no es ninguna carpeta del usuario: apuntarlo
    /// abriría el diálogo la próxima vez en un directorio que ni existe ya.
    #[test]
    fn a_document_from_the_portal_leaves_no_folder_to_remember() {
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

        assert_eq!(folder_it_came_from(&document), None);
    }

    /// «Junto al original» es la carpeta **de ese documento**, y la contesta el
    /// documento: no hay ningún enum de acceso a ficheros que clasifique el
    /// entorno primero (ID-183).
    #[test]
    fn a_document_with_a_direct_path_offers_the_folder_it_is_in() {
        let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

        let folder = next_to_the_original(&document).expect("hay carpeta original");

        assert_eq!(folder.path(), std::path::Path::new("/home/quien/Contratos"));
        assert_eq!(folder.name(), "Contratos");
    }

    /// Y uno que entró por el portal contesta que **no hay carpeta original**.
    /// Vale igual dentro del flatpak que en el `.deb`, que también puede
    /// recibir una ruta del portal: quien lo decide es el documento, no el
    /// canal (ID-183).
    #[test]
    fn a_document_from_the_portal_has_no_original_folder_to_offer() {
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

        assert_eq!(next_to_the_original(&document), None);
    }

    /// Fuera del sandbox se enseña la ruta real, como cualquier aplicación de
    /// escritorio (ID-185).
    #[test]
    fn outside_the_sandbox_the_real_path_of_the_document_is_told() {
        let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

        assert_eq!(
            real_path_of(&document),
            Some(std::path::Path::new("/home/quien/Contratos/contrato.pdf"))
        );
    }

    /// Y bajo el portal **no se enseña ninguna**: el enlace de `/run/user/…` no
    /// es la ruta del documento, así que devolverlo sería devolver una mentira
    /// (ID-185, ADR-0011).
    #[test]
    fn the_portal_handle_is_never_told_as_the_real_path() {
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

        assert_eq!(real_path_of(&document), None);
    }

    /// Y eso es lo que la ventana recibe: el documento del portal cruza sin
    /// ruta, y el de ruta directa con la suya.
    #[test]
    fn the_opened_document_crosses_with_the_real_path_only_when_there_is_one() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let pdf = home.path().join("contrato.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n").expect("deberia escribirse el temporal");
        let memory = a_memory(home.path());
        let opened = OpenedDocuments::new();

        let direct = note_opened(&memory, &Configuration::default(), &opened, pdf.clone());
        let through_the_portal = note_opened(
            &memory,
            &Configuration::default(),
            &opened,
            std::path::PathBuf::from("/run/user/1000/doc/1e8b83b9/contrato.pdf"),
        );

        assert_eq!(direct.path.as_deref(), pdf.to_str());
        assert_eq!(through_the_portal.path, None);
    }

    /// Lo pedido: la próxima vez el diálogo se abre donde estuvo la última vez,
    /// y no en el destino.
    #[test]
    fn the_last_folder_used_wins_over_the_destination_folder() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let contracts = documents.path().join("Contratos");
        std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
        let memory = a_memory(documents.path());
        remember_the_folder(
            &memory,
            &Configuration::default(),
            &PortalDocument::opened(contracts.join("contrato.pdf")),
        );

        assert_eq!(
            starting_folder(&memory, &Configuration::default(), documents.path()),
            Some(contracts)
        );
    }

    /// Una carpeta que ya no está no es un punto de partida: es un diálogo que
    /// se abre en un sitio que no existe.
    #[test]
    fn a_remembered_folder_that_is_gone_falls_back_to_the_destination() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let contracts = documents.path().join("Contratos");
        std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
        let memory = a_memory(documents.path());
        remember_the_folder(
            &memory,
            &Configuration::default(),
            &PortalDocument::opened(contracts.join("contrato.pdf")),
        );
        std::fs::remove_dir(&contracts).expect("deberia borrarse");

        assert_eq!(
            starting_folder(&memory, &Configuration::default(), documents.path()),
            Some(documents.path().to_path_buf())
        );
    }

    /// Bajo el sandbox no se apunta nada, así que el diálogo sigue abriéndose
    /// en el destino por los siglos de los siglos. Es lo correcto: la
    /// alternativa es guardar un directorio del portal.
    #[test]
    fn opening_through_the_portal_never_writes_a_folder_into_the_state() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(documents.path());

        remember_the_folder(
            &memory,
            &Configuration::default(),
            &PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf"),
        );

        assert_eq!(
            memory
                .state()
                .expect("deberia leerse el estado")
                .value()
                .last_open_folder,
            None
        );
        assert_eq!(
            starting_folder(&memory, &Configuration::default(), documents.path()),
            Some(documents.path().to_path_buf())
        );
    }

    /// La carpeta es actividad, y «Recordar mi actividad» manda: con el
    /// interruptor apagado no se apunta, y el diálogo vuelve al destino.
    #[test]
    fn the_folder_is_not_remembered_with_the_activity_switch_off() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let contracts = documents.path().join("Contratos");
        std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
        let memory = a_memory(documents.path());
        let switched_off = Configuration {
            remember_activity: false,
            ..Configuration::default()
        };

        remember_the_folder(
            &memory,
            &switched_off,
            &PortalDocument::opened(contracts.join("contrato.pdf")),
        );

        assert_eq!(
            starting_folder(&memory, &switched_off, documents.path()),
            Some(documents.path().to_path_buf())
        );
    }

    #[test]
    fn the_signed_document_falls_into_the_destination_folder_without_a_dialog() {
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let view = deliver(
            &Configuration::default(),
            folder.path(),
            &document,
            b"%PDF-firmado",
        )
        .expect("cae");

        assert_eq!(view.1.name, "contrato-firmado.pdf");
        // El tamaño que cruza es el de los bytes escritos, contados en la
        // escritura y no releídos del disco (ID-77).
        assert_eq!(view.1.size_bytes, b"%PDF-firmado".len() as u64);
        assert_eq!(
            std::fs::read(folder.path().join("contrato-firmado.pdf")).expect("esta"),
            b"%PDF-firmado"
        );
    }

    #[test]
    fn a_second_signature_is_numbered_instead_of_overwriting_the_first() {
        // Sin diálogo por firma no hay ningún «ya existe, ¿reemplazar?» que
        // avise: sin la numeración, la segunda machacaría a la primera callando.
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        deliver(
            &Configuration::default(),
            folder.path(),
            &document,
            b"la primera",
        )
        .expect("cae");
        let second = deliver(
            &Configuration::default(),
            folder.path(),
            &document,
            b"la segunda",
        )
        .expect("cae tambien");

        assert_ne!(second.1.name, "contrato-firmado.pdf");
        assert_eq!(
            std::fs::read(folder.path().join("contrato-firmado.pdf")).expect("sigue"),
            b"la primera"
        );
    }

    #[test]
    fn a_destination_folder_that_is_not_there_is_told_and_never_created() {
        // Bajo el sandbox crearla contesta OK y no deja nada en el anfitrión
        // (ID-38): la única respuesta correcta es decirlo.
        let missing = tempfile::tempdir()
            .expect("temporal")
            .path()
            .join("no-esta");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let failure =
            deliver(&Configuration::default(), &missing, &document, b"x").expect_err("no esta");

        assert_eq!(failure.situation, "folderMissing");
        assert!(!missing.exists(), "la carpeta se ha creado, y no debía");
    }

    /// El pie del panel enseña **carpeta y nombre**, y el nombre es el que va a
    /// caer de verdad: el sufijo `-firmado` ya puesto (ID-63).
    #[test]
    fn the_landing_is_told_by_its_folder_and_its_name_before_signing() {
        let folder = tempfile::tempdir().expect("deberia haber directorio temporal");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let view = where_it_lands(
            &with_destination(folder.path()),
            std::path::Path::new("/no/se/usa"),
            &document,
        );

        assert!(view.writable, "la carpeta esta y se puede escribir");
        assert_eq!(view.name.as_deref(), Some("contrato-firmado.pdf"));
        assert_eq!(
            view.folder,
            folder
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .expect("el temporal tiene nombre")
        );
    }

    /// El homónimo se resuelve **antes** de firmar: quien mira el pie está
    /// preguntándose si va a machacar el anterior, y la respuesta es el número.
    #[test]
    fn a_namesake_already_there_is_numbered_in_what_the_footer_shows() {
        let folder = tempfile::tempdir().expect("deberia haber directorio temporal");
        std::fs::write(folder.path().join("contrato-firmado.pdf"), b"x")
            .expect("deberia escribirse el homonimo");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let view = where_it_lands(
            &with_destination(folder.path()),
            std::path::Path::new("/no/se/usa"),
            &document,
        );

        assert_eq!(view.name.as_deref(), Some("contrato-firmado-2.pdf"));
    }

    /// `writable` sale de `CheckedFolder::check` y no de un literal (ID-67): con
    /// la carpeta ausente el pie avisa, **y la carpeta no se crea**.
    #[test]
    fn a_folder_that_is_not_there_is_told_as_unwritable_and_stays_uncreated() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let missing = home.path().join("Firmados");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let view = where_it_lands(
            &with_destination(&missing),
            std::path::Path::new("/no/se/usa"),
            &document,
        );

        assert!(!view.writable);
        assert_eq!(view.folder, "Firmados", "la carpeta se sigue nombrando");
        assert_eq!(view.name, None, "sin carpeta no hay nombre que prometer");
        assert!(!missing.exists(), "la carpeta se ha creado, y no debía");
    }

    /// Decidir dónde caerá **no escribe nada**: el pie se pinta en cada pintada
    /// y una que dejara ficheros llenaría la carpeta de vacíos.
    #[test]
    fn telling_the_landing_writes_nothing() {
        let folder = tempfile::tempdir().expect("deberia haber directorio temporal");
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let view = where_it_lands(
            &with_destination(folder.path()),
            std::path::Path::new("/no/se/usa"),
            &document,
        );

        assert!(view.name.is_some());
        assert_eq!(
            std::fs::read_dir(folder.path())
                .expect("deberia leerse el temporal")
                .count(),
            0,
            "decidir el destino ha dejado ficheros"
        );
    }
}
