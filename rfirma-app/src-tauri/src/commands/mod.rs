//! **Las órdenes de Tauri**: lo único que la ventana puede pedirle al backend.
//!
//! Son once, y la lista es cerrada a propósito. Cada una rellena un puerto que
//! la interfaz ya tenía declarado —`CertificateStore`, `Layer2Composer` y
//! `SigningBackend` desde el #76, `DocumentPicker` y `PdfSource` desde el #82,
//! `PreferencesStore` y `LanguagePreference` desde que hay dónde guardar—,
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
//!   producen; [`failure`], la mitad de eso que cuenta lo que ha salido mal.
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
//! # Y hay un camino más, que no es una orden
//!
//! Soltar un fichero en la ventana desemboca en el mismo sitio, pero **al
//! revés**: no lo pide la ventana, le ocurre. Por eso [`dropped_document`] no
//! es una duodécima orden sino lo que alimenta el evento [`DOCUMENT_DROPPED`],
//! que `lib.rs` emite desde el manejador del arrastre nativo (ID-67).
//!
//! # El recorrido está partido en tres porque el PIN va en medio
//!
//! [`begin_signing`] → [`sign_with_pin`] → [`finish_signing`]. El porqué está
//! en [`crate::app::signing`], que es quien lo hace.

pub mod failure;
pub mod orders;
pub mod views;

#[cfg(test)]
mod guards;

use tauri::State;

use crate::app::{self, Environment};
use crate::isolate::Isolate;
use crate::memory::OpenedDocuments;

pub use crate::app::signing::SigningSession;
pub use app::documents::dropped_document;
pub use failure::Failure;
pub use orders::SigningOrder;
pub use views::{
    CertificateView, ConfigurationView, DroppedDocumentView, OpenedDocumentView, SignedDocumentView,
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

/// **Orden 2.** El texto del recuadro, ya compuesto, para la vista previa.
#[tauri::command]
pub fn compose_visible_text(
    order: SigningOrder,
    environment: State<'_, Environment>,
) -> Result<String, Failure> {
    app::signing::visible_text(&order, &environment.stores, &environment.listed)
}

/// **Orden 3.** Prefirma: cruza la frontera y deja el ciclo abierto.
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

/// **Orden 4.** Firma en el token, con el PIN que se acaba de teclear.
///
/// El PIN entra por aquí y no se guarda en ningún sitio: ni en la sesión, ni en
/// el registro, ni de vuelta a la ventana (ADR-0001).
#[tauri::command]
pub fn sign_with_pin(pin: String, session: State<'_, SigningSession>) -> Result<(), Failure> {
    app::signing::sign_on_token(&session, &pin)
}

/// **Orden 5.** Postfirma: comprueba el sello, ensambla el PDF y lo deja caer.
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

/// **Orden 6.** Cancelar: se olvida el ciclo a medias.
#[tauri::command]
pub fn cancel_signing(session: State<'_, SigningSession>) {
    app::signing::cancel(&session);
}

/// **Orden 7.** Abre el diálogo del sistema y apunta lo que el portal conceda.
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

/// **Orden 8.** Los bytes del documento abierto, **como bytes** (ID-66).
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

/// **Orden 9.** Lo que hay guardado, para pintar Preferencias al abrir.
///
/// Lee de la copia viva y no del disco: el fichero se leyó una vez al arrancar
/// (`lib.rs`), y volver a leerlo aquí abriría la puerta a que la ventana y las
/// órdenes de firma vieran configuraciones distintas.
#[tauri::command]
pub fn read_configuration(environment: State<'_, Environment>) -> ConfigurationView {
    app::configuration::shown(&environment.configuration(), &environment.documents_folder)
}

/// **Orden 10.** Guarda lo que el usuario acaba de elegir.
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

/// **Orden 11.** Olvida lo acumulado: los recientes y el certificado.
///
/// Es «Vaciar la lista» y también lo que arrastra apagar «Recordar mi
/// actividad» (ID-34): las dos son la misma promesa y por eso son la misma
/// orden.
#[tauri::command(async)]
pub fn forget_activity(environment: State<'_, Environment>) -> Result<(), Failure> {
    app::configuration::forget_activity(&environment.memory)
}

/// El nombre del evento con el que la ventana se entera de un arrastre.
///
/// Es un **evento** y no una duodécima orden a propósito: el arrastre no lo
/// pide la ventana, le ocurre. En Tauri v2 el arrastre y la soltura del WebView
/// vienen desactivados por omisión a favor del evento nativo (ID-67), así que
/// un manejador de soltura en el JSX no se dispararía nunca; lo que hay debajo
/// es esto, y al otro lado lo recoge el puerto `DocumentDrops`.
pub const DOCUMENT_DROPPED: &str = "document-dropped";
