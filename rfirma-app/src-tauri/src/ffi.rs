//! Frontera FFI con `librfirma_crypto.so` compilada con GraalVM Native Image (ADR-0003, ADR-0004).

use std::ffi::{CStr, CString, OsString};
use std::fmt;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use base64::Engine;

use crate::signing::SessionSeal;

/// Nombre del fichero de la librería nativa compartida (ADR-0004, ADR-0012).
pub const LIBRARY_FILE: &str = "librfirma_crypto.so";

/// Variable de entorno que sobreescribe el directorio de la librería nativa.
pub const LIBRARY_DIRECTORY_VARIABLE: &str = "RFIRMA_LIB_DIR";

const RELATIVE_LIBRARY_DIRECTORY: &str = "../lib/rfirma";

/// Procedencia de un directorio candidato para la librería nativa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Directorio indicado en [`LIBRARY_DIRECTORY_VARIABLE`].
    Override,
    /// Directorio relativo al ejecutable.
    RelativeToExecutable,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Override => write!(f, "{LIBRARY_DIRECTORY_VARIABLE}"),
            Self::RelativeToExecutable => write!(f, "relativa al ejecutable"),
        }
    }
}

/// Directorio candidato para la librería nativa y su procedencia.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    directory: PathBuf,
    origin: Origin,
}

impl Candidate {
    /// Directorio examinado.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Procedencia del candidato.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Ruta esperada del fichero de la librería en este directorio.
    pub fn library_path(&self) -> PathBuf {
        self.directory.join(LIBRARY_FILE)
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.library_path().display(), self.origin)
    }
}

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

/// Error cuando la librería nativa no se encuentra en ningún directorio candidato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryNotFound {
    looked_at: Vec<Candidate>,
}

impl LibraryNotFound {
    /// Candidatos examinados.
    pub fn looked_at(&self) -> &[Candidate] {
        &self.looked_at
    }
}

impl fmt::Display for LibraryNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no encuentro {LIBRARY_FILE}; he mirado en:")?;
        for candidate in &self.looked_at {
            write!(f, "\n  {candidate}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LibraryNotFound {}

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

/// Resultado de la prefirma descompuesto en sus partes (ADR-0016).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreSignature {
    session: String,
    pre_sign: Vec<u8>,
    stamp: SessionSeal,
}

impl PreSignature {
    /// Datos trifásicos de la prefirma para la postfirma.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Bytes DER de los atributos firmados.
    pub fn pre_sign(&self) -> &[u8] {
        &self.pre_sign
    }

    /// Sello de sesión emitido por la prefirma.
    pub fn stamp(&self) -> &SessionSeal {
        &self.stamp
    }
}

/// Parámetros para la llamada de prefirma PAdES.
#[derive(Clone, Copy, Debug)]
pub struct PreSignRequest<'a> {
    /// PDF de entrada en Base64.
    pub pdf_b64: &'a str,
    /// Algoritmo de firma.
    pub algorithm: &'a str,
    /// Cadena de certificados en Base64 separada por punto y coma.
    pub certificate_chain_b64: &'a str,
    /// Parámetros adicionales en formato de propiedades.
    pub extra_params: &'a str,
}

/// Parámetros para la llamada de postfirma PAdES (ADR-0016).
#[derive(Clone, Copy, Debug)]
pub struct PostSignRequest<'a> {
    /// Mismo PDF de entrada que recibió la prefirma, en Base64.
    pub pdf_b64: &'a str,
    /// Misma cadena de certificados.
    pub certificate_chain_b64: &'a str,
    /// Sello devuelto por la prefirma.
    pub stamp: &'a SessionSeal,
    /// Datos trifásicos devueltos por la prefirma.
    pub session: &'a str,
    /// Firma PKCS#1 en Base64 calculada sobre los atributos firmados.
    pub pkcs1_b64: &'a str,
}

/// Parámetros para acotar un listado con el filtro de la sede.
#[derive(Clone, Copy, Debug)]
pub struct FilterRequest<'a> {
    /// Expresión de la sede en formato de propiedades.
    pub filter_properties: &'a str,
    /// Certificados en Base64 del DER separados por punto y coma.
    pub certificates_b64: &'a str,
}

