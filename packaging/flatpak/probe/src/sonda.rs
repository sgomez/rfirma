//! Sonda desechable del issue #22: carga la libreria nativa como la cargara
//! rfirma (dlopen por ruta relativa al ejecutable) y ejecuta el ciclo
//! trifasico completo con rubrica de imagen, firmando el PK1 con PKCS#11.

use base64::Engine;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

/// El fichero que el ADR-0004 exige. Era una lista de seis hasta el #36: al
/// excluir afirma-ui-utils (ADR-0012) los cinco auxiliares de AWT dejaron de
/// hacer falta, y tenerlos al lado convierte un error recuperable en un aborto.
pub const FICHEROS: [&str; 1] = ["librfirma_crypto.so"];

/// Ruta de la libreria: relativa al ejecutable, no una constante absoluta.
/// Sobrescribible con RFIRMA_LIB_DIR para desarrollar contra target/.
pub fn dir_libreria() -> PathBuf {
    if let Ok(d) = std::env::var("RFIRMA_LIB_DIR") {
        return PathBuf::from(d);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let d = exe.parent().unwrap().join("../lib/rfirma");
    d.canonicalize().unwrap_or(d)
}

pub fn faltan(dir: &Path) -> Vec<String> {
    FICHEROS.iter()
        .filter(|n| !dir.join(n).exists())
        .map(|n| n.to_string())
        .collect()
}

type CrearIsolate =
    unsafe extern "C" fn(*mut c_void, *mut *mut c_void, *mut *mut c_void) -> c_int;
type Presign = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
) -> *mut c_char;
type Postsign = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
) -> *mut c_char;
type Liberar = unsafe extern "C" fn(*mut c_void, *mut c_void);

pub struct Puente {
    lib: libloading::Library,
    hilo: *mut c_void,
    pub ruta: PathBuf,
}

impl Puente {
    pub fn abrir() -> Result<Puente, String> {
        let dir = dir_libreria();
        let ausentes = faltan(&dir);
        if !ausentes.is_empty() {
            return Err(format!(
                "faltan en {}: {}",
                dir.display(),
                ausentes.join(", ")
            ));
        }
        let ruta = dir.join("librfirma_crypto.so");
        let lib = unsafe { libloading::Library::new(&ruta) }
            .map_err(|e| format!("dlopen {}: {e}", ruta.display()))?;
        let mut isolate: *mut c_void = std::ptr::null_mut();
        let mut hilo: *mut c_void = std::ptr::null_mut();
        unsafe {
            let crear: libloading::Symbol<CrearIsolate> = lib
                .get(b"graal_create_isolate\0")
                .map_err(|e| format!("dlsym graal_create_isolate: {e}"))?;
            let rc = crear(std::ptr::null_mut(), &mut isolate, &mut hilo);
            if rc != 0 {
                return Err(format!("graal_create_isolate rc={rc}"));
            }
        }
        Ok(Puente { lib, hilo, ruta })
    }

    fn recoge(&self, p: *mut c_char, etiqueta: &str) -> Result<String, String> {
        if p.is_null() {
            return Err(format!("{etiqueta} devolvio NULL"));
        }
        let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
        unsafe {
            let liberar: libloading::Symbol<Liberar> = self.lib.get(b"rfirma_free_string\0").unwrap();
            liberar(self.hilo, p as *mut c_void);
        }
        if s.starts_with("ERROR:") {
            return Err(s);
        }
        Ok(s)
    }

    pub fn presign(&self, pdf_b64: &str, cert_b64: &str, extra: &str) -> Result<String, String> {
        let (a, b, c, d) = (
            CString::new(pdf_b64).unwrap(),
            CString::new("SHA256withRSA").unwrap(),
            CString::new(cert_b64).unwrap(),
            CString::new(extra).unwrap(),
        );
        let p = unsafe {
            let f: libloading::Symbol<Presign> = self
                .lib
                .get(b"rfirma_pades_presign\0")
                .map_err(|e| format!("dlsym presign: {e}"))?;
            f(self.hilo, a.as_ptr(), b.as_ptr(), c.as_ptr(), d.as_ptr())
        };
        self.recoge(p, "PRESIGN")
    }

