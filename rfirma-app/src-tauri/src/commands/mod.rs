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
    CertificateView, ConfigurationView, DestinationView, DroppedDocumentView, NewVersionView,
    NoCertificateView, NoChannelView, OpenedDocumentView, PlacementView, RecentDocumentView,
    RefusalSituationView, SecretView, SignatureRoundView, SignedDocumentView, SiteErrandView,
    SiteOutcomeView, SiteStageView, UrlHandlerView, UrlHandlersView,
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
        &environment.all_stores(),
        &environment.installed_certificates,
        &environment.listed,
        &environment.memory,
    )
}

/// **Orden 2.** Prefirma: cruza la frontera y deja el ciclo abierto.
///
/// Devuelve **cómo hay que pedirle el secreto al almacén** (ID-189): sin sesión
/// no hay diálogo que abrir, y con ella la ventana sabe que toca pedirlo.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    opened: State<'_, OpenedDocuments>,
) -> Result<SecretView, Failure> {
    app::signing::begin(
        &order,
        &environment.all_stores(),
        &environment.listed,
        &opened,
        &isolate,
        &session,
    )
    .map(SecretView::from)
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
    app::in_hand::take(
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
        &environment.all_stores(),
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

/// **Orden 23.** Si hay publicada una versión más nueva que esta, cuál es.
///
/// La pide la ventana al montarse y con eso pinta la franja (ID-181). No hay
/// nada que descargar ni que instalar (ID-177): lo que devuelve es un número.
///
/// `None` es el silencio del ID-180, y cubre las tres formas de no tener nada
/// que decir: no hay versión nueva, no hay red, o GitHub contestó algo que no
/// se entiende. **Ninguna es un [`Failure`]**: nadie ha pedido esta consulta,
/// así que nada de lo que le pase merece un aviso.
///
/// `(async)` no es decorativo: el puerto abre una conexión, y en el hilo del
/// bucle de eventos eso sería la ventana clavada hasta que GitHub conteste.
///
/// El puerto se le pasa aquí —[`crate::releases::latest_release`]— igual que el
/// entorno se le pasa a [`crate::paths`]: el caso de uso no sabe de dónde sale
/// la cadena, y por eso las pruebas lo doblan sin abrir un socket (ID-182,
/// TD-39).
#[tauri::command(async)]
pub fn check_for_new_version(environment: State<'_, Environment>) -> Option<NewVersionView> {
    let announced = app::version::new_version(
        app::version::Version::running(),
        &environment.memory,
        &crate::releases::latest_release,
        std::time::SystemTime::now(),
    )?;

    Some(NewVersionView {
        version: announced.to_string(),
    })
}

/// **Orden 24.** Instala un `.p12` como almacén propio de rfirma (ID-192).
///
/// Abre el diálogo del portal filtrado a `.p12` y `.pfx` —**desde aquí y no
/// desde el frontal**, por lo mismo que [`choose_rubric`] (ID-63)— y mete lo
/// que traiga dentro en un almacén NSS nuevo. `password` es la contraseña
/// **del fichero**, que es lo único que hace falta para abrirlo y lo único que
/// la ventana pide: del fichero no se recuerda nada, ni la ruta ni una copia
/// (ID-196), así que la ventana no lo nombra ni antes ni después.
///
/// Devuelve `false` cuando el diálogo se cerró sin elegir nada, que **no es un
/// fallo**: es lo que deja la lista de instalados como estaba. Un fichero que
/// no se puede abrir —contraseña que no es la suya— o que trae una clave que no
/// es RSA (ID-197) sí es un [`Failure`], y entonces no queda instalado nada.
#[tauri::command(async)]
pub fn install_certificate(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    password: String,
) -> Result<bool, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Certificado", &["p12", "pfx"]);
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(false);
    };

    app::certificates::install_pkcs12(&environment.installed_certificates, chosen, &password)
        .map(|()| true)
}

/// **Orden 25.** Quita un `.p12` instalado, por el asa de su fila.
///
/// Lo que se borra es el almacén entero, que es lo único que quedó del fichero.
/// Un asa que no sea de un `.p12` instalado —la de un certificado del perfil de
/// Firefox— se rechaza sin tocar nada.
#[tauri::command(async)]
pub fn remove_certificate(id: String, environment: State<'_, Environment>) -> Result<(), Failure> {
    app::certificates::remove_installed(
        &environment.installed_certificates,
        &id,
        &environment.listed,
    )
}

