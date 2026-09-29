// ADR-0013: aquí no se llama ni a Maven ni a `native-image`. Un `cargo build`
// que dispare por sorpresa 1 m 22 s de `native-image` arruina el bucle de
// realimentación que el issue #11 decidió proteger. `just native` construye la
// librería; `just dev` solo comprueba que está.
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        tauri_build::build();
        return;
    }
    // ADR-0035: el manifiesto va también a las pruebas, que sin él no arrancan.
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build debería preparar la aplicación");
}
