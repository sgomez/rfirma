// Sin consola en Windows en release. rfirma se distribuye solo en flatpak
// (ADR-0015), pero el atributo no cuesta nada y evita la sorpresa si algún día
// deja de ser así.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rfirma_lib::run()
}