/// **Orden 26.** Quién atiende los enlaces `afirma://`, para pintar
/// Preferencias y decidir si sale el banner (ID-238, ID-239, ID-240).
///
/// Dentro del flatpak contesta `available: false` sin llamar a nada, y esa es
/// toda la respuesta: la frase fija que se enseña entonces es de la ventana.
///
/// Sin `$HOME` ni `$XDG_CONFIG_HOME` no hay `mimeapps.list` que leer, y eso no
/// es un fallo que merezca un aviso: se contesta que no se sabe quién atiende,
/// igual que cuando el fichero no existe todavía.
#[tauri::command(async)]
pub fn url_handlers() -> UrlHandlersView {
    let channel = crate::desktop::Channel::detected();
    let list = crate::desktop::choice::mimeapps_list_from_environment().unwrap_or_default();
    app::handlers::who_handles(channel, &list)
}

/// **Orden 27.** Deja apuntado quién atiende los enlaces `afirma://` (ID-238).
///
/// `handler` es uno de los ficheros `.desktop` que dio la orden anterior: aquí
/// no se cablea ningún nombre de aplicación, ni el de rFirma.
#[tauri::command(async)]
pub fn choose_url_handler(handler: String) -> Result<(), Failure> {
    let channel = crate::desktop::Channel::detected();
    // La clave del catálogo la nombra `situation_name`, que es quien ata esta
    // situación con la que trae `From<DesktopError>`: cablearla aquí serían dos
    // sitios diciendo lo mismo sin nada que los obligue.
    let list = crate::desktop::choice::mimeapps_list_from_environment().map_err(|error| {
        Failure::new(
            app::handlers::situation_name(crate::desktop::error::Situation::TheListIsNotWritable),
            error.to_string(),
        )
    })?;
    app::handlers::chosen(channel, &list, &handler)
}

/// **Orden 28.** Si el documento abierto trae **firmas que rFirma no sabe
/// leer** (ID-297, ID-300).
///
/// La ventana la llama justo antes de firmar, y con un `true` enseña el aviso y
/// pide permiso: sin ese permiso la orden de firma no lleva
/// `allowCosigningUnregisteredSignatures` y el puente aborta la cofirma
/// (ID-301). No dice cuántas firmas hay ni de quién son, y no las valida
/// (ID-305): la pregunta es de sí o no.
#[tauri::command(async)]
pub fn unregistered_signatures(
    document: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<bool, Failure> {
    app::signing::unregistered_signatures_in(&opened, &document)
}

/// **Orden 29.** Cierra la ventana de sede. La sede ya tiene su respuesta.
///
/// Es `async` como todas las órdenes del trámite (ID-337): una orden sobre una
/// `fn` que no lo es corre en el hilo del bucle de eventos, y ahí cerrar la
/// ventana desde dentro de su propio manejador de IPC es pedirle al bucle que
/// se espere a sí mismo.
///
/// Cierra **la ventana de la etiqueta [`SITE_WINDOW`]**, y no la que pregunta:
/// hoy sólo la invoca `sede.html`, pero el nombre de la orden promete la de
/// sede y es la que cierra. Que no exista es una respuesta válida —cerrar dos
/// veces es lo mismo que cerrar una—.
///
/// No devuelve nada: cerrar la ventana que está preguntando no deja a nadie a
/// quien contarle que no se ha podido. El trámite **no termina aquí** —termina
/// al contestarle a la sede (ID-275)—, así que esto no toca
/// [`crate::app::errand::LiveErrand`].
#[tauri::command(async)]
pub fn close_site_window(app: tauri::AppHandle) {
    use tauri::Manager as _;

    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.close();
    }
}

/// **Orden 30.** La persona se identifica ante la sede con uno de los
/// certificados que tenía delante (ID-276).
///
/// El certificado sale al cable **desde el caso de uso** (ID-322), y lo que
/// vuelve a la ventana es sólo el desenlace: el trámite ya terminó para la
/// sede, que no espera a que nadie cierre nada (ID-275).
///
/// Es `async` como todas las órdenes del trámite (ID-337).
#[tauri::command(async)]
pub fn site_identify(
    certificate: String,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    live: State<'_, app::errand::LiveErrand>,
    consent: State<'_, SiteConsent>,
) -> Result<(), Failure> {
    let Some(filter) = consent.what_the_site_asked() else {
        return Err(Failure::new(
            "siteErrandNotLive",
            "no hay ninguna identificacion pendiente que contestar",
        ));
    };
    consent.forget();

    let reply = app::errand::identify_with(
        &TheBridge::borrowed_from(&isolate),
        &environment.all_stores(),
        &filter,
        &certificate,
        &environment.listed,
        &live,
    );

    match reply.failure() {
        Some(failure) => Err(failure.clone()),
        None => Ok(()),
    }
}

