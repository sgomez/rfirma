//! La frontera FFI: cargar `librfirma_crypto.so` y volver de ella sin fugas ni
//! dobles liberaciones (ID-10, ID-11; ADR-0003, ADR-0004, ADR-0013).
//!
//! Aquí **no hay orquestación**: se cruza la frontera y se vuelve. Quién llama
//! a la prefirma, quién firma el PKCS#1 con el token y quién comprueba el sello
//! entre las dos fases vive fuera de este fichero.
//!
//! Tres cosas que este módulo no delega en nadie:
//!
//! 1. **De dónde sale la librería.** De una ruta **relativa al ejecutable**
//!    (`../lib/rfirma`), sobreescribible con [`LIBRARY_DIRECTORY_VARIABLE`]. Ni
//!    `LD_LIBRARY_PATH` ni `RPATH`: el flatpak instala el fichero en
//!    `/app/lib/rfirma` y el binario en `/app/bin`, así que la ruta relativa ya
//!    es la correcta sin tocar el entorno de nadie.
//! 2. **Qué se dice cuando no está.** El fallo nombra **las dos rutas que se
//!    miraron**, con su procedencia. No es cortesía: cuando faltaban ficheros,
//!    la firma no fallaba al cargar sino más tarde, con un error engañoso sobre
//!    el formato de la imagen, y el rato que costó encontrarlo lo pagó el #36.
//! 3. **Quién libera el JSON.** Java lo reserva a mano en el C-heap
//!    (`UnmanagedMemory.malloc`) y **Rust llama a `autofirma_free_string`**.
//!    Nunca al revés: `CTypeConversion.toCString` liberaría al salir del bloque
//!    y esta parte haría un doble `free`. Aquí eso no se recuerda a mano —lo
//!    recuerda [`BridgeString`], que libera al soltarse, exactamente una vez.
//!
//! **Por qué se busca en las dos rutas y no solo en la del entorno.** Un
//! `RFIRMA_LIB_DIR` apuntando a un directorio vacío no puede acabar en «no hay
//! librería» a secas: el mensaje tiene que enseñar también dónde habría mirado
//! por omisión, o quien depura no sabe si le falta el fichero o le sobra la
//! variable. Buscar en las dos y nombrarlas las dos es la única versión de esto
//! que no miente.

use std::ffi::{CStr, CString, OsString};
use std::fmt;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

use base64::Engine;

use crate::signing::SessionSeal;

/// El único fichero que hace falta (ADR-0004, ADR-0012).
///
/// Era una lista de seis hasta el #36: al excluir `afirma-ui-utils` los cinco
/// auxiliares de AWT dejaron de hacer falta, y volver a ponerlos convierte un
/// error recuperable ante un JPEG con perfil ICC en un aborto del proceso.
pub const LIBRARY_FILE: &str = "librfirma_crypto.so";

/// La variable que sobreescribe el directorio de la librería.
pub const LIBRARY_DIRECTORY_VARIABLE: &str = "RFIRMA_LIB_DIR";

/// El directorio de la librería relativo al del ejecutable.
const RELATIVE_LIBRARY_DIRECTORY: &str = "../lib/rfirma";

/// De dónde salió un directorio candidato. Va en el mensaje de fallo, porque
/// «no está en /app/lib/rfirma» y «no está donde apunta tu variable» se
/// arreglan de maneras distintas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// El valor de [`LIBRARY_DIRECTORY_VARIABLE`].
    Override,
    /// `../lib/rfirma` desde el directorio del ejecutable.
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

/// Un sitio donde se ha mirado, y por qué se miró ahí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    directory: PathBuf,
    origin: Origin,
}

impl Candidate {
    /// El directorio, tal cual se miró.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// De dónde salió.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// El fichero completo que se buscó dentro.
    pub fn library_path(&self) -> PathBuf {
        self.directory.join(LIBRARY_FILE)
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.library_path().display(), self.origin)
    }
}

