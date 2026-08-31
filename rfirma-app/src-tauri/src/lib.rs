//! rfirma: firma y cofirma de PDFs en PAdES con firma visible.
//!
//! De momento la ventana está vacía: lo único que hay debajo son
//! [`pkcs11`], la capa que habla con el token, [`rubric`], la normalización y
//! el almacén de la imagen de la firma, [`signing`], la configuración de firma
//! y el sello de sesión, [`memory`], lo que se recuerda entre sesiones, y
//! [`paths`], el único sitio que sabe qué sistema operativo hay debajo. Ni
//! FFI, ni PDF, ni orquestación de las tres fases: los aportan los sub-issues
//! siguientes de #46. Si te encuentras escribiendo Rust que sabe qué es un
//! PDF, te has salido de estos módulos.

pub mod memory;
pub mod paths;
pub mod pkcs11;
pub mod rubric;
pub mod signing;

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}