/// **Orden 31.** La persona dice que no: la sede recibe `CANCEL` en el acto
/// (ID-293, ID-275).
///
/// Contestada la sede, ya no queda asa: cancelar dos veces —o cerrar la ventana
/// después de haber contestado— no escribe nada (ID-340).
///
/// Es `async` como todas las órdenes del trámite (ID-337).
#[tauri::command(async)]
pub fn site_decline(live: State<'_, app::errand::LiveErrand>, consent: State<'_, SiteConsent>) {
    consent.forget();
    app::errand::declined(&live);
}

/// **Orden 32.** La persona ha consentido firmar, y elige con qué certificado
/// (ID-272).
///
/// Es [`begin_signing`] del trámite de sede, y de la ventana llega **sólo el
/// asa del certificado**: el documento, el filtro y la política que declaró la
/// sede salen del consentimiento que este mismo adaptador apuntó, porque
/// hacerlos cumplir no es cosa de la ventana (ID-259, ID-266).
///
/// Devuelve cómo hay que pedirle el secreto al almacén, igual que su gemela
/// local: el PIN entra después por [`sign_with_pin`], que es la misma orden
/// para los dos recorridos porque la fase que toca la clave privada no sabe de
/// sedes (ADR-0001).
///
/// Si la prefirma no sale, la sede se entera en el acto y el trámite se cierra
/// (ID-275); la ventana recibe además la situación entera (ID-29).
///
/// Es `async` como todas las órdenes del trámite (ID-337).
#[tauri::command(async)]
pub fn site_begin_signing(
    certificate: String,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    opened: State<'_, OpenedDocuments>,
    live: State<'_, app::errand::LiveErrand>,
    consent: State<'_, SiteConsent>,
) -> Result<SecretView, Failure> {
    let Some(pending) = consent.the_signature_consented() else {
        return Err(Failure::new(
            "siteErrandNotLive",
            "no hay ninguna firma pendiente que consentir",
        ));
    };

    let bridge = TheBridge::borrowed_from(&isolate);
    let order = SigningOrder {
        document: pending.document,
        certificate,
        // La sede coloca su recuadro en sus propios `extraParams`, y ésos
        // cruzan al puente crudos: aquí no hay visor sobre el que arrastrar
        // nada, y emitir una colocación nuestra movería el suyo (ID-282).
        placement: None,
        fields: orders::VisibleFieldsOrder::default(),
        reason: String::new(),
        signed_at: String::new(),
        rubric: None,
        language: String::new(),
        allow_unregistered_signatures: pending.unregistered_signatures,
    };

    app::signing::begin_for_the_site(
        &app::signing::SiteSigning {
            engine: &bridge,
            filter: &pending.filter,
            from_the_site: &pending.from_the_site,
        },
        &order,
        &environment.all_stores(),
        &environment.listed,
        &opened,
        &isolate,
        &session,
    )
    .map(SecretView::from)
    // El código que va al cable lo trae la negativa desde donde la situación
    // todavía tenía tipo (ID-292): aquí sólo se separa lo que recibe la sede
    // de lo que recibe la ventana, que es la situación entera (ID-291).
    .map_err(|refusal| {
        consent.forget();
        let failure = refusal.failure().clone();
        app::errand::the_signature_did_not_come_out(&live, refusal);
        failure
    })
}

/// **Orden 33.** Postfirma del trámite de sede: la sede recibe el certificado y
/// el PDF firmado, y con eso el trámite termina (ID-275).
///
/// **No devuelve el documento**, y esa ausencia es la decisión: el firmado de
/// una sede no cae en ninguna carpeta, no anota fila en la bandeja y no cambia
/// el certificado recordado (ID-264, ID-286). Lo que la ventana enseña después
/// es un desenlace, no un fichero.
///
/// Es `async` como todas las órdenes del trámite (ID-337).
#[tauri::command(async)]
pub fn site_finish_signing(
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    live: State<'_, app::errand::LiveErrand>,
    consent: State<'_, SiteConsent>,
) -> Result<(), Failure> {
    consent.forget();

    let signed = app::signing::finish_for_the_site(&isolate, &session).map_err(|refusal| {
        let failure = refusal.failure().clone();
        app::errand::the_signature_did_not_come_out(&live, refusal);
        failure
    })?;
    app::errand::signature_handed_over(&live, &signed);
    Ok(())
}

