//! rfirma: firma y cofirma de PDFs en PAdES con firma visible.
//!
//! El backend está en tres alturas, y se leen de arriba abajo:
//!
//! - [`commands`], las veintidós órdenes de Tauri. Desempaquetan el estado, llaman a
//!   un caso de uso y traducen el resultado. No deciden nada (ID-79).
//! - [`app`], los casos de uso: qué certificados hay, cómo se planifica y se
//!   entrega una firma, qué se recuerda entre sesiones. Es la interfaz por la
//!   que se prueba (ID-77).
//! - Los módulos de dominio e infraestructura, que no saben que las dos
//!   alturas de arriba existen (ID-81): [`pkcs11`], la capa que habla con el
//!   token; [`rubric`], la normalización y el almacén de la imagen de la firma;
//!   [`signing`], las reglas puras de la firma y el sello de sesión; [`ffi`] y
//!   [`isolate`], la frontera con la librería nativa y el hilo que la posee;
//!   [`memory`], lo que se recuerda entre sesiones; [`destination`], por dónde
//!   entra el documento y dónde cae el firmado; [`dropped`], qué se abre de lo
//!   que se suelta en la ventana; y [`paths`], el único sitio que sabe qué
//!   sistema operativo hay debajo.
//!
//! Si te encuentras escribiendo Rust que sabe qué es un PDF, te has salido de
//! estos módulos.

pub mod app;
pub mod commands;
pub mod destination;
pub mod dropped;
pub mod ffi;
pub mod isolate;
pub mod memory;
pub mod paths;
pub mod pkcs11;
pub mod releases;
pub mod rubric;
pub mod signing;