    pub fn postsign(
        &self,
        pdf_b64: &str,
        cert_b64: &str,
        extra: &str,
        xml: &str,
    ) -> Result<Vec<u8>, String> {
        let (a, b, c, d, e) = (
            CString::new(pdf_b64).unwrap(),
            CString::new("SHA256withRSA").unwrap(),
            CString::new(cert_b64).unwrap(),
            CString::new(extra).unwrap(),
            CString::new(xml).unwrap(),
        );
        let p = unsafe {
            let f: libloading::Symbol<Postsign> = self
                .lib
                .get(b"rfirma_pades_postsign\0")
                .map_err(|err| format!("dlsym postsign: {err}"))?;
            f(
                self.hilo,
                a.as_ptr(),
                b.as_ptr(),
                c.as_ptr(),
                d.as_ptr(),
                e.as_ptr(),
            )
        };
        let b64 = self.recoge(p, "POSTSIGN")?;
        base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|e| format!("base64 del PDF: {e}"))
    }
}

/// Fase 2 del contrato trifasico: firma el bloque PRE con PKCS#11.
/// Mecanismo CKM_SHA256_RSA_PKCS sobre los bytes DER sin hashear (#8).
pub fn firma_pk1(
    modulo: &Path,
    pin: &str,
    etiqueta: &str,
    datos: &[u8],
) -> Result<Vec<u8>, String> {
    use cryptoki::context::{CInitializeArgs, Pkcs11};
    use cryptoki::mechanism::Mechanism;
    use cryptoki::object::{Attribute, AttributeType, ObjectClass};
    use cryptoki::session::UserType;
    use cryptoki::types::AuthPin;

    let ctx = Pkcs11::new(modulo).map_err(|e| format!("cargar {}: {e}", modulo.display()))?;
    ctx.initialize(CInitializeArgs::OsThreads)
        .map_err(|e| format!("C_Initialize: {e}"))?;
    let ranuras = ctx
        .get_slots_with_token()
        .map_err(|e| format!("get_slots_with_token: {e}"))?;
    for ranura in ranuras {
        let sesion = match ctx.open_ro_session(ranura) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if sesion
            .login(UserType::User, Some(&AuthPin::new(pin.into())))
            .is_err()
        {
            continue;
        }
        let plantilla = vec![
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(etiqueta.as_bytes().to_vec()),
        ];
        let claves = sesion
            .find_objects(&plantilla)
            .map_err(|e| format!("find_objects: {e}"))?;
        if let Some(clave) = claves.first() {
            let _ = sesion.get_attributes(*clave, &[AttributeType::Label]);
            return sesion
                .sign(&Mechanism::Sha256RsaPkcs, *clave, datos)
                .map_err(|e| format!("C_Sign: {e}"));
        }
    }
    Err(format!("no encuentro la clave privada '{etiqueta}' en ningun token"))
}

/// Sustituye/inserta el campo PK1 en el XML del TriphaseData.
pub fn inyecta_pk1(xml: &str, pk1_b64: &str) -> String {
    let marca = "<param n=\"PK1\">";
    if let Some(i) = xml.find(marca) {
        let resto = &xml[i + marca.len()..];
        let j = resto.find("</param>").unwrap();
        format!("{}{}{}", &xml[..i + marca.len()], pk1_b64, &resto[j..])
    } else {
        xml.replacen(
            "<param n=\"PRE\">",
            &format!("<param n=\"PK1\">{pk1_b64}</param>\n   <param n=\"PRE\">"),
            1,
        )
    }
}

pub fn extrae_pre(xml: &str) -> Result<Vec<u8>, String> {
    let marca = "<param n=\"PRE\">";
    let i = xml.find(marca).ok_or("no encuentro el campo PRE")?;
    let resto = &xml[i + marca.len()..];
    let j = resto.find("</param>").ok_or("PRE sin cierre")?;
    base64::engine::general_purpose::STANDARD
        .decode(&resto[..j])
        .map_err(|e| format!("base64 de PRE: {e}"))
}
