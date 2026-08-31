//! rfirma: firma y cofirma de PDFs en PAdES con firma visible.
//!
//! De momento la ventana está vacía: lo único que hay debajo es
//! [`pkcs11`], la capa que habla con el token. Ni FFI, ni PDF, ni orquestación
//! de las tres fases: los aportan los sub-issues siguientes de #46.

pub mod pkcs11;

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}