/// Parámetros para expandir la política de firma que declara la sede.
#[derive(Clone, Copy, Debug)]
pub struct ExpandRequest<'a> {
    /// Parámetros de la sede en formato de propiedades.
    pub extra_params: &'a str,
    /// Formato de firma.
    pub format: &'a str,
}

/// Errores posibles al cruzar la frontera FFI con el puente nativo.
#[derive(Debug)]
pub enum BridgeError {
    /// No se puede determinar la ruta del ejecutable.
    ExecutablePathUnknown(String),
    /// No hay librería que cargar.
    NotFound(LibraryNotFound),
    /// Error de carga dinámica de la librería.
    Load {
        /// Fichero que se intentó abrir.
        path: PathBuf,
        /// Detalle devuelto por el cargador dinámico.
        detail: String,
    },
    /// Falta un símbolo esperado en la librería.
    MissingSymbol {
        /// Símbolo ausente.
        symbol: String,
        /// Detalle devuelto por el cargador dinámico.
        detail: String,
    },
    /// Error al crear el isolate de GraalVM.
    IsolateFailed(c_int),
    /// Argumento con byte nulo no convertible a CString.
    InvalidArgument(&'static str),
    /// El puente ha devuelto un puntero nulo.
    NullResponse,
    /// Respuesta con formato no válido devuelta por el puente.
    MalformedResponse(String),
    /// Fallo devuelto por el puente nativo.
    Failed(String),
    /// La política de firma no se puede aplicar al formato solicitado.
    IncompatiblePolicy(String),
    /// El PDF contiene firmas no registradas en su diccionario.
    PdfHasUnregisteredSignatures(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutablePathUnknown(detail) => {
                write!(f, "no puedo saber dónde está el ejecutable: {detail}")
            }
            Self::NotFound(error) => write!(f, "{error}"),
            Self::Load { path, detail } => {
                write!(f, "no puedo cargar {}: {detail}", path.display())
            }
            Self::MissingSymbol { symbol, detail } => {
                write!(f, "la librería no exporta {symbol}: {detail}")
            }
            Self::IsolateFailed(code) => write!(f, "graal_create_isolate ha devuelto {code}"),
            Self::InvalidArgument(name) => write!(f, "{name} lleva un \\0 dentro"),
            Self::NullResponse => write!(f, "el puente ha devuelto NULL"),
            Self::MalformedResponse(detail) => write!(f, "respuesta ilegible del puente: {detail}"),
            Self::Failed(detail) => write!(f, "el puente ha fallado: {detail}"),
            Self::IncompatiblePolicy(detail) => {
                write!(f, "la politica de firma no se puede aplicar: {detail}")
            }
            Self::PdfHasUnregisteredSignatures(detail) => {
                write!(f, "el PDF trae firmas no registradas: {detail}")
            }
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<LibraryNotFound> for BridgeError {
    fn from(error: LibraryNotFound) -> Self {
        Self::NotFound(error)
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
        let stamp = c_string(request.stamp.as_bridge_payload(), "el sello")?;
        let session = c_string(request.session, "la sesión")?;
        let pkcs1 = c_string(request.pkcs1_b64, "el PKCS#1")?;
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
mod tests {
    use super::*;
    use std::alloc::{alloc, dealloc, Layout};
    use std::cell::{Cell, RefCell};
    use std::collections::{HashMap, HashSet};

    /// **Grada A**: ni librería nativa, ni token, ni entorno del proceso.
    fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), OsString::from(*value)))
            .collect();
        move |name: &str| map.get(name).cloned()
    }

    #[test]
    fn the_library_is_looked_for_next_to_the_executable() {
        let looked_at = candidates(&environment(&[]), Path::new("/app/bin"));

        assert_eq!(looked_at.len(), 1);
        assert_eq!(looked_at[0].origin(), Origin::RelativeToExecutable);
        assert!(looked_at[0]
            .library_path()
            .ends_with("lib/rfirma/librfirma_crypto.so"));
    }

    #[test]
    fn the_environment_variable_is_looked_at_first() {
        let looked_at = candidates(
            &environment(&[(LIBRARY_DIRECTORY_VARIABLE, "/otro/sitio")]),
            Path::new("/app/bin"),
        );

        assert_eq!(looked_at.len(), 2);
        assert_eq!(looked_at[0].origin(), Origin::Override);
        assert_eq!(
            looked_at[0].library_path(),
            PathBuf::from("/otro/sitio/librfirma_crypto.so")
        );
        assert_eq!(looked_at[1].origin(), Origin::RelativeToExecutable);
    }

    #[test]
    fn an_empty_variable_is_ignored_instead_of_pointing_at_the_working_directory() {
        let looked_at = candidates(
            &environment(&[(LIBRARY_DIRECTORY_VARIABLE, "")]),
            Path::new("/app/bin"),
        );

        assert_eq!(looked_at.len(), 1);
        assert_eq!(looked_at[0].origin(), Origin::RelativeToExecutable);
    }

    #[test]
    fn the_override_wins_when_both_directories_have_the_library() {
        let directory = tempfile::tempdir().expect("debería haber directorio temporal");
        let overridden = directory.path().join("override");
        let next_to_executable = directory.path().join("app/lib/rfirma");
        for place in [&overridden, &next_to_executable] {
            std::fs::create_dir_all(place).expect("debería crearse");
            std::fs::write(place.join(LIBRARY_FILE), b"no es una libreria de verdad")
                .expect("debería escribirse");
        }

        let found = locate(
            &environment(&[(LIBRARY_DIRECTORY_VARIABLE, &overridden.to_string_lossy())]),
            &directory.path().join("app/bin"),
        )
        .expect("debería encontrarla");

        assert_eq!(found, overridden.join(LIBRARY_FILE));
    }

    #[test]
    fn starting_without_the_library_names_the_two_paths_it_looked_at() {
        let directory = tempfile::tempdir().expect("debería haber directorio temporal");
        let overridden = directory.path().join("vacio");

        let error = locate(
            &environment(&[(LIBRARY_DIRECTORY_VARIABLE, &overridden.to_string_lossy())]),
            &directory.path().join("app/bin"),
        )
        .expect_err("sin librería no debería resolverse");

        let message = error.to_string();
        assert_eq!(error.looked_at().len(), 2, "{message}");
        assert!(
            message.contains(&overridden.display().to_string()),
            "{message}"
        );
        assert!(message.contains("lib/rfirma"), "{message}");
        assert!(message.contains(LIBRARY_DIRECTORY_VARIABLE), "{message}");
        assert!(message.contains("relativa al ejecutable"), "{message}");
    }

    #[test]
    fn a_directory_without_the_file_is_not_the_library() {
        let directory = tempfile::tempdir().expect("debería haber directorio temporal");
        std::fs::create_dir_all(directory.path().join("lib/rfirma")).expect("debería crearse");

        let error = locate(&environment(&[]), &directory.path().join("bin"))
            .expect_err("un directorio no es la librería");

        assert_eq!(error.looked_at().len(), 1);
    }

    #[derive(Default)]
    struct Counter {
        live: RefCell<HashSet<usize>>,
        freed: Cell<usize>,
    }

    impl Counter {
        fn allocate(&self, contents: &str) -> *mut c_char {
            let bytes = contents.as_bytes();
            let layout = Layout::array::<u8>(bytes.len() + 1).expect("cabe");
            let pointer = unsafe { alloc(layout) };
            assert!(!pointer.is_null(), "sin memoria");
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
                pointer.add(bytes.len()).write(0);
            }
            assert!(
                self.live.borrow_mut().insert(pointer as usize),
                "el asignador ha devuelto una dirección que ya estaba viva"
            );
            pointer.cast()
        }

        fn freed(&self) -> usize {
            self.freed.get()
        }

        fn live(&self) -> usize {
            self.live.borrow().len()
        }
    }

