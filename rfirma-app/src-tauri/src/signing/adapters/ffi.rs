//! Frontera FFI con `librfirma_crypto.so` compilada con GraalVM Native Image (ADR-0003, ADR-0004).

use std::ffi::{CStr, CString, OsString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use base64::Engine;

use crate::signing::domain::SessionSeal;

pub use crate::signing::domain::bridge::{
    BridgeError, Candidate, ExpandRequest, FilterRequest, LibraryNotFound, Origin, PostSignRequest,
    PreSignRequest, PreSignature, LIBRARY_DIRECTORY_VARIABLE, LIBRARY_FILE,
};

const RELATIVE_LIBRARY_DIRECTORY: &str = "../lib/rfirma";

/// Directorios candidatos donde buscar la librería nativa en orden de prioridad.
pub fn candidates(
    environment: &dyn Fn(&str) -> Option<OsString>,
    executable_directory: &Path,
) -> Vec<Candidate> {
    let mut found = Vec::with_capacity(2);
    if let Some(value) = environment(LIBRARY_DIRECTORY_VARIABLE).filter(|value| !value.is_empty()) {
        found.push(Candidate {
            directory: PathBuf::from(value),
            origin: Origin::Override,
        });
    }
    found.push(Candidate {
        directory: normalise(executable_directory.join(RELATIVE_LIBRARY_DIRECTORY)),
        origin: Origin::RelativeToExecutable,
    });
    found
}

fn normalise(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// Localiza el fichero de la librería nativa en los directorios candidatos.
pub fn locate(
    environment: &dyn Fn(&str) -> Option<OsString>,
    executable_directory: &Path,
) -> Result<PathBuf, LibraryNotFound> {
    let looked_at = candidates(environment, executable_directory);
    looked_at
        .iter()
        .map(Candidate::library_path)
        .find(|path| path.is_file())
        .ok_or(LibraryNotFound { looked_at })
}

/// Quien sabe liberar una cadena del puente.
/// Capacidad de liberar una cadena asignada por el puente (ADR-0003).
pub trait FreeBridgeString {
    /// Libera un puntero que devolvió el puente.
    ///
    /// # Safety
    ///
    /// `pointer` tiene que venir de este mismo puente y no haberse liberado antes.
    unsafe fn free(&self, pointer: *mut c_char);
}

/// Cadena asignada por el puente cuya memoria gestiona Rust (ADR-0003).
pub struct BridgeString<D: FreeBridgeString> {
    pointer: *mut c_char,
    deallocator: D,
}

impl<D: FreeBridgeString> BridgeString<D> {
    /// Adopta el puntero devuelto por el puente.
    ///
    /// # Safety
    ///
    /// `pointer` debe ser nulo o una cadena C válida reservada por el puente.
    pub unsafe fn adopt(pointer: *mut c_char, deallocator: D) -> Result<Self, BridgeError> {
        if pointer.is_null() {
            return Err(BridgeError::NullResponse);
        }
        Ok(Self {
            pointer,
            deallocator,
        })
    }

    /// Copia el contenido a un `String` de Rust.
    pub fn to_utf8_lossy(&self) -> String {
        unsafe { CStr::from_ptr(self.pointer) }
            .to_string_lossy()
            .into_owned()
    }
}

impl<D: FreeBridgeString> Drop for BridgeString<D> {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            unsafe { self.deallocator.free(self.pointer) };
            self.pointer = std::ptr::null_mut();
        }
    }
}

type CreateIsolate = unsafe extern "C" fn(*mut c_void, *mut *mut c_void, *mut *mut c_void) -> c_int;
type TearDownIsolate = unsafe extern "C" fn(*mut c_void) -> c_int;
type FreeStringSymbol = unsafe extern "C" fn(*mut c_void, *mut c_void);
type PreSignSymbol = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
) -> *mut c_char;
type PostSignSymbol = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
) -> *mut c_char;

type FilterSymbol = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> *mut c_char;

