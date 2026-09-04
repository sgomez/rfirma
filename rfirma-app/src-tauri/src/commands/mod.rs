//! **Las órdenes de Tauri**: lo único que la ventana puede pedirle al backend.
//!
//! Son veintidós, y la lista es cerrada a propósito. Cada una rellena un puerto que
//! la interfaz ya tenía declarado —`CertificateStore`, `Layer2Composer` y
//! `SigningBackend` desde el #76, `DocumentPicker` y `PdfSource` desde el #82,
//! `PreferencesStore` y `LanguagePreference` desde que hay dónde guardar,
//! `RecentsStore` desde el #126, y `RubricPicker` desde el #128—,
//! así que la ventana no aprende nada nuevo de Tauri: sigue hablando con los
//! mismos puertos y es `main.tsx` quien elige estas implementaciones.
//!
//! # Una orden desempaqueta, llama y traduce. No decide nada
//!
//! Ese es el ID-79 y es lo único que hay en los cuerpos de abajo: sacar del
//! `State` lo que ese caso de uso pide, llamarlo, y convertir lo que devuelve
//! en un tipo de [`views`]. Quien decide está en [`crate::app`], y se prueba
//! desde allí llamándolo por su nombre (TD-20). Si un cuerpo de aquí necesita
//! una condición que no sea desempaquetar o traducir, esa condición pertenece a
//! [`crate::app`] y este fichero se ha vuelto a estropear.
//!
//! El reparto del módulo:
//!
//! - [`views`], los tipos que cruzan a la ventana y las conversiones que los
//!   producen; [`failure`], la mitad de eso que cuenta lo que ha salido mal;
//!   [`rubric`], los mismos dos papeles pero solo para la rúbrica —aparte por
//!   tamaño, no porque sea otra cosa.
//! - [`orders`], lo que la ventana manda, ya deserializado.
//! - `guards`, las cuatro pruebas que necesitan ver **todas** las órdenes a la
//!   vez (ID-85). Solo existe en las pruebas.
//!
//! # Los ajustes se guardan al elegirlos, y en el disco
//!
//! [`read_configuration`] y [`write_configuration`] son las dos mitades del
//! puerto `PreferencesStore`, y [`forget_activity`] es lo que promete «Recordar
//! mi actividad» al apagarse. Las tres pasan por [`crate::memory::Memory`], que
//! es el único sitio donde los dos interruptores no se pueden olvidar
//! (ADR-0010).
//!
//! # El documento entra por el portal y se nombra con un identificador
//!
//! [`open_document`] abre el diálogo del sistema **desde Rust** (ID-63), apunta
//! lo que el portal conceda en [`crate::memory::OpenedDocuments`] y devuelve un
//! identificador opaco; [`read_document`] entrega sus bytes contra ese
//! identificador. Ninguna de las dos devuelve una ruta.
//!
//! # La rúbrica se elige con su propio diálogo, y se copia
//!
//! [`choose_rubric`] abre el diálogo del sistema filtrado a imágenes, y
//! adopta lo que el portal conceda en [`crate::rubric::RubricStore`] —se
//! copia, no se referencia (ID-33)—. Cancelar y una imagen que no vale no son
//! lo mismo: cancelar es `None`, una imagen inválida es
//! `Some(RubricChoiceView::refused(..))`, porque el frontal la cuenta con el
//! panel de firma todavía abierto y no como un fallo que reviente la promesa.
//! [`read_rubric`] es la otra mitad: lee lo que un `choose_rubric` de una
//! sesión anterior dejó adoptado, para que la ventana la encuentre puesta al
//! arrancar.
//!
//! # El destino se enseña antes de firmar, y se elige con un selector de directorio
//!
//! [`preview_destination`] contesta lo que el pie del panel enseña: la carpeta
//! y el **nombre** con el que el documento va a caer, más si esa carpeta se
//! puede escribir —de [`crate::destination::CheckedFolder::check`], nunca de un
//! literal (ID-67)—. [`choose_destination`] abre el selector de directorio del
//! sistema y guarda lo que conceda: es un desplegable menos, y un control que
//! fingía elegir menos (ID-65).
//!
//! # Y hay un camino más, que no es una orden
//!
//! Soltar un fichero en la ventana desemboca en el mismo sitio, pero **al
//! revés**: no lo pide la ventana, le ocurre. Por eso [`dropped_document`] no
//! es una orden más sino lo que alimenta el evento [`DOCUMENT_DROPPED`],
//! que `lib.rs` emite desde el manejador del arrastre nativo (ID-67).
//!
//! # La bandeja está en el disco y cada fila recuerda su recuadro
//!
//! [`list_recents`], [`record_recent`] y [`forget_recent`] son tres cuartos del
//! puerto `RecentsStore`; el cuarto, «Vaciar la lista», ya era
//! [`forget_activity`] y no se duplica. Lo que cruza en las tres es el
//! **identificador opaco** (ID-62): la deduplicación de la bandeja sigue siendo
//! por la ruta canónica que solo Rust conoce (ID-75). La insignia `Firmado` no
//! la escribe ninguna de ellas —solo [`finish_signing`], ID-76—.
//!
//! # El recorrido está partido en tres porque el PIN va en medio
//!
//! [`begin_signing`] → [`sign_with_pin`] → [`finish_signing`]. El porqué está
//! en [`crate::app::signing`], que es quien lo hace.
//!
//! # Y hay un cuarto paso que no firma: la vista previa
//!
//! [`preview_signature`] recorre el mismo ciclo con un `PK1` inventado y
//! devuelve el PDF compuesto, sin PIN y sin escribir nada (ID-136). Lo que la
//! ventana pinta dentro del recuadro es entonces **el sello**, no un dibujo
//! parecido.

