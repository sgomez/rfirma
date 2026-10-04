//! El binario de consola de Windows, que el instalador llama `rfirma.com`: delega en `rfirma_lib::run_in_the_console` y no tiene nada más dentro (ADR-0041).

fn main() {
    std::process::exit(rfirma_lib::run_in_the_console())
}