type ExpandSymbol = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> *mut c_char;

/// La librería nativa cargada, con su isolate de GraalVM ya creado.
///
/// **No es `Sync`, y es a propósito**: el `IsolateThread` de GraalVM pertenece
/// al hilo que lo creó. Compartir este valor entre hilos sería el segundo fallo
/// silencioso de esta frontera, así que el tipo no deja.
pub struct NativeBridge {
    /// Nadie la lee, y aun así tiene que estar: es lo que mantiene cargada la
    /// librería —y por tanto válidos los seis punteros a función de abajo—
    /// mientras el puente viva. Soltarla antes sería un `dlclose` con el
    /// isolate dentro.
    #[expect(dead_code, reason = "mantiene viva la librería de los punteros")]
    library: libloading::Library,
    isolate: *mut c_void,
    thread: *mut c_void,
    path: PathBuf,
    presign: PreSignSymbol,
    postsign: PostSignSymbol,
    filter: FilterSymbol,
    expand: ExpandSymbol,
    free_string: FreeStringSymbol,
    tear_down: TearDownIsolate,
}

/// Cómo libera las cadenas un puente ya cargado: llamando a
/// `autofirma_free_string` con su propio `IsolateThread`.
struct BridgeDeallocator<'a> {
    bridge: &'a NativeBridge,
}

impl FreeBridgeString for BridgeDeallocator<'_> {
    unsafe fn free(&self, pointer: *mut c_char) {
        // SAFETY: el puntero lo acaba de devolver este mismo puente, que sigue
        // cargado porque el préstamo lo mantiene vivo.
        unsafe {
            (self.bridge.free_string)(self.bridge.thread, pointer.cast());
        }
    }
}

impl NativeBridge {
    /// Carga la librería mirando en los dos sitios de [`candidates`].
    pub fn open() -> Result<Self, BridgeError> {
        let executable = std::env::current_exe()
            .map_err(|error| BridgeError::ExecutablePathUnknown(error.to_string()))?;
        let directory = executable.parent().unwrap_or(Path::new(".")).to_path_buf();
        let path = locate(&|name| std::env::var_os(name), &directory)?;
        Self::open_at(&path)
    }

    /// Carga la librería de una ruta concreta. La usan las pruebas de grada C y
    /// [`NativeBridge::open`] cuando ya sabe cuál de los dos candidatos hay.
    pub fn open_at(path: &Path) -> Result<Self, BridgeError> {
        // SAFETY: `dlopen` ejecuta los inicializadores de la librería. Es la
        // librería del ADR-0004, construida por `just native` desde este mismo
        // repositorio; no se abre nada que venga de fuera.
        let library =
            unsafe { libloading::Library::new(path) }.map_err(|error| BridgeError::Load {
                path: path.to_path_buf(),
                detail: error.to_string(),
            })?;
        // Los símbolos se resuelven **antes** de crear el isolate, y el orden
        // importa: si faltara alguno después de `graal_create_isolate`, el `?`
        // saldría con el isolate vivo y sin `NativeBridge` que lo desmontara en
        // `Drop`, y además soltaría `library` —un `dlclose` con un isolate
        // dentro—. Resolviendo primero, ese camino no existe.
        //
        // SAFETY: las cuatro firmas son las que declaran `NativeBridge.java` y
        // el contrato de GraalVM para esas entradas, y los punteros a función
        // valen mientras `library` siga cargada, que es lo que garantiza
        // guardarla en el mismo valor.
        let (create, presign, postsign, filter, expand, free_string, tear_down) = unsafe {
            (
                resolve::<CreateIsolate>(&library, b"graal_create_isolate\0")?,
                resolve::<PreSignSymbol>(&library, b"autofirma_pades_presign\0")?,
                resolve::<PostSignSymbol>(&library, b"autofirma_pades_postsign\0")?,
                resolve::<FilterSymbol>(&library, b"autofirma_filter_certificates\0")?,
                resolve::<ExpandSymbol>(&library, b"autofirma_expand_extra_params\0")?,
                resolve::<FreeStringSymbol>(&library, b"autofirma_free_string\0")?,
                resolve::<TearDownIsolate>(&library, b"graal_tear_down_isolate\0")?,
            )
        };
        let mut isolate: *mut c_void = std::ptr::null_mut();
        let mut thread: *mut c_void = std::ptr::null_mut();
        // SAFETY: la firma de `graal_create_isolate` es la del contrato de
        // GraalVM y los dos punteros de salida son locales válidos.
        let code = unsafe { create(std::ptr::null_mut(), &mut isolate, &mut thread) };
        if code != 0 {
            return Err(BridgeError::IsolateFailed(code));
        }
        Ok(Self {
            library,
            isolate,
            thread,
            path: path.to_path_buf(),
            presign,
            postsign,
            filter,
            expand,
            free_string,
            tear_down,
        })
    }

