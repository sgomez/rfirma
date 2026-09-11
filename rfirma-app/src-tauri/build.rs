// ADR-0013: aquí no se llama ni a Maven ni a `native-image`. Un `cargo build`
// que dispare por sorpresa 1 m 22 s de `native-image` arruina el bucle de
// realimentación que el issue #11 decidió proteger. `just native` construye la
// librería; `just dev` solo comprueba que está.
fn main() {
    tauri_build::build()
}
