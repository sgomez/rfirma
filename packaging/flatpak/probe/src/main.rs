//! Sonda del issue #22. Dos modos:
//!   rfirma-probe entorno            -> informe del arenero
//!   rfirma-probe ciclo <pdf> <out>  -> ciclo trifasico completo, sin GUI
//!   rfirma-probe                    -> ventana Tauri (WebKitGTK + portales)

mod sonda;

use base64::Engine;
use std::path::{Path, PathBuf};

fn datos() -> PathBuf {
    std::env::var("RFIRMA_PROBE_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/app/share/rfirma-probe"))
}

fn manda(cmd: &str, args: &[&str]) -> String {
    match std::process::Command::new(cmd).args(args).output() {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                String::from_utf8_lossy(&o.stderr).trim().to_string()
            } else {
                s
            }
        }
        Err(e) => format!("<no ejecutable: {e}>"),
    }
}

fn glibc() -> String {
    unsafe {
        match libloading::os::unix::Library::this()
            .get::<unsafe extern "C" fn() -> *const std::os::raw::c_char>(b"gnu_get_libc_version\0")
        {
            Ok(f) => std::ffi::CStr::from_ptr(f()).to_string_lossy().into_owned(),
            Err(_) => "?".into(),
        }
    }
}

fn mapa(patron: &str) -> Vec<String> {
    std::fs::read_to_string("/proc/self/maps")
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|p| p.contains(patron))
        .map(|s| s.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn informe_entorno() -> String {
    let mut s = String::new();
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = sonda::dir_libreria();
    s += &format!("ejecutable      : {}\n", exe.display());
    s += &format!("dir libreria    : {}\n", dir.display());
    let ausentes = sonda::faltan(&dir);
    s += &format!(
        "los seis .so    : {}\n",
        if ausentes.is_empty() {
            "los seis presentes".to_string()
        } else {
            format!("FALTAN: {}", ausentes.join(", "))
        }
    );
    s += &format!("glibc           : {}\n", glibc());
    let arenero = Path::new("/.flatpak-info").exists();
    s += &format!("arenero flatpak : {}\n", if arenero { "si" } else { "no" });
    if arenero {
        let info = std::fs::read_to_string("/.flatpak-info").unwrap_or_default();
        for l in info.lines() {
            if l.starts_with("filesystems=")
                || l.starts_with("sockets=")
                || l.starts_with("devices=")
                || l.starts_with("runtime=")
                || l.starts_with("name=")
            {
                s += &format!("  {l}\n");
            }
        }
    }
    s += &format!("TZ              : {:?}\n", std::env::var("TZ").ok());
    s += &format!(
        "/etc/localtime  : {}\n",
        std::fs::read_link("/etc/localtime")
            .map(|p| p.display().to_string())
            .unwrap_or_else(|e| format!("<{e}>"))
    );
    s += &format!(
        "zoneinfo Madrid : {}\n",
        Path::new("/usr/share/zoneinfo/Europe/Madrid").exists()
    );
    for m in ["libwebkit2gtk", "libjavascriptcore", "librfirma_crypto", "libawt"] {
        let v = mapa(m);
        s += &format!("mapa {m:<18}: {}\n", if v.is_empty() { "<no cargada>".into() } else { v.join(" ") });
    }
    s += &format!("p11-kit-client  : {}\n", Path::new("/usr/lib/x86_64-linux-gnu/pkcs11/p11-kit-client.so").exists());
    let modulo = std::env::var("RFIRMA_P11_MODULE").unwrap_or_default();
    s += &format!("modulo PKCS#11  : {modulo} (existe: {})\n", Path::new(&modulo).exists());
    s += &format!(
        "portal FileChooser: {}\n",
        manda(
            "gdbus",
            &[
                "call", "--session", "--dest", "org.freedesktop.portal.Desktop",
                "--object-path", "/org/freedesktop/portal/desktop",
                "--method", "org.freedesktop.DBus.Properties.Get",
                "org.freedesktop.portal.FileChooser", "version"
            ]
        )
    );
    s += &format!(
        "portal Documents  : {}\n",
        manda(
            "gdbus",
            &[
                "call", "--session", "--dest", "org.freedesktop.portal.Documents",
                "--object-path", "/org/freedesktop/portal/documents",
                "--method", "org.freedesktop.portal.Documents.GetMountPoint"
            ]
        )
    );
    s
}

/// Ciclo trifasico completo con rubrica de imagen, firmando con PKCS#11.
fn ciclo(pdf: &Path, salida: &Path) -> Result<String, String> {
    let mut s = String::new();
    let d = datos();
    let fichero_cert = std::env::var("RFIRMA_CERT").unwrap_or_else(|_| "cert.b64".into());
    let cert = std::fs::read_to_string(d.join(&fichero_cert))
        .map_err(|e| format!("{fichero_cert}: {e}"))?
        .trim()
        .to_string();
    let extra = std::fs::read_to_string(d.join("visible-imagen.properties"))
        .map_err(|e| format!("extraParams: {e}"))?;
    let bytes = std::fs::read(pdf).map_err(|e| format!("leer {}: {e}", pdf.display()))?;
    s += &format!("certificado     : {fichero_cert}\n");
    s += &format!("PDF de entrada  : {} ({} bytes)\n", pdf.display(), bytes.len());
    let pdf_b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let puente = sonda::Puente::abrir()?;
    s += &format!("dlopen          : OK ({})\n", puente.ruta.display());

    let xml = puente.presign(&pdf_b64, &cert, &extra)?;
    s += &format!("prefirma        : OK ({} bytes de TriphaseData)\n", xml.len());

    let pre = sonda::extrae_pre(&xml)?;
    let modulo = std::env::var("RFIRMA_P11_MODULE")
        .map_err(|_| "falta RFIRMA_P11_MODULE".to_string())?;
    let pin = std::env::var("RFIRMA_P11_PIN").unwrap_or_else(|_| "1234".into());
    let etiqueta = std::env::var("RFIRMA_P11_LABEL")
        .unwrap_or_else(|_| "FNMT-ACTIVO-99999999R".into());
    let pk1 = sonda::firma_pk1(Path::new(&modulo), &pin, &etiqueta, &pre)?;
    s += &format!(
        "firma PKCS#11   : OK (PRE {} bytes DER -> PK1 {} bytes)\n",
        pre.len(),
        pk1.len()
    );

    let xml2 = sonda::inyecta_pk1(&xml, &base64::engine::general_purpose::STANDARD.encode(&pk1));
    let firmado = puente.postsign(&pdf_b64, &cert, &extra, &xml2)?;
    std::fs::write(salida, &firmado).map_err(|e| format!("escribir {}: {e}", salida.display()))?;
    s += &format!(
        "postfirma       : OK ({} bytes) -> {}\n",
        firmado.len(),
        salida.display()
    );
    Ok(s)
}

// ---------------------------------------------------------------- GUI

#[tauri::command]
fn entorno() -> String {
    informe_entorno()
}

#[tauri::command]
fn arrancado(ua: String, ancho: u32, alto: u32) {
    println!("WEBVIEW OK  {ancho}x{alto}  userAgent: {ua}");
    for m in ["libwebkit2gtk", "libjavascriptcore"] {
        println!("  cargada: {}", mapa(m).join(" "));
    }
}

#[tauri::command]
fn inspecciona(ruta: String) -> String {
    let p = Path::new(&ruta);
    match std::fs::metadata(p) {
        Ok(m) => format!(
            "legible: si, {} bytes; padre: {}",
            m.len(),
            p.parent().map(|x| x.display().to_string()).unwrap_or_default()
        ),
        Err(e) => format!("NO legible: {e}"),
    }
}

#[derive(serde::Serialize)]
struct Resultado {
    informe: String,
    salida: String,
}

#[tauri::command]
fn firma(ruta: String) -> Result<Resultado, String> {
    let salida = std::env::temp_dir().join("sonda-firmado.pdf");
    let informe = ciclo(Path::new(&ruta), &salida)?;
    Ok(Resultado {
        informe,
        salida: salida.display().to_string(),
    })
}

#[tauri::command]
fn copia(origen: String, destino: String) -> String {
    match std::fs::copy(&origen, &destino) {
        Ok(n) => format!("escritos {n} bytes en {destino}"),
        Err(e) => format!("ERROR al escribir en {destino}: {e}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("entorno") => print!("{}", informe_entorno()),
        Some("ciclo") => {
            let pdf = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| {
                datos().join("test.pdf").display().to_string()
            }));
            let out = PathBuf::from(
                args.get(3)
                    .cloned()
                    .unwrap_or_else(|| "/tmp/sonda-firmado.pdf".into()),
            );
            print!("{}", informe_entorno());
            match ciclo(&pdf, &out) {
                Ok(s) => print!("{s}"),
                Err(e) => {
                    eprintln!("FALLO: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            tauri::Builder::default()
                .plugin(tauri_plugin_dialog::init())
                .setup(|app| {
                    // Con RFIRMA_PROBE_ABRIR=1 dispara el portal FileChooser
                    // nada mas arrancar, sin esperar a que nadie pulse.
                    if std::env::var("RFIRMA_PROBE_ABRIR").is_ok() {
                        use tauri_plugin_dialog::DialogExt;
                        app.dialog().file().pick_file(|r| {
                            println!("PORTAL FileChooser devolvio: {r:?}");
                        });
                        println!("PORTAL FileChooser: peticion lanzada");
                    }
                    Ok(())
                })
                .invoke_handler(tauri::generate_handler![entorno, arrancado, inspecciona, firma, copia])
                .run(tauri::generate_context!())
                .expect("arrancar tauri");
        }
    }
}