/// Los sitios donde se busca la librería, **en orden**: primero la variable de
/// entorno, después la ruta relativa al ejecutable.
///
/// Recibe el entorno y el directorio del ejecutable en vez de leerlos, por la
/// misma razón que [`crate::paths::Paths::resolve`]: cambiar el entorno del
/// proceso es global y las pruebas corren en hilos.
///
/// Una variable vacía se ignora igual que si no estuviera: heredar
/// `RFIRMA_LIB_DIR=` de un script no puede acabar buscando en el directorio de
/// trabajo de turno.
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

/// Quita el `..` cuando se puede, para que el mensaje de fallo enseñe una ruta
/// que se pueda pegar en un `ls`. Si el directorio no existe —que es justo el
/// caso interesante— se queda la ruta tal cual, que sigue siendo cierta.
fn normalise(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// El primer candidato que contiene la librería, o el fallo que nombra todos
/// los que se miraron.
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

/// La librería nativa no está en ninguno de los sitios donde se miró.
///
/// El `Display` los nombra **todos**, uno por línea y con su procedencia: es la
/// única información que convierte «rfirma no arranca» en algo accionable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryNotFound {
    looked_at: Vec<Candidate>,
}

impl LibraryNotFound {
    /// Los sitios donde se miró, en el orden en que se miraron.
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
///
/// Es un rasgo y no una llamada directa para que la disciplina de memoria se
/// pueda probar **sin la librería nativa**: la grada C comprueba que el ciclo
/// real no se cae, y la grada A comprueba —contando— que cada puntero se libera
/// una vez y solo una.
pub trait FreeBridgeString {
    /// Libera un puntero que devolvió el puente. Nunca se llama con nulo.
    ///
    /// # Safety
    ///
    /// `pointer` tiene que venir de este mismo puente y no haberse liberado
    /// antes: liberar dos veces corrompe el montón sin decir nada.
    unsafe fn free(&self, pointer: *mut c_char);
}

/// Una cadena **propiedad de Rust** que vino del puente.
///
/// Existe para que «hay que llamar a `autofirma_free_string`» deje de ser algo
/// que alguien recuerda: se libera al soltarse, en todos los caminos, incluidos
/// los de error y los de pánico. El puntero se pone a nulo al liberarlo, así
/// que un doble `free` no es que sea improbable: no hay forma de escribirlo.
pub struct BridgeString<D: FreeBridgeString> {
    pointer: *mut c_char,
    deallocator: D,
}

impl<D: FreeBridgeString> BridgeString<D> {
    /// Adopta el puntero que acaba de devolver el puente.
    ///
    /// # Safety
    ///
    /// `pointer` tiene que ser nulo o una cadena C terminada en `\0` reservada
    /// por el puente, y nadie más puede quedarse una copia: a partir de aquí la
    /// libera este valor.
    pub unsafe fn adopt(pointer: *mut c_char, deallocator: D) -> Result<Self, BridgeError> {
        if pointer.is_null() {
            return Err(BridgeError::NullResponse);
        }
        Ok(Self {
            pointer,
            deallocator,
        })
    }

    /// El contenido, copiado a memoria de Rust.
    ///
    /// Copiar no es un descuido: la cadena de origen se libera en cuanto este
    /// valor se suelta, y devolver algo que apunte dentro sería devolver
    /// memoria liberada.
    pub fn to_utf8_lossy(&self) -> String {
        // SAFETY: el puntero no es nulo (lo comprueba `adopt`) y sigue vivo
        // porque solo lo libera `Drop`.
        unsafe { CStr::from_ptr(self.pointer) }
            .to_string_lossy()
            .into_owned()
    }
}

impl<D: FreeBridgeString> Drop for BridgeString<D> {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            // SAFETY: el puntero vino del puente, no es nulo, y este es el único
            // sitio que lo libera —después queda a nulo—.
            unsafe { self.deallocator.free(self.pointer) };
            self.pointer = std::ptr::null_mut();
        }
    }
}