/// La escotilla para apuntar a **otro** módulo PKCS#11.
///
/// Cuando está puesta manda ella sola y se ignoran los candidatos de
/// [`pkcs11::stores`]: quien la exporta quiere ese módulo. Es de lo que
/// dependen las pruebas de grada B y C contra SoftHSM.
pub const PKCS11_MODULE_VARIABLE: &str = "RFIRMA_PKCS11_MODULE";

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    use tauri::{Emitter, Manager};

    let paths = paths::Paths::from_environment().expect("debería saberse cuál es el HOME");
    let memory = memory::Memory::at(&paths);
    let configuration = memory
        .configuration()
        .map(memory::Loaded::into_value)
        .unwrap_or_default();
    let environment = app::Environment {
        // Los almacenes se resuelven **aquí**, en el binario, y no en la
        // receta que lo arranca: es lo que hace que `just dev` encuentre el
        // token del anfitrión sin exportar nada, y con el mismo código que
        // corre instalado (ID-13).
        stores: pkcs11::stores::from_environment(),
        // El último listado de certificados, vacío hasta la primera búsqueda.
        // Es donde se quedan las referencias que la ventana no puede tener.
        listed: memory::ListedCertificates::new(),
        documents_folder: paths::documents_folder().unwrap_or_default(),
        configuration: std::sync::Mutex::new(configuration),
        // La memoria viaja con el entorno y no aparte: las órdenes que guardan
        // ajustes tienen que actualizar la copia viva y el fichero a la vez, y
        // dos estados separados invitan a hacer solo una de las dos cosas.
        memory,
        // Se copia, no se referencia (ID-33): el almacén vive en una ruta fija
        // del directorio de datos, resuelta aquí una sola vez.
        rubric: rubric::RubricStore::at(paths.rubric_path()),
        // Los `.p12` instalados viven en el directorio de datos y se releen en
        // cada listado, no aquí: instalar y quitar son gestos de esta misma
        // sesión (ID-192).
        installed_certificates: paths.installed_certificates_dir(),
    };

    tauri::Builder::default()
        // **Instancia única, con sustitución y sin preguntar** (ID-160). Dos
        // procesos escribiendo la misma memoria acaban con el último en cerrar
        // pisando al otro, y el `.deb` quita el aislamiento que hoy lo tapa a
        // medias. Va **el primero** de los complementos: es lo que exige el
        // suyo para que la segunda instancia se muera antes de montar nada.
        //
        // La excepción del ID-160 se decide en `app::invocation`, no aquí: con
        // una sesión de firma viva no se sustituye nada, porque es el único
        // estado donde perder el hilo cuesta un PIN.
        .plugin(tauri_plugin_single_instance::init(
            |app, command_line, folder| {
                use tauri::Manager as _;
                let invocation = app::invocation::Invocation {
                    command_line,
                    folder: std::path::PathBuf::from(folder),
                };
                // Sin ventana no hay a quién entregarle nada, y anotar el
                // documento en `OpenedDocuments` antes de saberlo dejaría una
                // entrada que no recoge nadie: primero la ventana.
                let Some(window) = app.get_webview_window("main") else {
                    return;
                };
                let session = app.state::<commands::SigningSession>();
                let opened = app.state::<memory::OpenedDocuments>();
                let substitution = commands::second_invocation(
                    &invocation,
                    &opened,
                    app::signing::is_live(&session),
                );
                // Traer la ventana al frente ocurre siempre, también con la firma a
                // medias: quien invoca quiere ver la aplicación, y enseñarle el PIN
                // que dejó a medias es la respuesta correcta.
                let _ = window.set_focus();
                if let Some(substitution) = substitution {
                    // Por la **misma** puerta que el arrastre: el estado en que
                    // queda la ventana es el mismo, no uno parecido (ID-159).
                    let _ = window.emit(commands::DOCUMENT_DROPPED, substitution);
                }
            },
        ))
        // El diálogo de fichero se abre desde Rust (ID-63), así que el
        // complemento entra aquí y no en el frontal: la lista de permisos de la
        // ventana no crece por esto.
        .plugin(tauri_plugin_dialog::init())
        // Y el que abre el PDF firmado y su carpeta, por la misma puerta: bajo
        // el sandbox es el portal `OpenURI`, y es lo único que lleva al usuario
        // hasta un fichero cuya ruta nunca ve (ID-79, ID-85, ADR-0011).
        .plugin(tauri_plugin_opener::init())
        .manage(environment)
        // El hilo del isolate arranca con la ventana y **no abre la librería
        // todavía**: quien solo quiere mirar un PDF no paga el dlopen de 27,7
        // MB, y una librería que falta se cuenta como un error de firma y no
        // como una ventana que no abre.
        .manage(isolate::Isolate::start())
        .manage(commands::SigningSession::default())
        // Lo que traía la línea de órdenes que abrió esta ventana, hasta que la
        // ventana lo recoja con `read_invocation` (ID-157). Se lee aquí, en el
        // arranque, porque es el único momento en que los argumentos son los de
        // **esta** invocación.
        .manage(commands::PendingInvocation::of(
            app::invocation::Invocation::of_this_process(),
        ))
        // Los documentos abiertos, del identificador opaco al documento del
        // portal (ID-61). Vive mientras vive el proceso.
        .manage(memory::OpenedDocuments::new())
        // El arrastre entra por aquí y no por un `onDrop` del JSX (ID-67): con
        // `dragDropEnabled` —que es lo que hay— el WebView **no** dispara los
        // eventos de arrastre de HTML, así que un manejador en el frontal no se
        // ejecutaría nunca y parecería un fallo de la interfaz. Lo que llega
        // aquí son rutas del anfitrión, y se quedan aquí: lo que cruza es el
        // documento ya apuntado (ADR-0011).
        // El tamaño de la ventana se aplica al arrancar (ID-72) leyendo lo
        // recordado, o el de por omisión si no hay nada. No es ruido: es el
        // mismo tamaño que ya trae `tauri.conf.json` cuando no hay nada
        // guardado (`app::window::default_window`).
        .setup(|app| {
            let environment = app.state::<app::Environment>();
            let remembered = app::window::initial_window(&environment.memory);
            if let Some(webview) = app.get_webview_window("main") {
                // El tamaño restaurado se aplica siempre, y maximizar va
                // después: si se maximizara primero, el tamaño sin maximizar
                // de la ventana se quedaría en el de `tauri.conf.json` y el
                // primer «desmaximizar» de la sesión lo pisaría con ese valor
                // de fábrica en vez de con el recordado.
                let _ =
                    webview.set_size(tauri::LogicalSize::new(remembered.width, remembered.height));
                if remembered.maximized {
                    let _ = webview.maximize();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                let opened = window.state::<memory::OpenedDocuments>();
                let Some(dropped) = commands::dropped_document(paths, &opened) else {
                    return;
                };
                // Emitir puede fallar si la ventana se está cerrando, y
                // entonces no hay nadie a quien contarle nada: no es un motivo
                // para tumbar la aplicación mientras se va.
                let _ = window.emit(commands::DOCUMENT_DROPPED, dropped);
            }
            // El tamaño se recuerda en cada cambio y no solo al cerrar: así
            // sobrevive también a un cierre que no dispara `Resized` (matar el
            // proceso, un cuelgue). Maximizada, no se lee el tamaño físico
            // —sería el de la pantalla entera— y se conserva el restaurado que
            // ya hubiera (ID-73, `app::window::resized`).
            tauri::WindowEvent::Resized(_) => {
                let maximized = window.is_maximized().unwrap_or(false);
                let logical_size = (!maximized)
                    .then(|| window.inner_size().ok())
                    .flatten()
                    .map(|size| {
                        let scale = window.scale_factor().unwrap_or(1.0);
                        let logical = size.to_logical::<f64>(scale);
                        (logical.width, logical.height)
                    })
                    // Un tamaño degenerado (0×0, lo que un `Resized` de
                    // minimizar podría emitir en algún canal) no se persiste:
                    // el próximo arranque lo aplicaría tal cual y abriría una
                    // ventana inservible sin forma de salir de ahí sin borrar
                    // `state.json` a mano.
                    .filter(|(width, height)| *width > 0.0 && *height > 0.0);
                let environment = window.state::<app::Environment>();
                app::window::resized(&environment.memory, maximized, logical_size);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_certificates,
            commands::begin_signing,
            commands::sign_with_pin,
            commands::finish_signing,
            commands::cancel_signing,
            commands::open_document,
            commands::read_document,
            commands::read_configuration,
            commands::write_configuration,
            commands::forget_activity,
            commands::list_recents,
            commands::record_recent,
            commands::forget_recent,
            commands::choose_rubric,
            commands::read_rubric,
            commands::preview_destination,
            commands::choose_destination,
            commands::open_signed_document,
            commands::open_signed_folder,
            commands::preview_signature,
            commands::pades_lower_left,
            commands::read_invocation,
            commands::check_for_new_version,
            commands::install_certificate,
            commands::remove_certificate,
        ])
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}
