// Sin consola en Windows en release. rfirma se distribuye hoy solo en Linux
// —flatpak, `.deb` y `.rpm` (ADR-0004, ADR-0015)—, pero el atributo no cuesta
// nada y evita la sorpresa el día que haya un `.msi`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rfirma_lib::run()
}