    impl FreeBridgeString for &Counter {
        unsafe fn free(&self, pointer: *mut c_char) {
            assert!(!pointer.is_null(), "nunca se libera un nulo");
            assert!(
                self.live.borrow_mut().remove(&(pointer as usize)),
                "doble free: {pointer:?} ya se había liberado"
            );
            self.freed.set(self.freed.get() + 1);
            unsafe {
                let length = CStr::from_ptr(pointer).to_bytes().len();
                let layout = Layout::array::<u8>(length + 1).expect("cabe");
                dealloc(pointer.cast(), layout);
            }
        }
    }

    #[test]
    fn every_pointer_the_bridge_returns_is_freed_exactly_once() {
        let counter = Counter::default();

        for _ in 0..1_000 {
            let pointer = counter.allocate(r#"{"ok":true,"pdf":"AAAA"}"#);
            let owned = unsafe { BridgeString::adopt(pointer, &counter) }.expect("no es nulo");
            assert_eq!(owned.to_utf8_lossy(), r#"{"ok":true,"pdf":"AAAA"}"#);
        }

        assert_eq!(counter.freed(), 1_000, "cada cadena se libera");
        assert_eq!(counter.live(), 0, "y no queda ninguna sin liberar");
    }

    #[test]
    fn the_pointer_is_freed_even_when_the_response_is_unusable() {
        let counter = Counter::default();

        let pointer = counter.allocate("esto no es JSON");
        let owned = unsafe { BridgeString::adopt(pointer, &counter) }.expect("no es nulo");
        let error = parse_presign(&owned.to_utf8_lossy()).expect_err("no es el JSON del contrato");
        drop(owned);

        assert!(matches!(error, BridgeError::MalformedResponse(_)));
        assert_eq!(counter.freed(), 1, "el camino de error también libera");
    }

    #[test]
    fn a_null_answer_is_an_error_and_frees_nothing() {
        let counter = Counter::default();

        let adopted = unsafe { BridgeString::adopt(std::ptr::null_mut(), &counter) };

        assert!(matches!(adopted, Err(BridgeError::NullResponse)));
        assert_eq!(counter.freed(), 0);
    }

    #[test]
    fn a_presign_answer_comes_back_split_into_its_three_pieces() {
        let signature =
            parse_presign(r#"{"ok":true,"session":"<xml/>","pre":"MTIz","stamp":"c2VsbG8="}"#)
                .expect("es el JSON del contrato");

        assert_eq!(signature.session(), "<xml/>");
        assert_eq!(signature.pre_sign(), b"123");
        assert_eq!(
            signature.stamp(),
            &SessionSeal::from_bridge("c2VsbG8="),
            "el sello viaja opaco, tal y como vino"
        );
    }

    #[test]
    fn a_postsign_answer_comes_back_as_the_bytes_of_the_pdf() {
        let pdf =
            parse_postsign(r#"{"ok":true,"pdf":"JVBERi0="}"#).expect("es el JSON del contrato");

        assert_eq!(pdf, b"%PDF-");
    }

    #[test]
    fn a_filter_answer_comes_back_as_the_rows_that_survived() {
        let selected =
            parse_filter_selection(r#"{"ok":true,"selected":[0,2]}"#).expect("es valida");

        assert_eq!(selected, vec![0, 2]);
    }

    #[test]
    fn an_empty_selection_is_an_answer_and_not_a_failure() {
        assert_eq!(
            parse_filter_selection(r#"{"ok":true,"selected":[]}"#).expect("es valida"),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn a_selection_that_is_not_a_list_of_rows_is_a_malformed_answer() {
        assert!(parse_filter_selection(r#"{"ok":true}"#).is_err());
        assert!(parse_filter_selection(r#"{"ok":true,"selected":"0,2"}"#).is_err());
        assert!(parse_filter_selection(r#"{"ok":true,"selected":[-1]}"#).is_err());
    }

    #[test]
    fn a_failure_of_the_filter_engine_travels_like_any_other() {
        let error = parse_filter_selection(
            r#"{"ok":false,"error":"java.lang.IllegalArgumentException: mal"}"#,
        )
        .expect_err("el motor ha fallado");

        assert!(error.to_string().contains("IllegalArgumentException"));
    }

    #[test]
    fn a_failure_from_the_bridge_keeps_the_java_message_untranslated() {
        let error = parse_presign(r#"{"ok":false,"error":"java.io.IOException: no es un PDF"}"#)
            .expect_err("ok:false es un fallo");

        let message = error.to_string();
        assert!(matches!(error, BridgeError::Failed(_)), "{message}");
        assert!(message.contains("java.io.IOException"), "{message}");
    }

    #[test]
    fn a_pdf_with_unregistered_signatures_is_not_just_a_failure() {
        let error = parse_presign(
            r#"{"ok":false,"kind":"pdfHasUnregisteredSignatures","error":"PdfHasUnregisteredSignaturesException"}"#,
        )
        .expect_err("ok:false es un fallo");

        assert!(
            matches!(error, BridgeError::PdfHasUnregisteredSignatures(_)),
            "{error}"
        );
    }

    #[test]
    fn a_failure_kind_this_binary_does_not_know_is_still_a_failure() {
        let error = parse_presign(r#"{"ok":false,"kind":"loQueSea","error":"algo"}"#)
            .expect_err("ok:false es un fallo");

        assert!(matches!(error, BridgeError::Failed(_)), "{error}");
    }

    #[test]
    fn an_answer_without_the_ok_field_is_not_a_signature() {
        let error = parse_postsign(r#"{"pdf":"JVBERi0="}"#).expect_err("falta \"ok\"");

        assert!(matches!(error, BridgeError::MalformedResponse(_)));
    }

    #[test]
    fn an_answer_missing_a_field_is_not_a_signature_either() {
        let error = parse_presign(r#"{"ok":true,"session":"<xml/>","pre":"MTIz"}"#)
            .expect_err("falta stamp");

        assert!(error.to_string().contains("stamp"), "{error}");
    }

    #[test]
    fn a_field_that_is_not_base64_is_a_malformed_answer_and_not_a_panic() {
        let error = parse_postsign(r#"{"ok":true,"pdf":"esto no es base64 %%%"}"#)
            .expect_err("no es Base64");

        assert!(matches!(error, BridgeError::MalformedResponse(_)));
    }

    #[test]
    fn every_failure_of_the_border_says_what_actually_went_wrong() {
        let directory = tempfile::tempdir().expect("debería haber directorio temporal");
        let not_found = locate(&environment(&[]), &directory.path().join("bin"))
            .expect_err("sin librería no debería resolverse");

        let messages = [
            (
                BridgeError::ExecutablePathUnknown("no such file".to_owned()),
                "ejecutable",
            ),
            (BridgeError::from(not_found), "lib/rfirma"),
            (
                BridgeError::Load {
                    path: PathBuf::from("/app/lib/rfirma/librfirma_crypto.so"),
                    detail: "no es un ELF".to_owned(),
                },
                "librfirma_crypto.so",
            ),
            (
                BridgeError::MissingSymbol {
                    symbol: "autofirma_free_string".to_owned(),
                    detail: "undefined symbol".to_owned(),
                },
                "autofirma_free_string",
            ),
            (BridgeError::IsolateFailed(7), "graal_create_isolate"),
            (BridgeError::InvalidArgument("el PDF"), "el PDF"),
            (BridgeError::NullResponse, "NULL"),
            (
                BridgeError::MalformedResponse("no trae \"ok\"".to_owned()),
                "respuesta ilegible",
            ),
            (
                BridgeError::Failed("java.io.IOException: no es un PDF".to_owned()),
                "java.io.IOException",
            ),
        ];

        for (error, expected) in messages {
            let message = error.to_string();
            assert!(
                message.contains(expected),
                "{error:?} debería nombrar «{expected}»: {message}"
            );
        }
    }

    #[test]
    fn not_knowing_where_the_executable_is_does_not_blame_the_bridge() {
        let error = BridgeError::ExecutablePathUnknown("no such file".to_owned()).to_string();

        assert!(!error.contains("puente"), "{error}");
    }

    #[test]
    fn an_argument_with_a_nul_inside_is_rejected_before_crossing() {
        let error = c_string("con\0nulo", "el PDF").expect_err("no puede ser una cadena C");

        assert!(matches!(error, BridgeError::InvalidArgument("el PDF")));
    }
}