    /// El fichero que se cargó.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Prefirma PAdES: devuelve el `TriphaseData`, los bytes a firmar y el sello de sesión.
    pub fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError> {
        let pdf = c_string(request.pdf_b64, "el PDF")?;
        let algorithm = c_string(request.algorithm, "el algoritmo")?;
        let chain = c_string(request.certificate_chain_b64, "la cadena de certificados")?;
        let extra = c_string(request.extra_params, "los extraParams")?;
        let json = self.call(|thread| unsafe {
            (self.presign)(
                thread,
                pdf.as_ptr(),
                algorithm.as_ptr(),
                chain.as_ptr(),
                extra.as_ptr(),
            )
        })?;
        parse_presign(&json)
    }

    /// Postfirma PAdES: devuelve los bytes del PDF firmado.
    pub fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError> {
        let pdf = c_string(request.pdf_b64, "el PDF")?;
        let chain = c_string(request.certificate_chain_b64, "la cadena de certificados")?;
        let stamp = c_string(request.sealed.stamp().as_bridge_payload(), "el sello")?;
        let session = c_string(request.sealed.session(), "la sesión")?;
        let pkcs1 = c_string(request.sealed.pkcs1_b64(), "el PKCS#1")?;
        let json = self.call(|thread| unsafe {
            (self.postsign)(
                thread,
                pdf.as_ptr(),
                chain.as_ptr(),
                stamp.as_ptr(),
                session.as_ptr(),
                pkcs1.as_ptr(),
            )
        })?;
        parse_postsign(&json)
    }

    /// Acota un listado de certificados con la expresión de filtro de la sede.
    pub fn filter_certificates(
        &self,
        request: FilterRequest<'_>,
    ) -> Result<Vec<usize>, BridgeError> {
        let properties = c_string(request.filter_properties, "la expresion de filtro")?;
        let certificates = c_string(request.certificates_b64, "los certificados")?;
        let json = self.call(|thread| unsafe {
            (self.filter)(thread, properties.as_ptr(), certificates.as_ptr())
        })?;
        parse_filter_selection(&json)
    }

    /// Expande la política de firma que declara la sede.
    pub fn expand_extra_params(&self, request: ExpandRequest<'_>) -> Result<String, BridgeError> {
        let params = c_string(request.extra_params, "los extraParams")?;
        let format = c_string(request.format, "el formato")?;
        let json =
            self.call(|thread| unsafe { (self.expand)(thread, params.as_ptr(), format.as_ptr()) })?;
        parse_expanded_params(&json)
    }

    fn call<F>(&self, invoke: F) -> Result<String, BridgeError>
    where
        F: FnOnce(*mut c_void) -> *mut c_char,
    {
        let returned = invoke(self.thread);
        let owned = unsafe { BridgeString::adopt(returned, BridgeDeallocator { bridge: self }) }?;
        Ok(owned.to_utf8_lossy())
    }
}