/// **Orden 34.** Lleva a instalar un certificado desde la ventana de sede
/// (ID-278, ID-341).
///
/// Es el arreglo de «no tienes ninguno», y por eso es la acción principal de
/// esa pantalla. Abre el mismo diálogo del portal que [`install_certificate`] y
/// mete el `.p12` en un almacén propio: **la instalación es una sola**, y ésta
/// existe para que la ventana de sede no tenga que conocer la contraseña de la
/// ventana principal ni al revés.
///
/// Devuelve `false` cuando el diálogo se cerró sin elegir nada, que no es un
/// fallo: es lo que deja el almacén como estaba, y la pantalla igual.
///
/// Es `async` como todas las órdenes del trámite (ID-337), y aquí además es
/// obligatorio: dentro llama al `blocking_pick_file` del complemento de
/// diálogo, que en el hilo del bucle de eventos se cuelga para siempre y sin
/// error visible.
#[tauri::command(async)]
pub fn site_install_certificate(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    password: String,
) -> Result<bool, Failure> {
    install_certificate(app_handle, environment, password)
}

/// **Orden 35.** Vuelve a mirar el almacén, por si se instaló un certificado
/// con la ventana abierta (ID-278, ID-341).
///
/// **Continúa el trámite, no lo reinicia**: se vuelve a atender la petición que
/// la sede mandó por el canal —la que [`crate::app::errand::LiveErrand`] tiene
/// apuntada—, con el mismo canal, la misma asa y el mismo trámite vivo. La sede
/// no ha recibido nada todavía y no tiene que invocar otra vez.
///
/// Sin trámite vivo no hay nada que volver a mirar, y es la respuesta correcta:
/// quien llegue aquí después de que el trámite haya contestado no mueve nada.
///
/// Es `async` como todas las órdenes del trámite (ID-337).
#[tauri::command(async)]
pub fn site_look_again(app_handle: tauri::AppHandle) {
    use tauri::Manager as _;

    let Some(url) = app_handle.state::<app::errand::LiveErrand>().the_request() else {
        return;
    };

    dispatch_the_site_operation(&app_handle, &url);
}

/// **Dónde está la CA local y qué perfiles NSS hay que dejarla de confianza**
/// (ID-329), para que la ventana de sede pueda instalarla cuando el canal no
/// llega a abrirse.
///
/// Las mismas dos cosas que el arranque le pasa a
/// [`crate::app::startup::attend_startup`], sostenidas aquí porque la orden 36
/// las necesita mucho después: el arranque las resuelve una vez y esto es su
/// copia viva.
pub struct LocalCaTrust {
    /// Las dos ranuras de la CA local: la que sirve y la del solape.
    pub store: crate::tls::LocalCaStore,
    /// Los perfiles NSS que se intentan recorrer.
    pub profiles: Vec<std::path::PathBuf>,
}

/// **Orden 36.** Instala la CA local en los almacenes NSS de la persona
/// (ID-329, ID-341).
///
/// Es la **acción principal** de la pantalla de reparación: sin la CA local
/// ningún navegador llega a intentar el canal, así que el resto de la receta
/// —el permiso de red local— sobra hasta que esté. La pide la persona, con el
/// botón delante; no es un refresco automático a mitad de trámite, que es lo
/// que el ID-224 prohíbe.
///
/// Por eso el momento que se le pasa al caso de uso es [`Moment::Startup`] y no
/// [`Moment::MidErrand`]: lo que se pide es exactamente el trabajo del arranque
/// —instalar la que hay, o fabricarla si no la hay—, mientras que `MidErrand`
/// está definido como «no hacer nada». El aviso de reiniciar el navegador se
/// descarta aquí y no se enseña: la ventana de sede no tiene dónde ponerlo, y
/// esa es la mitad del ID-224 que sigue en pie.
///
/// Lo que la ventana ve después es el resultado, publicado por el mismo evento
/// que todo lo demás (ID-338), y son **dos preguntas y no una**: si la CA local
/// ha quedado en algún almacén, y si hay canal sirviendo. Las decide
/// [`what_the_repair_leaves`].
///
/// Es `async` como todas las órdenes del trámite (ID-337).
#[tauri::command(async)]
pub fn install_local_ca(
    app_handle: tauri::AppHandle,
    trust: State<'_, LocalCaTrust>,
    held: State<'_, app::startup::HeldChannel>,
) {
    use crate::trust::{Moment, NssTrustStores};

    let in_some_store = app::trust::refresh_local_ca_trust(
        &trust.store,
        &trust.profiles,
        &NssTrustStores,
        Moment::Startup,
    )
    .is_ok_and(|outcome| !outcome.nowhere());

    let view = what_the_repair_leaves(in_some_store, held.is_serving());
    publish_to_the_site_window(&app_handle, view);
}

