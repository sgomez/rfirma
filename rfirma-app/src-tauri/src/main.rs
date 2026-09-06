// Sin consola en Windows en release (ADR-0004, ADR-0015).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rfirma_lib::run()
}