pub mod failure;
pub mod orders;
pub mod rubric;
pub mod views;

#[cfg(test)]
mod guards;

use tauri::State;

use crate::app::{self, Environment};
use crate::isolate::Isolate;
use crate::memory::OpenedDocuments;

pub use crate::app::invocation::PendingInvocation;
pub use crate::app::signing::SigningSession;
pub use app::documents::dropped_document;
pub use app::invocation::second_invocation;
pub use failure::Failure;
pub use orders::{PlacementOrder, SigningOrder};
pub use rubric::{RubricChoiceView, RubricView};
pub use views::{
    CertificateView, ConfigurationView, DestinationView, DroppedDocumentView, OpenedDocumentView,
    PlacementView, RecentDocumentView, SignedDocumentView,
};

/// **Orden 1.** Los certificados de los tokens conectados.
///
/// No pide el PIN: los certificados son objetos públicos y su estado se decide
/// leyendo el DER. Pedir el secreto que desbloquea la clave para luego decir
/// que el certificado caducó es hacerlo teclear para nada.
#[tauri::command]
pub fn list_certificates(
    environment: State<'_, Environment>,
) -> Result<Vec<CertificateView>, Failure> {
    app::certificates::listed_rows(
        &environment.stores,
        &environment.listed,
        &environment.memory,
    )
}

/// **Orden 2.** Prefirma: cruza la frontera y deja el ciclo abierto.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    opened: State<'_, OpenedDocuments>,
) -> Result<(), Failure> {
    app::signing::begin(
        &order,
        &environment.stores,
        &environment.listed,
        &opened,
        &isolate,
        &session,
    )
}

/// **Orden 3.** Firma en el token, con el PIN que se acaba de teclear.
///
/// El PIN entra por aquí y no se guarda en ningún sitio: ni en la sesión, ni en
/// el registro, ni de vuelta a la ventana (ADR-0001).
#[tauri::command]
pub fn sign_with_pin(pin: String, session: State<'_, SigningSession>) -> Result<(), Failure> {
    app::signing::sign_on_token(&session, &pin)
}

/// **Orden 4.** Postfirma: comprueba el sello, ensambla el PDF y lo deja caer.
#[tauri::command]
pub fn finish_signing(
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
) -> Result<SignedDocumentView, Failure> {
    app::signing::finish(
        &isolate,
        &session,
        &environment.memory,
        &environment.configuration(),
        &environment.documents_folder,
    )
}