/// **En qué queda la pantalla de reparación después de instalar la CA local**
/// (ID-341).
///
/// Las dos preguntas son distintas y hasta el #402 se confundían: que la CA haya
/// entrado en un almacén NSS no dice que el canal esté en pie. Al botón se llega
/// desde tres sitios —`LocalCaMissing`, `ChannelNotOpened` y la espera pasada
/// [`UNREACHABLE_AFTER_MS`](SiteErrandView)— y sólo desde el primero es cierto
/// que el canal sigue sirviendo.
///
/// Y el canal **no se reabre desde aquí**: [`crate::app::site::open_the_channel`]
/// se llama una sola vez, en el arranque, y allí se emite el certificado del
/// servidor. Así que con la CA instalada pero sin canal la respuesta correcta es
/// la pantalla de reparación definitiva —la que lleva la dirección del ajuste
/// del navegador—, no treinta segundos de «Conectando con la sede» sobre algo
/// que el backend ya sabe que no va a llegar.
fn what_the_repair_leaves(in_some_store: bool, channel_is_serving: bool) -> SiteErrandView {
    match (in_some_store, channel_is_serving) {
        // Sin CA en ningún almacén ningún navegador llega a intentar el canal:
        // la reparación sigue siendo instalarla.
        (false, _) => SiteErrandView::no_channel(NoChannelView::LocalCaMissing),
        // Con CA y con canal la petición de la sede puede llegar ya.
        (true, true) => SiteErrandView::waiting(),
        // Con CA y sin canal, lo que de verdad le pasa a la persona.
        (true, false) => SiteErrandView::no_channel(NoChannelView::ChannelNotOpened),
    }
}

/// **Lo que la sede pidió, hasta que la persona conteste.**
///
/// Vive en el adaptador y no en [`crate::app::errand::Errand`] a propósito: el
/// trámite guarda la credencial, el puerto y por dónde se contesta (ID-321), y
/// la operación la lleva quien la está atendiendo. Quien la atiende es
/// [`attend_site_operation`], y esto es su memoria entre el momento del
/// consentimiento y la respuesta.
///
/// Hace falta porque lo que la sede pidió **se vuelve a comprobar antes de
/// entregar nada** (ID-259, ID-266): que el certificado estuviera en la lista
/// que la ventana enseñó no basta, y la ventana no puede devolver ni un filtro
/// ni una política que nunca cruzaron.
#[derive(Default)]
pub struct SiteConsent(std::sync::Mutex<Option<PendingConsent>>);

/// Lo que queda pendiente de contestar, según lo que la sede pidiera.
enum PendingConsent {
    /// `selectcert`: para entregar identidad basta con volver a comprobar el
    /// filtro (ID-276).
    Identity(crate::protocol::SiteFilter),
    /// `sign` o `cosign`: además del filtro hacen falta el documento y la
    /// política que la sede declaró.
    Signature(PendingSignature),
}

/// **Lo que hace falta para firmar cuando la persona ya ha consentido**, y que
/// la ventana no puede devolver.
///
/// Es la mitad del consentimiento que **no** es para mirar: las filas, la ronda
/// y el aviso de las firmas ilegibles se los lleva la ventana en su
/// [`SiteStageView`]; esto se queda aquí porque es lo que hace cumplir lo que
/// pidió la sede, y eso no se le pregunta a la ventana (ID-259, ID-266).
#[derive(Clone)]
struct PendingSignature {
    /// El asa del documento que mandó la sede, la misma que cruzó a la ventana.
    document: String,
    /// Lo que la sede pide del listado, que se vuelve a comprobar (ID-259).
    filter: crate::protocol::SiteFilter,
    /// Los `extraParams` que declaró, ya expandidos (ID-266).
    from_the_site: std::collections::BTreeMap<String, String>,
    /// Que el documento trae firmas que rFirma no sabe leer (ID-297).
    ///
    /// Se apunta porque **consentir el trámite es consentirlas**: la pregunta
    /// viaja dentro del momento del consentimiento y decir que no a ella es
    /// cancelar el trámite entero (ID-299, ID-301). Quien firma después de eso
    /// ya ha dicho que sí, y sin esta clave el puente abortaría la cofirma.
    unregistered_signatures: bool,
}