/// Lo que devuelve la prefirma, ya separado en sus tres piezas.
///
/// `session` y `stamp` son **opacos**: viajan a la postfirma tal cual (ADR-0016)
/// y aquí nadie los interpreta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreSignature {
    session: String,
    pre_sign: Vec<u8>,
    stamp: SessionSeal,
}

impl PreSignature {
    /// El `TriphaseData` de la prefirma, para devolvérselo a la postfirma.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Los bytes DER de los atributos firmados: lo que el token firma.
    ///
    /// Son los bytes **sin hashear**; el mecanismo del token es
    /// `CKM_SHA256_RSA_PKCS`, que hashea él.
    pub fn pre_sign(&self) -> &[u8] {
        &self.pre_sign
    }

    /// El sello de sesión, que la postfirma exige idéntico.
    pub fn stamp(&self) -> &SessionSeal {
        &self.stamp
    }
}

/// Lo que hace falta para prefirmar. Todo en Base64 salvo lo que no lo es,
/// porque es lo que el puente espera.
#[derive(Clone, Copy, Debug)]
pub struct PreSignRequest<'a> {
    /// El PDF de entrada en Base64.
    pub pdf_b64: &'a str,
    /// El algoritmo, p. ej. `SHA256withRSA`.
    pub algorithm: &'a str,
    /// La cadena de certificados en Base64, separada por `;`.
    pub certificate_chain_b64: &'a str,
    /// Los `extraParams`, en formato `java.util.Properties`.
    pub extra_params: &'a str,
}

/// Lo que hace falta para postfirmar.
///
/// No lleva ni algoritmo ni `extraParams`: los toma el puente del sello, que es
/// justamente lo que impide que se desvíen de la prefirma (ADR-0016).
#[derive(Clone, Copy, Debug)]
pub struct PostSignRequest<'a> {
    /// El **mismo** PDF de entrada que recibió la prefirma, en Base64.
    pub pdf_b64: &'a str,
    /// La **misma** cadena de certificados.
    pub certificate_chain_b64: &'a str,
    /// El sello que devolvió la prefirma, tal cual.
    pub stamp: &'a SessionSeal,
    /// El `TriphaseData` de la prefirma, tal cual.
    pub session: &'a str,
    /// El PKCS#1 que calculó el token sobre los atributos firmados, en Base64.
    pub pkcs1_b64: &'a str,
}

/// Lo que hace falta para acotar un listado con el filtro de la sede.
///
/// No lleva sello ni sesión, y no es un olvido: la llamada es **sin estado**
/// (ID-252). El sello existe para que la postfirma no se desvíe de la prefirma,
/// y aquí no hay dos fases que atar.
#[derive(Clone, Copy, Debug)]
pub struct FilterRequest<'a> {
    /// La expresión de la sede en formato `java.util.Properties`, **literal**
    /// (ID-256): quien la interpreta es el motor.
    pub filter_properties: &'a str,
    /// Los certificados a acotar, Base64 del DER separado por `;`, en su orden.
    pub certificates_b64: &'a str,
}

