//! Andamiaje de rfirma (#47): arranca una ventana vacía y nada más.
//!
//! Aquí no hay lógica de firma, ni FFI, ni PKCS#11: los aportan los sub-issues
//! siguientes de #46. Si te encuentras escribiendo Rust que sabe qué es un PDF,
//! te has salido de este módulo.

/// Punto de entrada compartido por el binario y por las pruebas.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error arrancando la ventana de rfirma");
}
