//! rfirma: firma y cofirma de PDFs en PAdES con firma visible.
//!
//! De momento la ventana está vacía: lo único que hay debajo son
//! [`pkcs11`], la capa que habla con el token, [`rubric`], la normalización y
//! el almacén de la imagen de la firma, [`signing`], la configuración de firma
//! y el sello de sesión, [`ffi`], la frontera con la librería nativa,
//! [`memory`], lo que se recuerda entre sesiones, [`destination`], por dónde
//! entra el documento y dónde cae el firmado, [`dropped`], qué se abre de lo
//! que se suelta en la ventana, y [`paths`], el único sitio que
//! sabe qué sistema operativo hay debajo. La orquestación de las tres fases
//! —quién llama a la prefirma, quién firma con el token y quién postfirma—
//! todavía no está: la aporta un sub-issue siguiente de #46. Si te encuentras
//! escribiendo Rust que sabe qué es un PDF, te has salido de estos módulos.

pub mod commands;
pub mod destination;
pub mod dropped;
pub mod ffi;
pub mod memory;
pub mod paths;
pub mod pkcs11;
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
    let environment = commands::Environment {
        // Los almacenes se resuelven **aquí**, en el binario, y no en la
        // receta que lo arranca: es lo que hace que `just dev` encuentre el
        // token del anfitrión sin exportar nada, y con el mismo código que
        // corre instalado (ID-13).
        stores: pkcs11::stores::from_environment(),
        documents_folder: paths::documents_folder().unwrap_or_default(),
        configuration: std::sync::Mutex::new(configuration),
        // La memoria viaja con el entorno y no aparte: las órdenes que guardan
        // ajustes tienen que actualizar la copia viva y el fichero a la vez, y
        // dos estados separados invitan a hacer solo una de las dos cosas.
        memory,
    };

    tauri::Builder::default()
        // El diálogo de fichero se abre desde Rust (ID-63), así que el
        // complemento entra aquí y no en el frontal: la lista de permisos de la
        // ventana no crece por esto.
        .plugin(tauri_plugin_dialog::init())
        .manage(environment)
        // El hilo del isolate arranca con la ventana y **no abre la librería
        // todavía**: quien solo quiere mirar un PDF no paga el dlopen de 27,7
        // MB, y una librería que falta se cuenta como un error de firma y no
        // como una ventana que no abre.
        .manage(commands::Isolate::start())
        .manage(commands::SigningSession::default())
        // Los documentos abiertos, del identificador opaco al documento del
        // portal (ID-61). Vive mientras vive el proceso.
        .manage(memory::OpenedDocuments::new())
        // El arrastre entra por aquí y no por un `onDrop` del JSX (ID-67): con
        // `dragDropEnabled` —que es lo que hay— el WebView **no** dispara los
        // eventos de arrastre de HTML, así que un manejador en el frontal no se
        // ejecutaría nunca y parecería un fallo de la interfaz. Lo que llega
        // aquí son rutas del anfitrión, y se quedan aquí: lo que cruza es el
        // documento ya apuntado (ADR-0011).
        .on_window_event(|window, event| {
            let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event
            else {
                return;
            };
            let opened = window.state::<memory::OpenedDocuments>();
            let Some(dropped) = commands::dropped_document(paths, &opened) else {
                return;
            };
            // Emitir puede fallar si la ventana se está cerrando, y entonces no
            // hay nadie a quien contarle nada: no es un motivo para tumbar la
            // aplicación mientras se va.
            let _ = window.emit(commands::DOCUMENT_DROPPED, dropped);
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_certificates,
            commands::compose_visible_text,
            commands::begin_signing,
            commands::sign_with_pin,
            commands::finish_signing,
            commands::cancel_signing,
            commands::open_document,
            commands::read_document,
            commands::read_configuration,
            commands::write_configuration,
            commands::forget_activity,
        ])
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}