/// Lo que puede salir mal al cruzar la frontera.
///
/// [`BridgeError::Failed`] es el puente contestando `{"ok":false}`: no es un
/// fallo de la frontera, es la firma que no ha podido hacerse, y lleva el texto
/// crudo de Java sin traducir para poder pegarlo en un informe.
#[derive(Debug)]
pub enum BridgeError {
    /// No se sabe dónde está el ejecutable, así que no hay desde dónde medir la
    /// ruta relativa de la librería. Aquí todavía no se ha cruzado ninguna
    /// frontera: el puente no ha tenido nada que ver.
    ExecutablePathUnknown(String),
    /// No hay librería que cargar.
    NotFound(LibraryNotFound),
    /// `dlopen` ha fallado.
    Load {
        /// El fichero que se intentó abrir.
        path: PathBuf,
        /// Lo que dijo `dlopen`.
        detail: String,
    },
    /// Falta un símbolo: la librería no es la que este código espera.
    MissingSymbol {
        /// El símbolo que no estaba.
        symbol: String,
        /// Lo que dijo `dlsym`.
        detail: String,
    },
    /// GraalVM no ha podido crear el isolate.
    IsolateFailed(c_int),
    /// Un argumento lleva un `\0` dentro y no puede ser una cadena C.
    InvalidArgument(&'static str),
    /// El puente ha devuelto un puntero nulo, que no es una respuesta.
    NullResponse,
    /// El puente ha contestado algo que no es el JSON del contrato.
    MalformedResponse(String),
    /// El puente ha contestado `{"ok":false,...}`.
    Failed(String),
    /// El puente ha contestado `{"ok":false,"kind":"pdfHasUnregisteredSignatures",...}`:
    /// el PDF trae firmas que su propio diccionario no registra, y firmarlo
    /// encima puede invalidar las que ya tenía. **No es un fallo cualquiera**
    /// (ID-296): es la situación que la sede tiene que confirmar, y por eso
    /// llega con nombre propio en vez de colapsada en [`BridgeError::Failed`].
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

/// La librería nativa cargada, con su isolate de GraalVM ya creado.
///
/// **No es `Sync`, y es a propósito**: el `IsolateThread` de GraalVM pertenece
/// al hilo que lo creó. Compartir este valor entre hilos sería el segundo fallo
/// silencioso de esta frontera, así que el tipo no deja.
pub struct NativeBridge {
    /// Nadie la lee, y aun así tiene que estar: es lo que mantiene cargada la
    /// librería —y por tanto válidos los cinco punteros a función de abajo—
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
        let (create, presign, postsign, filter, free_string, tear_down) = unsafe {
            (
                resolve::<CreateIsolate>(&library, b"graal_create_isolate\0")?,
                resolve::<PreSignSymbol>(&library, b"autofirma_pades_presign\0")?,
                resolve::<PostSignSymbol>(&library, b"autofirma_pades_postsign\0")?,
                resolve::<FilterSymbol>(&library, b"autofirma_filter_certificates\0")?,
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
            free_string,
            tear_down,
        })
    }

    /// El fichero que se cargó.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Prefirma PAdES: devuelve el `TriphaseData`, los bytes que hay que firmar
    /// y el sello de sesión.
    pub fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError> {
        let pdf = c_string(request.pdf_b64, "el PDF")?;
        let algorithm = c_string(request.algorithm, "el algoritmo")?;
        let chain = c_string(request.certificate_chain_b64, "la cadena de certificados")?;
        let extra = c_string(request.extra_params, "los extraParams")?;
        let json = self.call(|thread| {
            // SAFETY: los cinco argumentos viven hasta que la llamada vuelve.
            unsafe {
                (self.presign)(
                    thread,
                    pdf.as_ptr(),
                    algorithm.as_ptr(),
                    chain.as_ptr(),
                    extra.as_ptr(),
                )
            }
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
        let json = self.call(|thread| {
            // SAFETY: los seis argumentos viven hasta que la llamada vuelve.
            unsafe {
                (self.postsign)(
                    thread,
                    pdf.as_ptr(),
                    chain.as_ptr(),
                    stamp.as_ptr(),
                    session.as_ptr(),
                    pkcs1.as_ptr(),
                )
            }
        })?;
        parse_postsign(&json)
    }

    /// Acota un listado de certificados con la expresión de filtro de la sede.
    ///
    /// **Sin estado y sin sello** (ADR-0016, ID-252): no abre ninguna sesión
    /// trifásica, así que no hay nada que atar entre dos llamadas. El DER ya
    /// viaja en cada certificado.
    ///
    /// Devuelve los **índices** que pasan, sobre la lista que se le dio y en su
    /// orden. Índices y no certificados porque quien llamó ya los tiene: lo que
    /// le falta es saber cuáles siguen dentro.
    pub fn filter_certificates(
        &self,
        request: FilterRequest<'_>,
    ) -> Result<Vec<usize>, BridgeError> {
        let properties = c_string(request.filter_properties, "la expresion de filtro")?;
        let certificates = c_string(request.certificates_b64, "los certificados")?;
        let json = self.call(|thread| {
            // SAFETY: los tres argumentos viven hasta que la llamada vuelve.
            unsafe { (self.filter)(thread, properties.as_ptr(), certificates.as_ptr()) }
        })?;
        parse_filter_selection(&json)
    }

    /// El único sitio que toca un puntero devuelto por el puente: llama, adopta
    /// la cadena y la libera al salir.
    fn call<F>(&self, invoke: F) -> Result<String, BridgeError>
    where
        F: FnOnce(*mut c_void) -> *mut c_char,
    {
        let returned = invoke(self.thread);
        // SAFETY: lo que devuelve el puente es una cadena C reservada con
        // `UnmanagedMemory.malloc` y nadie más se queda una copia (ID-11).
        let owned = unsafe { BridgeString::adopt(returned, BridgeDeallocator { bridge: self }) }?;
        Ok(owned.to_utf8_lossy())
    }
}

/// Busca un símbolo y se queda el **puntero a función**, no el préstamo.
///
/// Se resuelven los cinco de una vez al cargar, y antes de crear el isolate,
/// por tres razones: que la librería que no exporta alguno falle al abrirse en
/// vez de a la primera firma —o, en el caso de `autofirma_free_string`, en vez
/// de no fallar nunca y filtrar en silencio—; que un símbolo que falta no deje
/// un isolate huérfano ni un `dlclose` con el isolate dentro; y que firmar y
/// desmontar no paguen un `dlsym` cada uno.
///
/// # Safety
///
/// `T` tiene que ser la firma real del símbolo, y el puntero solo vale mientras
/// la librería siga cargada.
unsafe fn resolve<T: Copy>(
    library: &libloading::Library,
    name: &'static [u8],
) -> Result<T, BridgeError> {
    // SAFETY: el nombre está terminado en `\0` y la firma la pone quien llama.
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
        // SAFETY: el isolate lo creó este mismo valor y nadie más lo ha
        // destruido; el símbolo se resolvió al abrir, así que aquí no hay nada
        // que pueda fallar en silencio; después de esto el puente ya no se usa.
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

/// El JSON de la prefirma: `{"ok":true,"session":..,"pre":..,"stamp":..}`.
///
/// Se separa de la llamada para que el contrato se pueda probar sin librería
/// nativa: la forma del JSON es lo que se rompe cuando alguien toca el puente,
/// y esa prueba tiene que estar en el carril rápido.
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

/// El JSON de la postfirma: `{"ok":true,"pdf":"<b64>"}`.
pub fn parse_postsign(json: &str) -> Result<Vec<u8>, BridgeError> {
    let response = parse_response(json)?;
    base64::engine::general_purpose::STANDARD
        .decode(field(&response, "pdf")?)
        .map_err(|error| BridgeError::MalformedResponse(format!("pdf no es Base64: {error}")))
}

/// El JSON del filtro: `{"ok":true,"selected":[0,2]}`.
///
/// Se separa de la llamada por lo mismo que [`parse_presign`]: la forma del
/// JSON tiene que poder probarse sin librería nativa delante.
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

/// La clase de fallo con la que el puente marca un PDF con firmas no
/// registradas (`NativeBridge.errorJson`, ID-296).
const UNREGISTERED_SIGNATURES_KIND: &str = "pdfHasUnregisteredSignatures";

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
            // `kind` es la clase de fallo, y sólo una se distingue (ID-296).
            // Un puente viejo no lo trae y un puente nuevo puede traer una
            // clase que aquí no se conozca: las dos son `Failed`, que es
            // exactamente lo que eran antes.
            Err(
                match value.get("kind").and_then(serde_json::Value::as_str) {
                    Some(UNREGISTERED_SIGNATURES_KIND) => {
                        BridgeError::PdfHasUnregisteredSignatures(detail)
                    }
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

    /// El contador de la instrumentación de memoria de la grada A: reserva la
    /// cadena como la reservaría el puente y **lleva la cuenta de lo que sigue
    /// vivo**.
    ///
    /// Anotar las direcciones liberadas y buscar repetidas no valdría: el
    /// asignador reutiliza la dirección que se acaba de liberar, así que dos
    /// vueltas seguidas dan el mismo puntero sin que nadie haya liberado dos
    /// veces. Lo que delata un doble `free` es liberar algo que ya no está
    /// vivo, y eso es lo que se comprueba.
    #[derive(Default)]
    struct Counter {
        live: RefCell<HashSet<usize>>,
        freed: Cell<usize>,
    }

    impl Counter {
        fn allocate(&self, contents: &str) -> *mut c_char {
            let bytes = contents.as_bytes();
            let layout = Layout::array::<u8>(bytes.len() + 1).expect("cabe");
            // SAFETY: el tamaño no es cero (siempre hay al menos el `\0`).
            let pointer = unsafe { alloc(layout) };
            assert!(!pointer.is_null(), "sin memoria");
            // SAFETY: el bloque acaba de reservarse con ese tamaño exacto.
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
            // SAFETY: el puntero salió de `Counter::allocate`, sigue vivo (lo
            // acaba de comprobar el registro) y el tamaño se recalcula sobre la
            // misma cadena que hay dentro.
            unsafe {
                let length = CStr::from_ptr(pointer).to_bytes().len();
                let layout = Layout::array::<u8>(length + 1).expect("cabe");
                dealloc(pointer.cast(), layout);
            }
        }
    }

    /// El camino de ida y vuelta completo bajo instrumentación: una cadena
    /// reservada fuera de Rust, leída, y liberada **una sola vez** (ID-11).
    #[test]
    fn every_pointer_the_bridge_returns_is_freed_exactly_once() {
        let counter = Counter::default();

        for _ in 0..1_000 {
            let pointer = counter.allocate(r#"{"ok":true,"pdf":"AAAA"}"#);
            // SAFETY: el puntero acaba de reservarse y nadie más lo tiene.
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
        // SAFETY: el puntero acaba de reservarse y nadie más lo tiene.
        let owned = unsafe { BridgeString::adopt(pointer, &counter) }.expect("no es nulo");
        let error = parse_presign(&owned.to_utf8_lossy()).expect_err("no es el JSON del contrato");
        drop(owned);

        assert!(matches!(error, BridgeError::MalformedResponse(_)));
        assert_eq!(counter.freed(), 1, "el camino de error también libera");
    }

    #[test]
    fn a_null_answer_is_an_error_and_frees_nothing() {
        let counter = Counter::default();

        // SAFETY: adoptar un nulo es justo lo que se está probando.
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

    /// El filtro contesta con los índices que pasan, y ni uno mas: lo que se
    /// filtró es el listado de quien llamó, no una copia que vuelva.
    #[test]
    fn a_filter_answer_comes_back_as_the_rows_that_survived() {
        let selected =
            parse_filter_selection(r#"{"ok":true,"selected":[0,2]}"#).expect("es valida");

        assert_eq!(selected, vec![0, 2]);
    }

    /// Excluirlos a todos es una **respuesta**, no un fallo: el `[]` es lo que
    /// permite decir «la sede los excluyó» (ID-258).
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

    /// Y un fallo del motor de filtros llega como cualquier otro del puente,
    /// con el mensaje de Java sin traducir.
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

    /// **El PDF con firmas no registradas se distingue de un fallo
    /// cualquiera** (ID-296): el puente lo marca con `kind` y aquí llega con
    /// nombre propio, que es lo que hace posible el `SAF_50`.
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

    /// Y una clase que este binario no conozca —o un puente sin `kind`— sigue
    /// siendo lo que era: un fallo del puente.
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

    /// Cada variante tiene que decir **lo suyo**: la tesis del módulo es que un
    /// fallo de esta frontera no reaparezca disfrazado de otro, y eso solo se
    /// sostiene si el texto de cada una nombra su propia situación.
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

    /// El fallo de `current_exe` no es una respuesta ilegible del puente: ahí
    /// todavía no se ha cruzado nada.
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