impl SiteConsent {
    /// Apunta lo que la sede pide del listado para identificarse.
    fn remember_identity(&self, filter: crate::protocol::SiteFilter) {
        *app::lock(&self.0) = Some(PendingConsent::Identity(filter));
    }

    /// Apunta lo que hace falta para firmar lo que la sede mandó.
    fn remember_signature(&self, pending: PendingSignature) {
        *app::lock(&self.0) = Some(PendingConsent::Signature(pending));
    }

    /// Lo que la sede pidió, si hay una identificación pendiente.
    fn what_the_site_asked(&self) -> Option<crate::protocol::SiteFilter> {
        match &*app::lock(&self.0) {
            Some(PendingConsent::Identity(filter)) => Some(filter.clone()),
            _ => None,
        }
    }

    /// Lo que hace falta para firmar, si hay una firma pendiente.
    fn the_signature_consented(&self) -> Option<PendingSignature> {
        match &*app::lock(&self.0) {
            Some(PendingConsent::Signature(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Se acabó el consentimiento: ni la ventana ni el canal tienen ya nada
    /// que contestar con esto.
    pub fn forget(&self) {
        *app::lock(&self.0) = None;
    }
}

/// **El puente prestado**, que es quien sabe filtrar y expandir políticas
/// (ID-252, ID-266).
///
/// Los dos motores del trámite corren en el hilo del isolate y por eso el
/// escritorio los recibe como puertos: aquí se cumplen contra [`Isolate`], que
/// es lo único que puede tocar `librfirma_crypto.so`.
struct TheBridge<'a> {
    isolate: &'a Isolate,
}

impl<'a> TheBridge<'a> {
    /// El puente que corre en ese isolate.
    fn borrowed_from(isolate: &'a Isolate) -> Self {
        Self { isolate }
    }

    /// Lo que devuelve el hilo del isolate, aplanado: el hilo que ya no está es
    /// el puente que no contesta, y para el trámite es la firma que no sale.
    fn ran<T: Send + 'static>(
        outcome: Result<Result<T, crate::ffi::BridgeError>, crate::isolate::IsolateGone>,
    ) -> Result<T, crate::ffi::BridgeError> {
        outcome.unwrap_or_else(|_| {
            Err(crate::ffi::BridgeError::Failed(
                "el hilo del isolate ya no esta".to_owned(),
            ))
        })
    }
}

impl app::filtering::FilterEngine for TheBridge<'_> {
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, crate::ffi::BridgeError> {
        let properties = filter_properties.to_owned();
        let certificates = certificates_b64.to_owned();
        Self::ran(self.isolate.run(move |bridge| {
            app::filtering::FilterEngine::select(bridge, &properties, &certificates)
        }))?
    }
}

impl app::policies::PolicyEngine for TheBridge<'_> {
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, crate::ffi::BridgeError> {
        let declared = extra_params.to_owned();
        let format = format.to_owned();
        Self::ran(
            self.isolate
                .run(move |bridge| app::policies::PolicyEngine::expand(bridge, &declared, &format)),
        )?
    }
}

/// **La operación de la sede, atendida con el escritorio armado desde el estado
/// de la aplicación** (ID-330).
///
/// No es una orden: la llama el canal, no la ventana. Lo que hace es lo que
/// hace una orden —desempaquetar el estado, llamar al caso de uso y traducir—,
/// y por eso el escritorio se arma **aquí** y no dentro de
/// [`crate::app::errand::attend_operation`].
///
/// Qué pasa después lo decide el [`crate::app::errand::ErrandStep`] (ID-331):
/// el momento del consentimiento se publica hacia la ventana y no escribe nada
/// en el cable; la operación que ya tiene respuesta ya la escribió el caso de
/// uso al cerrarse el trámite (ID-322).
pub fn attend_site_operation(
    app: &tauri::AppHandle,
    url: crate::protocol::AfirmaUrl,
    reply: crate::channel::ReplyHandle,
) {
    use tauri::Manager as _;

    let live = app.state::<app::errand::LiveErrand>();

    // Lo primero, antes de nada que pueda contestar: el asa es por donde sale
    // todo lo que este trámite le diga a la sede (ID-321).
    live.answer_through(reply);

    dispatch_the_site_operation(app, &url);
}