/// **Orden 5.** Cancelar: se olvida el ciclo a medias.
#[tauri::command]
pub fn cancel_signing(session: State<'_, SigningSession>) {
    app::signing::cancel(&session);
}

/// **Orden 6.** Abre el diálogo del sistema y apunta lo que el portal conceda.
///
/// El diálogo se abre **desde aquí y no desde el frontal** (ID-63): así la
/// ventana sigue con un solo fichero que conoce `invoke`, y la lista de
/// permisos de `capabilities/default.json` no crece, porque los permisos de
/// Tauri v2 vigilan lo que la ventana puede pedir y no lo que Rust hace.
/// Filtra por PDF porque es lo único que la aplicación sabe firmar (ID-64).
///
/// Cerrar el diálogo sin elegir nada devuelve `None`, que **no es un fallo**:
/// es lo que deja el documento activo, la lista y el visor como estaban
/// (ID-73).
///
/// El diálogo se abre en la última carpeta usada, y donde esa no se puede
/// saber, en la de destino: ver [`crate::app::documents::starting_folder`].
#[tauri::command(async)]
pub fn open_document(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<Option<OpenedDocumentView>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let configuration = environment.configuration();
    let mut dialog = app_handle.dialog().file().add_filter("PDF", &["pdf"]);
    if let Some(folder) = app::documents::starting_folder(
        &environment.memory,
        &configuration,
        &environment.documents_folder,
    ) {
        dialog = dialog.set_directory(folder);
    }
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let handle = chosen
        .into_path()
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    Ok(Some(app::documents::note_opened(
        &environment.memory,
        &configuration,
        &opened,
        handle,
    )))
}

/// **Orden 7.** Los bytes del documento abierto, **como bytes** (ID-66).
///
/// Devuelve una [`tauri::ipc::Response`] y no un `Vec<u8>`: serializado a JSON,
/// un PDF de unos pocos megabytes se convierte en un array de miles de números
/// y multiplica el tamaño y el tiempo. Esta es la respuesta binaria que el
/// puente de Tauri ofrece justo para esto, y al otro lado llega un
/// `ArrayBuffer` que `pdf.js` abre sin nada en medio.
#[tauri::command(async)]
pub fn read_document(
    id: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(app::documents::bytes_of(
        &opened, &id,
    )?))
}

/// **Orden 8.** Lo que hay guardado, para pintar Preferencias al abrir.
///
/// Lee de la copia viva y no del disco: el fichero se leyó una vez al arrancar
/// (`lib.rs`), y volver a leerlo aquí abriría la puerta a que la ventana y las
/// órdenes de firma vieran configuraciones distintas.
#[tauri::command]
pub fn read_configuration(environment: State<'_, Environment>) -> ConfigurationView {
    app::configuration::shown(&environment.configuration(), &environment.documents_folder)
}

/// **Orden 9.** Guarda lo que el usuario acaba de elegir.
#[tauri::command(async)]
pub fn write_configuration(
    configuration: ConfigurationView,
    environment: State<'_, Environment>,
) -> Result<(), Failure> {
    app::configuration::write(
        &environment.memory,
        &environment.configuration,
        &configuration,
    )
}

/// **Orden 10.** Olvida lo acumulado: los recientes y el certificado.
///
/// Es «Vaciar la lista» y también lo que arrastra apagar «Recordar mi
/// actividad» (ID-34): las dos son la misma promesa y por eso son la misma
/// orden.
#[tauri::command(async)]
pub fn forget_activity(environment: State<'_, Environment>) -> Result<(), Failure> {
    app::configuration::forget_activity(&environment.memory)
}