unsafe fn resolve<T: Copy>(
    library: &libloading::Library,
    name: &'static [u8],
) -> Result<T, BridgeError> {
    let symbol = unsafe { library.get::<T>(name) }.map_err(|error| BridgeError::MissingSymbol {
        symbol: String::from_utf8_lossy(&name[..name.len() - 1]).into_owned(),
        detail: error.to_string(),
    })?;
    Ok(*symbol)
}

impl Drop for NativeBridge {
    fn drop(&mut self) {
        if self.isolate.is_null() {
            return;
        }
        unsafe {
            (self.tear_down)(self.thread);
        }
        self.isolate = std::ptr::null_mut();
        self.thread = std::ptr::null_mut();
    }
}

fn c_string(value: &str, name: &'static str) -> Result<CString, BridgeError> {
    CString::new(value).map_err(|_| BridgeError::InvalidArgument(name))
}

/// Parsea la respuesta JSON de prefirma.
pub fn parse_presign(json: &str) -> Result<PreSignature, BridgeError> {
    let response = parse_response(json)?;
    let session = field(&response, "session")?.to_owned();
    let stamp = SessionSeal::from_bridge(field(&response, "stamp")?);
    let pre_sign = base64::engine::general_purpose::STANDARD
        .decode(field(&response, "pre")?)
        .map_err(|error| BridgeError::MalformedResponse(format!("pre no es Base64: {error}")))?;
    Ok(PreSignature {
        session,
        pre_sign,
        stamp,
    })
}

/// Parsea la respuesta JSON de postfirma.
pub fn parse_postsign(json: &str) -> Result<Vec<u8>, BridgeError> {
    let response = parse_response(json)?;
    base64::engine::general_purpose::STANDARD
        .decode(field(&response, "pdf")?)
        .map_err(|error| BridgeError::MalformedResponse(format!("pdf no es Base64: {error}")))
}

/// Parsea la respuesta JSON del filtrado de certificados.
pub fn parse_filter_selection(json: &str) -> Result<Vec<usize>, BridgeError> {
    let response = parse_response(json)?;
    let selected = response
        .get("selected")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| BridgeError::MalformedResponse("falta el campo \"selected\"".to_owned()))?;

    selected
        .iter()
        .map(|index| {
            index
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| {
                    BridgeError::MalformedResponse(format!("«{index}» no es un indice del listado"))
                })
        })
        .collect()
}

/// Parsea la respuesta JSON de expansión de parámetros.
pub fn parse_expanded_params(json: &str) -> Result<String, BridgeError> {
    let response = parse_response(json)?;
    Ok(field(&response, "params")?.to_owned())
}

const UNREGISTERED_SIGNATURES_KIND: &str = "pdfHasUnregisteredSignatures";
const INCOMPATIBLE_POLICY_KIND: &str = "incompatiblePolicy";

fn parse_response(json: &str) -> Result<serde_json::Value, BridgeError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| BridgeError::MalformedResponse(format!("{error}: {json}")))?;
    match value.get("ok").and_then(serde_json::Value::as_bool) {
        Some(true) => Ok(value),
        Some(false) => {
            let detail = value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("sin detalle")
                .to_owned();
            Err(
                match value.get("kind").and_then(serde_json::Value::as_str) {
                    Some(UNREGISTERED_SIGNATURES_KIND) => {
                        BridgeError::PdfHasUnregisteredSignatures(detail)
                    }
                    Some(INCOMPATIBLE_POLICY_KIND) => BridgeError::IncompatiblePolicy(detail),
                    _ => BridgeError::Failed(detail),
                },
            )
        }
        None => Err(BridgeError::MalformedResponse(format!(
            "no trae \"ok\": {json}"
        ))),
    }
}

fn field<'a>(response: &'a serde_json::Value, name: &str) -> Result<&'a str, BridgeError> {
    response
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| BridgeError::MalformedResponse(format!("falta el campo \"{name}\"")))
}

#[cfg(test)]
mod tests;