/// **Atiende la operación con el escritorio armado desde el estado**, y publica
/// lo que salga en la ventana de sede.
///
/// Aparte de [`attend_site_operation`] porque se atiende **dos veces la misma
/// petición** (ID-341): la primera cuando llega por el canal con su asa, y la
/// segunda cuando quien no tenía ningún certificado instala uno y vuelve a
/// mirar. El asa se apunta una sola vez, en la primera; lo de aquí no la toca.
fn dispatch_the_site_operation(app: &tauri::AppHandle, url: &crate::protocol::AfirmaUrl) {
    use tauri::Manager as _;

    let environment = app.state::<Environment>();
    let opened = app.state::<OpenedDocuments>();
    let live = app.state::<app::errand::LiveErrand>();
    let consent = app.state::<SiteConsent>();
    let isolate = app.state::<Isolate>();

    let bridge = TheBridge::borrowed_from(&isolate);
    let stores = environment.all_stores();
    // El documento que manda la sede se escribe en un fichero de paso que el
    // trámite borra al contestar (ID-286): de él no queda rastro, así que el
    // sitio es el de los ficheros que no se guardan.
    let scratch_dir = std::env::temp_dir();
    let desk = app::errand::ErrandDesk {
        engine: &bridge,
        policies: &bridge,
        stores: &stores,
        installed_dir: &environment.installed_certificates,
        listed: &environment.listed,
        opened: &opened,
        memory: &environment.memory,
        scratch_dir: &scratch_dir,
    };

    match app::errand::attend_operation(&desk, url, &live) {
        app::errand::ErrandStep::AskingForConsent {
            certificates,
            filter,
        } => {
            consent.remember_identity(filter);
            publish_to_the_site_window(app, SiteErrandView::asking_for_consent(certificates));
        }
        app::errand::ErrandStep::AskingToSign(asked) => {
            // La vista se compone antes de desguazar el consentimiento: lo que
            // la ventana enseña y lo que se queda para hacer cumplir lo que
            // pidió la sede son las dos mitades de lo mismo (ID-259, ID-266).
            let view = SiteErrandView::asking_to_sign(&asked);
            consent.remember_signature(PendingSignature {
                document: asked.document,
                filter: asked.filter,
                from_the_site: asked.from_the_site,
                unregistered_signatures: asked.unregistered_signatures,
            });
            publish_to_the_site_window(app, view);
        }
        // **El callejón que sí tiene arreglo** (ID-278, ID-341): la ventana lo
        // dice con su motivo, y con `NotOne` el trámite sigue vivo —no se ha
        // escrito nada en el cable— esperando a que se instale uno y se vuelva
        // a mirar.
        //
        // El consentimiento se olvida en las dos: aquí no hay nada que
        // consentir ni nada que elegir, así que lo que quedara apuntado de un
        // reparto anterior no vale ya para nada.
        app::errand::ErrandStep::NoCertificate { reason, owned, .. } => {
            consent.forget();
            publish_to_the_site_window(
                app,
                SiteErrandView::without_certificates(reason.into(), owned),
            );
        }
        // Ya está contestada: `attend_operation` cierra el trámite y escribe la
        // línea por el asa (ID-322). Lo que la ventana enseñe de eso es del
        // #394. Lo que sí toca aquí es olvidar el consentimiento apuntado: sin
        // trámite vivo no queda nada que consentir, y un `site_identify` que
        // llegara después listaría el token para no escribir en ninguna parte.
        app::errand::ErrandStep::Answering(_) => consent.forget(),
    }
}

/// Le publica el trámite a la ventana de sede, si sigue abierta.
///
/// Que no esté es una respuesta válida: sin ventana no hay a quien contarle
/// nada, y el trámite no depende de que la haya.
fn publish_to_the_site_window(app: &tauri::AppHandle, view: SiteErrandView) {
    use tauri::{Emitter as _, Manager as _};

    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.emit(SITE_ERRAND, view);
    }
}

/// La etiqueta de la ventana de sede (ID-333).
///
/// Es **suya y sólo suya**: la ventana principal es `main`, y las dos existen a
/// la vez sin que una tape a la otra.
pub const SITE_WINDOW: &str = "site";

