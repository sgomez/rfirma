//! rfirma: firma y cofirma de PDFs en PAdES con firma visible.
//!
//! De momento la ventana está vacía: lo único que hay debajo son
//! [`pkcs11`], la capa que habla con el token, [`rubric`], la normalización y
//! el almacén de la imagen de la firma, [`signing`], la configuración de firma
//! y el sello de sesión, [`ffi`], la frontera con la librería nativa,
//! [`memory`], lo que se recuerda entre sesiones, [`destination`], por dónde
//! entra el documento y dónde cae el firmado, y [`paths`], el único sitio que
//! sabe qué sistema operativo hay debajo. La orquestación de las tres fases
//! —quién llama a la prefirma, quién firma con el token y quién postfirma—
//! todavía no está: la aporta un sub-issue siguiente de #46. Si te encuentras
//! escribiendo Rust que sabe qué es un PDF, te has salido de estos módulos.

pub mod commands;
pub mod destination;
pub mod ffi;
pub mod memory;
pub mod paths;
pub mod pkcs11;
pub mod rubric;
pub mod signing;

/// Dónde está el módulo PKCS#11 dentro del arenero.
///
/// Lo empaqueta el propio flatpak: los del anfitrión no cargan dentro
/// (`docs/research/flatpak-canal-unico.md`). Se puede apuntar a otro con
/// `RFIRMA_PKCS11_MODULE`, que es lo que hacen las pruebas de grada B y C
/// contra SoftHSM.
pub const PKCS11_MODULE_VARIABLE: &str = "RFIRMA_PKCS11_MODULE";

/// El módulo por omisión: el que instala el flatpak.
pub const DEFAULT_PKCS11_MODULE: &str = "/app/lib/pkcs11/opensc-pkcs11.so";

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    let paths = paths::Paths::from_environment().expect("debería saberse cuál es el HOME");
    let memory = memory::Memory::at(&paths);
    let configuration = memory
        .configuration()
        .map(memory::Loaded::into_value)
        .unwrap_or_default();
    let environment = commands::Environment {
        module: std::env::var_os(PKCS11_MODULE_VARIABLE)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_PKCS11_MODULE)),
        documents_folder: paths::documents_folder().unwrap_or_default(),
        configuration: std::sync::Mutex::new(configuration),
    };

    tauri::Builder::default()
        .manage(environment)
        // El hilo del isolate arranca con la ventana y **no abre la librería
        // todavía**: quien solo quiere mirar un PDF no paga el dlopen de 27,7
        // MB, y una librería que falta se cuenta como un error de firma y no
        // como una ventana que no abre.
        .manage(commands::Isolate::start())
        .manage(commands::SigningSession::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_certificates,
            commands::compose_visible_text,
            commands::begin_signing,
            commands::sign_with_pin,
            commands::finish_signing,
            commands::cancel_signing,
        ])
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}