/// **Orden 11.** La bandeja entera, la más reciente primero.
///
/// `available` se **recalcula aquí** contra el disco de ahora mismo y no se
/// persiste nunca: una ruta que no responde sale con `available: false` —la
/// ventana la pinta `No disponible`— y la fila **revive** cuando la ruta
/// reaparece. Nadie la purga por su cuenta.
///
/// No abre ni un PDF: la fila se pinta con lo cacheado (ADR-0010).
#[tauri::command(async)]
pub fn list_recents(
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Vec<RecentDocumentView> {
    app::recents::listed_rows(&environment.memory, &opened)
}

/// **Orden 12.** Anota en la bandeja el documento abierto, y dónde cayó su
/// recuadro.
///
/// Devuelve la fila ya lista para pintar porque es donde la ventana recupera lo
/// que ya se sabía del documento: su insignia cacheada y su recuadro. El
/// recuadro entra entero y se guarda partido (ID-74).
#[tauri::command(async)]
pub fn record_recent(
    id: String,
    placement: Option<PlacementView>,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<RecentDocumentView, Failure> {
    app::recents::record(
        &environment.memory,
        &environment.configuration(),
        &opened,
        &id,
        placement,
    )
}

/// **Orden 13.** Quita una fila de la bandeja.
///
/// Es lo único que saca una fila. Vaciar la lista entera es
/// [`forget_activity`], que además se lleva el certificado.
#[tauri::command(async)]
pub fn forget_recent(
    id: String,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<(), Failure> {
    app::recents::forget(
        &environment.memory,
        &environment.configuration(),
        &opened,
        &id,
    )
}

/// **Orden 14.** Abre el diálogo del portal y adopta la imagen elegida como
/// rúbrica.
///
/// Filtra por PNG y JPEG, que es lo único que
/// [`crate::rubric::normalize`] admite. Se abre **desde aquí y no desde el
/// frontal**, por la misma razón que [`open_document`] (ID-63): la ventana
/// sigue sin pedir el permiso del diálogo.
///
/// Cerrar el diálogo sin elegir nada devuelve `None`, y **no es un fallo**: es
/// lo que deja la rúbrica ya elegida como estaba (ID-73). Una imagen que no
/// vale —no es PNG ni JPEG, está dañada, pasa del tope— tampoco es un fallo
/// que reviente la promesa: viaja como `RubricChoiceView::refused`, con el
/// panel de firma todavía abierto (ADR-0010), porque es justo lo que
/// [`crate::signing::rubric::RubricPicker`] del frontal espera encontrar en su
/// `RubricChoice`.
#[tauri::command(async)]
pub fn choose_rubric(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
) -> Option<RubricChoiceView> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Imagen", &["png", "jpg", "jpeg"]);
    let chosen = dialog.blocking_pick_file()?;
    Some(match app::rubric::choose(&environment.rubric, chosen) {
        Ok(normalized) => RubricChoiceView::adopted(&normalized),
        Err(error) => RubricChoiceView::refused(&error),
    })
}

/// **Orden 15.** La rúbrica ya adoptada, si la hay, para que una sesión nueva
/// la encuentre puesta (ID-33).
///
/// El JPEG sobrevive en [`crate::rubric::RubricStore`] aunque se cierre la
/// aplicación; sin esta orden nadie lo leía nunca en producción y «Tu
/// rúbrica» arrancaba siempre apagada. Se llama una vez, al montar. Es
/// `(async)` como [`list_recents`]: lee del disco, y no de la copia viva que
/// [`read_configuration`] sí tiene a mano.
#[tauri::command(async)]
pub fn read_rubric(environment: State<'_, Environment>) -> Result<Option<RubricView>, Failure> {
    let stored = app::rubric::stored(&environment.rubric)?;
    Ok(stored.map(|bytes| RubricView::from_bytes(&bytes)))
}

/// **Orden 16.** Dónde va a caer el documento que hay delante, **antes** de
/// firmarlo.
///
/// Es lo que el pie del panel enseña: la carpeta y el nombre, los dos por su
/// nombre (ID-63). Escribe nada y **no crea la carpeta**; que no esté o no se
/// deje escribir viaja como un destino no escribible y no como un fallo, porque
/// el botón de firmar sigue vivo y lo que se ofrece es `Cambiar` (ID-67).
///
/// Es `(async)` como [`list_recents`]: mira el disco —la carpeta y sus
/// homónimos— y no la copia viva.
#[tauri::command(async)]
pub fn preview_destination(
    id: String,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<DestinationView, Failure> {
    let document = app::documents::opened_document(&opened, &id)?;
    Ok(app::documents::where_it_lands(
        &environment.configuration(),
        &environment.documents_folder,
        &document,
    ))
}

/// **Orden 17.** Abre el selector de directorio del sistema y guarda la carpeta
/// de destino que conceda.
///
/// Sustituye al desplegable que recibía una sola opción, que es un control que
/// fingía elegir (ID-65). Se abre **desde aquí y no desde el frontal**, por la
/// misma razón que [`open_document`] (ID-63), y lo que vuelve es el **último
/// segmento** de lo concedido: un directorio del portal llega como
/// `/run/user/1000/doc/<id>/Documentos`, cuyo último segmento es el nombre de la
/// carpeta, así que la ventana enseña lo mismo conozcamos la ruta real o no
/// (ADR-0011).
///
/// Cerrar el diálogo sin elegir devuelve `None`, y **no es un fallo**: deja la
/// carpeta que hubiera.
#[tauri::command(async)]
pub fn choose_destination(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
) -> Result<Option<String>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let Some(chosen) = app_handle.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let folder = chosen
        .into_path()
        .map_err(|error| Failure::new("folderMissing", error.to_string()))?;
    app::configuration::choose_destination(
        &environment.memory,
        &environment.configuration,
        crate::destination::DestinationFolder::at(folder),
    )
    .map(Some)
}

/// **Orden 18.** Abre el PDF firmado con el visor del sistema.
///
/// Bajo el sandbox esto **no es comodidad**: la ventana nunca conoce la ruta
/// del fichero (ADR-0011) y el usuario tampoco la ve, así que este botón y el
/// siguiente son la única forma que tiene de llegar a lo que acaba de firmar
/// (ID-79).
///
/// Por eso la orden **no recibe ninguna ruta**: la que se abre es la del
/// último documento entregado, que guarda la sesión de firma. Lo que la
/// ventana no tiene no lo puede pedir mal.
///
/// Debajo es el portal `OpenURI`, que fuera del sandbox cae en `xdg-open`.
#[tauri::command(async)]
pub fn open_signed_document(
    app_handle: tauri::AppHandle,
    session: State<'_, SigningSession>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let landing = app::signing::signed_document(&session)?;
    app_handle
        .opener()
        .open_path(landing.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}

/// **Orden 19.** Abre la carpeta donde quedó el PDF firmado.
///
/// La carpeta es la del fichero del resumen y no la de destino leída otra vez:
/// si el usuario la ha cambiado desde que firmó, abrir la nueva le enseñaría un
/// directorio donde su documento no está.
///
/// El mismo portal que [`open_signed_document`], con el directorio en vez del
/// fichero: el gestor de archivos lo abre y el usuario ve dentro lo que acaba
/// de firmar, junto a las firmas anteriores que **siguen ahí** (ID-81).
#[tauri::command(async)]
pub fn open_signed_folder(
    app_handle: tauri::AppHandle,
    session: State<'_, SigningSession>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let folder = app::signing::signed_folder(&session)?;
    app_handle
        .opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}

/// **Orden 20.** El PDF con el sello que va a quedar, compuesto sin firmar
/// (ID-136).
///
/// La **prefirma en seco**: el ciclo trifásico entero con un `PK1` inventado,
/// para que la ventana pinte dentro del recuadro lo que va a quedar de verdad y
/// no una aproximación dibujada. **No pide PIN** y **no toca el disco de
/// destino**; el porqué de las dos está en [`crate::app::preview`].
///
/// Devuelve una [`tauri::ipc::Response`] por lo mismo que [`read_document`]: un
/// PDF serializado a JSON es un array de miles de números.
///
/// Es `(async)` porque el trabajo se va al hilo del isolate y la espera es de
/// segundos en un documento grande —≈1,9 s en un escaneado de 37 MB—: en el
/// hilo del bucle de eventos eso es la ventana clavada.
#[tauri::command(async)]
pub fn preview_signature(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(app::preview::compose(
        &order,
        &environment.stores,
        &environment.listed,
        &opened,
        &isolate,
    )?))
}

/// **Orden 21.** La esquina inferior izquierda del recuadro, en puntos PAdES
/// (ID-105).
///
/// `correctPositionSignature` (`PdfUtil.java:607-632`) descarta en silencio,
/// antes de firmar, cualquier página del conjunto donde esta esquina no cabe
/// — comparada contra el ancho y el alto de **cada** página. El diálogo de
/// páginas sin sello anticipa esa guardia, pero la conversión de espacio de
/// usuario a puntos PAdES (`T⁻¹` de la `/Rotate`, `signing::placement`) no
/// tiene copia en TypeScript: la pide aquí, en vez de recalcularla del lado
/// de la ventana.
#[tauri::command]
pub fn pades_lower_left(placement: PlacementOrder) -> Result<[i32; 2], Failure> {
    let placement = placement.placement()?;
    Ok([placement.rect.lower_left_x, placement.rect.lower_left_y])
}

/// **Orden 22.** El documento con el que se invocó a la aplicación, si vino con
/// alguno (ID-157).
///
/// La pide la ventana una sola vez, al montarse, y lo que devuelve es lo mismo
/// que emite un arrastre: la invocación termina en la ventana completa, en el
/// estado en que la deja arrastrar un PDF (ID-159).
///
/// Es una orden y no un evento —al revés que [`DOCUMENT_DROPPED`]— porque el
/// documento se conoce **antes** de que haya nadie escuchando: emitirlo al
/// arrancar sería emitirlo al vacío. Se consume al leerla.
#[tauri::command]
pub fn read_invocation(
    pending: State<'_, PendingInvocation>,
    opened: State<'_, OpenedDocuments>,
) -> Option<DroppedDocumentView> {
    let invocation = pending.take()?;
    app::invocation::invoked_document(&invocation, &opened)
}

/// El nombre del evento con el que la ventana se entera de un arrastre.
///
/// Es un **evento** y no una orden más a propósito: el arrastre no lo
/// pide la ventana, le ocurre. En Tauri v2 el arrastre y la soltura del WebView
/// vienen desactivados por omisión a favor del evento nativo (ID-67), así que
/// un manejador de soltura en el JSX no se dispararía nunca; lo que hay debajo
/// es esto, y al otro lado lo recoge el puerto `DocumentDrops`.
pub const DOCUMENT_DROPPED: &str = "document-dropped";

#[cfg(test)]
mod tests {
    use super::{pades_lower_left, PlacementOrder};

    /// El mismo ejemplo numérico del hallazgo: con `/Rotate 0` la esquina
    /// PAdES coincide con la de espacio de usuario, que es el único caso que
    /// cubrían las pruebas de `unsealedPages.test.ts` antes de este cambio.
    #[test]
    fn matches_user_space_when_the_page_is_not_rotated() {
        let placement: PlacementOrder = serde_json::from_value(serde_json::json!({
            "page": 1,
            "pages": { "only": [1] },
            "pageCount": 1,
            "mediaBox": [0.0, 0.0, 595.0, 842.0],
            "rotation": 0,
            "rect": [250.0, 50.0, 450.0, 100.0],
        }))
        .expect("la orden del visor");

        assert_eq!(
            pades_lower_left(placement).expect("cabe en la pagina"),
            [250, 50]
        );
    }

    /// Con `/Rotate 90` la esquina PAdES **no** coincide con la de espacio de
    /// usuario: es el caso que el hallazgo señala como el que hacía que el
    /// diálogo avisara de páginas que no se caían, o al revés.
    #[test]
    fn diverges_from_user_space_when_the_page_is_rotated() {
        let placement: PlacementOrder = serde_json::from_value(serde_json::json!({
            "page": 1,
            "pages": { "only": [1] },
            "pageCount": 1,
            "mediaBox": [0.0, 0.0, 595.0, 842.0],
            "rotation": 90,
            "rect": [250.0, 50.0, 450.0, 100.0],
        }))
        .expect("la orden del visor");

        assert_eq!(
            pades_lower_left(placement).expect("cabe en la pagina"),
            [50, 145]
        );
    }
}