/// El nombre del evento con el que la ventana de sede recibe el trámite
/// (ID-338).
///
/// Es un **evento y no un sondeo**: el trámite empuja cada momento nuevo. Que
/// no llegue nunca es la respuesta normal, porque la mayoría de los arranques
/// no vienen de una sede —y entonces esta ventana ni siquiera existe (ID-334)—.
pub const SITE_ERRAND: &str = "site-errand";

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
    use super::{
        pades_lower_left, what_the_repair_leaves, PendingSignature, PlacementOrder, SiteConsent,
    };
    use crate::commands::views::SignatureRoundView;
    use crate::protocol::SiteFilter;

    /// Lo mínimo que hace falta para tener una firma consentida.
    fn a_pending_signature() -> PendingSignature {
        PendingSignature {
            document: "00000000000000000000000000000000".to_owned(),
            filter: SiteFilter::default(),
            from_the_site: std::collections::BTreeMap::new(),
            unregistered_signatures: false,
        }
    }

    /// **La reparación no manda esperar sobre un canal que no existe**
    /// (ID-341).
    ///
    /// Al botón de instalar la CA local se llega desde `channelNotOpened`
    /// también, y desde ahí instalarla no reabre nada: el canal se abre una
    /// sola vez, en el arranque. Publicar `waiting` ahí tapaba la pantalla de
    /// reparación —la que lleva la dirección del ajuste del navegador— con
    /// «Conectando con la sede» durante treinta segundos, para volver después
    /// a la misma pantalla.
    #[test]
    fn the_repair_only_waits_when_a_channel_is_serving() {
        assert_eq!(
            serde_json::to_value(what_the_repair_leaves(true, false)).expect("el callejon cruza"),
            serde_json::json!({
                "origin": null,
                "stage": { "kind": "noChannel", "reason": "channelNotOpened" },
            })
        );
        assert_eq!(
            serde_json::to_value(what_the_repair_leaves(true, true)).expect("la espera cruza"),
            serde_json::json!({
                "origin": null,
                "stage": { "kind": "waiting" },
            })
        );
    }

    /// Y sin CA en ningún almacén la respuesta sigue siendo instalarla, haya
    /// canal o no: ningún navegador llega a intentar abrirlo (ID-329).
    #[test]
    fn the_repair_asks_for_the_local_ca_again_when_it_reached_no_store() {
        for serving in [false, true] {
            assert_eq!(
                serde_json::to_value(what_the_repair_leaves(false, serving))
                    .expect("el callejon cruza"),
                serde_json::json!({
                    "origin": null,
                    "stage": { "kind": "noChannel", "reason": "localCaMissing" },
                })
            );
        }
    }

    /// **ID-276**: los dos consentimientos no son intercambiables, y la
    /// asimetría es la que protege. Con una firma consentida no hay
    /// identificación que entregar: un `site_identify` que llegara ahí falla,
    /// que es lo correcto —la sede pidió firmar, no un certificado—.
    #[test]
    fn a_consented_signature_is_never_an_identity_to_hand_over() {
        let consent = SiteConsent::default();

        consent.remember_signature(a_pending_signature());

        assert!(
            consent.what_the_site_asked().is_none(),
            "una firma consentida no entrega identidad"
        );
        assert!(consent.the_signature_consented().is_some());
    }

    /// Y al revés: con una identificación consentida no hay nada que firmar.
    #[test]
    fn a_consented_identity_is_never_a_signature_to_begin() {
        let consent = SiteConsent::default();

        consent.remember_identity(SiteFilter::default());

        assert!(consent.the_signature_consented().is_none());
        assert!(consent.what_the_site_asked().is_some());
    }

    /// Y olvidar deja las dos preguntas sin respuesta: lo que se contestó una
    /// vez no se contesta dos (ID-275).
    #[test]
    fn forgetting_leaves_nothing_to_answer_with() {
        let consent = SiteConsent::default();
        consent.remember_signature(a_pending_signature());

        consent.forget();

        assert!(consent.the_signature_consented().is_none());
        assert!(consent.what_the_site_asked().is_none());
    }

    /// La ronda cruza con el nombre del verbo que la sede usó, y no con el de
    /// la variante del protocolo: es lo que la ventana enseña.
    #[test]
    fn the_round_crosses_named_as_the_site_asked_for_it() {
        assert_eq!(
            serde_json::to_value(SignatureRoundView::Sign).expect("la ronda cruza"),
            serde_json::json!("sign")
        );
        assert_eq!(
            serde_json::to_value(SignatureRoundView::Cosign).expect("la ronda cruza"),
            serde_json::json!("cosign")
        );
    }

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
