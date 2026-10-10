//! El informe de `--debug-info`: el texto por secciones que se pega en un informe de fallo, formateado desde un `DebugReport` ya relleno, con las rutas sin el home ni el usuario.

use std::path::{Path, PathBuf};

use crate::identity::domain::readers::Reader;

use pkcs11::module_lines;
pub use pkcs11::{DiscardReason, ModuleStatus, Pkcs11Module, Pkcs11Modules};

/// Qué se sabe de la librería nativa tras intentar cargarla de verdad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeLibraryStatus {
    /// Se cargó y resolvió todos sus símbolos.
    Loaded,
    /// No hay librería en ningún sitio donde se busca.
    NotFound,
    /// Se cargó, pero le falta algún símbolo: una librería antigua.
    IncompatibleSymbols,
    /// Está, pero el cargador dinámico no la abre.
    NotLoadable,
}

/// Quién atiende `afirma://`, según lo que se pueda consultar en este canal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolHandlerStatus {
    /// El sandbox no deja preguntarlo al escritorio.
    NotQueryableFromTheSandbox,
    /// Ninguna aplicación tiene registrado el esquema.
    NoneRegistered,
    /// El identificador de la aplicación que lo tiene registrado.
    Registered(String),
}

/// Lo que dice una consulta puntual a PC/SC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcscStatus {
    /// El servicio no contesta.
    NotResponding,
    /// El servicio contesta, con los lectores que ve.
    Responding(Vec<Reader>),
}

/// Qué se sabe de un perfil NSS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NssProfileState {
    /// El navegador confía en el canal local, hasta la fecha de caducidad de su CA.
    TrustsLocalChannel {
        /// Fecha `AAAA-MM-DD`, si se pudo leer.
        until: Option<String>,
    },
    /// El navegador no confía en el canal local.
    DoesNotTrustLocalChannel,
    /// El descubrimiento lo descarta porque no tiene `cert9.db`.
    IgnoredWithoutCertificateDatabase,
}

/// Un perfil NSS que el descubrimiento ve, con el navegador al que pertenece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NssProfile {
    /// Navegador, como se lee en el informe.
    pub browser: String,
    /// Directorio del perfil.
    pub directory: PathBuf,
    /// Qué se sabe de él.
    pub state: NssProfileState,
}

/// Los almacenes NSS: los perfiles y si el Almacén de rFirma existe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NssStores {
    /// Perfiles encontrados o ignorados.
    pub profiles: Vec<NssProfile>,
    /// Si el Almacén de rFirma está instalado.
    pub rfirma_store_installed: bool,
}

/// Lo que solo existe en Windows: el almacén del sistema y los minidrivers de tarjeta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsStores {
    /// Si el almacén raíz del usuario confía en el canal local; nunca es un perfil ignorado.
    pub local_channel: NssProfileState,
    /// Tipos de tarjeta que Windows tiene dados de alta.
    pub minidrivers: Vec<String>,
}

/// Dónde y con qué instalador está el flatpak.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatpakInstallation {
    /// Remoto del que se instaló.
    pub remote: String,
    /// Rama instalada.
    pub branch: String,
    /// Runtime sobre el que corre.
    pub runtime: String,
}

/// Para quién instaló el instalador de Windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallerScope {
    /// Solo para la persona que lo ejecutó.
    PerUser,
    /// Para todo el equipo.
    PerMachine,
}

/// Cómo se instaló rFirma de verdad.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Installation {
    /// Paquete de dpkg.
    Deb,
    /// Paquete de rpm.
    Rpm,
    /// Flatpak, con su origen.
    Flatpak(FlatpakInstallation),
    /// Instalador de Windows.
    WindowsInstaller(InstallerScope),
    /// Aplicación de macOS.
    MacOs,
    /// Ejecutable que no pertenece a ningún paquete, con el commit grabado al compilar.
    Development {
        /// Commit de la compilación.
        commit: String,
    },
}

/// El motor web y su versión.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebView {
    /// `WebKitGTK` o `WebView2`.
    pub name: String,
    /// Versión.
    pub version: String,
}

/// Los datos que solo existen en Linux.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxEnvironment {
    /// Distribución y versión, de `os-release`.
    pub distribution: String,
    /// Entorno de escritorio.
    pub desktop: String,
    /// Tipo de sesión gráfica: Wayland o X11.
    pub session: String,
}

/// Una tarjeta gráfica, tal como la ve el sistema: DRM, el registro de Windows o `system_profiler`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gpu {
    /// Fabricante.
    pub vendor: String,
    /// Controlador del kernel en Linux, descripción del adaptador en Windows, modelo en macOS.
    pub driver: String,
    /// Versión del controlador, si el sistema la declara.
    pub driver_version: Option<String>,
}

/// Quién fijó la variable que rige el renderizador de WebKitGTK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererOrigin {
    /// La fija rFirma al arrancar (ADR-0007).
    SetByRfirma,
    /// Ya venía en el entorno.
    FromTheEnvironment,
}

/// La variable que rige el renderizador de WebKitGTK, con su valor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererVariable {
    /// Nombre de la variable.
    pub name: String,
    /// Valor que tendrá.
    pub value: String,
    /// Quién la fija.
    pub origin: RendererOrigin,
}

/// La extensión GL de NVIDIA que corresponde al controlador del anfitrión.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlExtension {
    /// Versión del controlador a la que corresponde.
    pub driver_version: String,
    /// Si está instalada en el sandbox.
    pub installed: bool,
}

/// Lo que se sabe de los gráficos de esta sesión.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graphics {
    /// Una por tarjeta.
    pub gpus: Vec<Gpu>,
    /// Solo en Linux, y si hay algo que decir.
    pub renderer: Option<RendererVariable>,
    /// `GDK_BACKEND`, si está definida.
    pub display_backend: Option<String>,
    /// Solo en flatpak con NVIDIA.
    pub gl_extension: Option<GlExtension>,
}

/// La librería nativa: cómo acabó y qué fichero se intentó cargar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeLibrary {
    /// Cómo acabó la carga.
    pub status: NativeLibraryStatus,
    /// El fichero, si se llegó a encontrar.
    pub path: Option<PathBuf>,
}

/// De quién es la sesión: lo que ninguna línea del informe puede contener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportOwner {
    /// Directorio personal.
    pub home: PathBuf,
    /// Nombre de usuario.
    pub user_name: String,
}

/// Todo lo que el informe cuenta, recogido ya del entorno.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugReport {
    /// Versión de rFirma.
    pub version: String,
    /// Cómo se instaló.
    pub installation: Installation,
    /// Versión de glibc, solo en Linux nativo.
    pub glibc: Option<String>,
    /// Motor web, si se conoce su versión.
    pub webview: Option<WebView>,
    /// Sistema operativo.
    pub operating_system: String,
    /// Arquitectura.
    pub architecture: String,
    /// Distribución, escritorio y sesión; solo en Linux.
    pub linux: Option<LinuxEnvironment>,
    /// Tarjetas gráficas y renderizador.
    pub graphics: Graphics,
    /// Idioma del sistema.
    pub locale: String,
    /// Quién atiende `afirma://`.
    pub protocol_handler: ProtocolHandlerStatus,
    /// Estado de la librería nativa.
    pub native_library: NativeLibrary,
    /// `RFIRMA_LIB_DIR`, si está definida.
    pub library_directory_override: Option<PathBuf>,
    /// `RFIRMA_PKCS11_MODULE`, si está definida.
    pub pkcs11_module_override: Option<PathBuf>,
    /// PC/SC; `None` donde esta versión no lo consulta.
    pub pcsc: Option<PcscStatus>,
    /// Versión del cliente pcsc-lite que lleva el paquete, si lo lleva.
    pub bundled_pcsc_lite: Option<String>,
    /// Los módulos PKCS#11; `None` donde no hay descubrimiento que contar.
    pub pkcs11_modules: Option<Pkcs11Modules>,
    /// Almacenes NSS; `None` donde esta versión no los consulta.
    pub nss_stores: Option<NssStores>,
    /// Almacén de Windows y minidrivers; `None` fuera de Windows.
    pub windows_stores: Option<WindowsStores>,
}

/// Una cabecera y una sección por bloque, con el patrón `clave: estado · detalle` y las rutas anonimizadas.
pub fn debug_report_text(report: &DebugReport, owner: &ReportOwner) -> String {
    [
        Some(format!("rFirma {}", report.version)),
        section("Sistema", &system_lines(report)),
        section("Gráficos", &graphics_lines(&report.graphics)),
        section("Integración", &integration_lines(report, owner)),
    ]
    .into_iter()
    .chain(
        report
            .pcsc
            .as_ref()
            .map(|pcsc| section("Lectores", &reader_lines(pcsc))),
    )
    .chain(report.pkcs11_modules.as_ref().map(|modules| {
        section(
            "Módulos PKCS#11",
            &module_lines(modules, &report.installation, owner),
        )
    }))
    .chain(
        report
            .nss_stores
            .as_ref()
            .map(|stores| section("Almacenes NSS", &stores::nss_lines(stores, owner))),
    )
    .chain(stores::windows_sections(report.windows_stores.as_ref()))
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n")
}

fn reader_lines(pcsc: &PcscStatus) -> Vec<String> {
    match pcsc {
        PcscStatus::NotResponding => vec!["pcscd: no responde".to_owned()],
        PcscStatus::Responding(readers) => std::iter::once("pcscd: responde".to_owned())
            .chain(readers.iter().map(|reader| {
                let card = if reader.has_a_card {
                    "con tarjeta"
                } else {
                    "sin tarjeta"
                };
                format!("{}: {card}", reader.name)
            }))
            .collect(),
    }
}

fn section(title: &str, lines: &[String]) -> Option<String> {
    if lines.is_empty() {
        return None;
    }
    Some(
        std::iter::once(title.to_owned())
            .chain(lines.iter().map(|line| format!("  {line}")))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn system_lines(report: &DebugReport) -> Vec<String> {
    let mut lines = installation_lines(&report.installation);
    match &report.linux {
        Some(linux) => {
            lines.push(format!("Distribución: {}", linux.distribution));
            lines.push(format!("Arquitectura: {}", report.architecture));
            lines.extend(report.glibc.iter().map(|glibc| format!("glibc: {glibc}")));
            lines.push(format!("Escritorio: {}", linux.desktop));
            lines.push(format!("Sesión: {}", linux.session));
        }
        None => {
            lines.push(format!("Sistema operativo: {}", report.operating_system));
            lines.push(format!("Arquitectura: {}", report.architecture));
        }
    }
    lines.push(format!("Idioma: {}", report.locale));
    lines.extend(
        report
            .webview
            .iter()
            .map(|webview| format!("{}: {}", webview.name, webview.version)),
    );
    if let Some(version) = &report.bundled_pcsc_lite {
        lines.push(format!("pcsc-lite: {version}"));
    }
    lines
}

fn graphics_lines(graphics: &Graphics) -> Vec<String> {
    let mut lines: Vec<String> = graphics.gpus.iter().map(gpu_line).collect();
    if let Some(renderer) = &graphics.renderer {
        let origin = match renderer.origin {
            RendererOrigin::SetByRfirma => "fijado por rFirma",
            RendererOrigin::FromTheEnvironment => "del entorno",
        };
        lines.push(format!(
            "Renderizador: {}={} · {origin}",
            renderer.name, renderer.value
        ));
    }
    if let Some(backend) = &graphics.display_backend {
        lines.push(format!("GDK_BACKEND: {backend}"));
    }
    if let Some(extension) = &graphics.gl_extension {
        let state = if extension.installed {
            "instalada"
        } else {
            "falta"
        };
        lines.push(format!(
            "Extensión GL: {state} · {}",
            extension.driver_version
        ));
    }
    lines
}

fn gpu_line(gpu: &Gpu) -> String {
    match &gpu.driver_version {
        Some(version) => format!("GPU: {} · {} {version}", gpu.vendor, gpu.driver),
        None => format!("GPU: {} · {}", gpu.vendor, gpu.driver),
    }
}

fn installation_lines(installation: &Installation) -> Vec<String> {
    match installation {
        Installation::Deb => vec!["Instalación: deb".to_owned()],
        Installation::Rpm => vec!["Instalación: rpm".to_owned()],
        Installation::MacOs => vec!["Instalación: macOS".to_owned()],
        Installation::Flatpak(flatpak) => vec![
            "Instalación: flatpak".to_owned(),
            format!("  Remoto: {}", flatpak.remote),
            format!("  Rama: {}", flatpak.branch),
            format!("  Runtime: {}", flatpak.runtime),
        ],
        Installation::WindowsInstaller(scope) => vec![format!(
            "Instalación: instalador de Windows · {}",
            match scope {
                InstallerScope::PerUser => "por usuario",
                InstallerScope::PerMachine => "por equipo",
            }
        )],
        Installation::Development { commit } => vec![
            "Instalación: compilación de desarrollo".to_owned(),
            format!("  Commit: {commit}"),
        ],
    }
}

fn integration_lines(report: &DebugReport, owner: &ReportOwner) -> Vec<String> {
    let mut lines = vec![
        format!("afirma://: {}", handler_state(&report.protocol_handler)),
        format!(
            "Biblioteca nativa: {}",
            library_state(report.native_library.status)
        ),
    ];
    if let Some(path) = &report.native_library.path {
        lines.push(format!("  ruta: {}", owner.anonymized(path)));
    }
    for (variable, value) in [
        ("RFIRMA_LIB_DIR", &report.library_directory_override),
        ("RFIRMA_PKCS11_MODULE", &report.pkcs11_module_override),
    ] {
        if let Some(path) = value {
            lines.push(format!("{variable}: {}", owner.anonymized(path)));
        }
    }
    lines
}

fn handler_state(handler: &ProtocolHandlerStatus) -> String {
    match handler {
        ProtocolHandlerStatus::NotQueryableFromTheSandbox => {
            "no consultable · sandbox de flatpak".to_owned()
        }
        ProtocolHandlerStatus::NoneRegistered => "ninguno registrado".to_owned(),
        ProtocolHandlerStatus::Registered(id) => id.clone(),
    }
}

fn library_state(status: NativeLibraryStatus) -> &'static str {
    match status {
        NativeLibraryStatus::Loaded => "cargada",
        NativeLibraryStatus::NotFound => "no encontrada",
        NativeLibraryStatus::IncompatibleSymbols => "no carga · símbolos incompatibles",
        NativeLibraryStatus::NotLoadable => "no carga",
    }
}

const SYSTEM_DIRECTORIES: [&str; 3] = ["/usr", "/etc", "/app"];
const RUNTIME_DIRECTORY: &str = "/run/user";

impl ReportOwner {
    /// La ruta sin el home ni el nombre de usuario; las del sistema, tal cual.
    pub fn anonymized(&self, path: &Path) -> String {
        if SYSTEM_DIRECTORIES
            .iter()
            .any(|directory| path.starts_with(directory))
        {
            return path.display().to_string();
        }
        let shortened = self
            .relative_to_home(path)
            .or_else(|| relative_to_runtime_directory(path))
            .unwrap_or_else(|| path.display().to_string());
        if self.user_name.is_empty() {
            return shortened;
        }
        shortened.replace(&self.user_name, "<usuario>")
    }

    fn relative_to_home(&self, path: &Path) -> Option<String> {
        self.home.parent()?;
        with_prefix_replaced(path, &self.home, "~")
    }
}

fn relative_to_runtime_directory(path: &Path) -> Option<String> {
    let uid = path
        .strip_prefix(RUNTIME_DIRECTORY)
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?;
    if !uid.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let own_directory = format!("{RUNTIME_DIRECTORY}/{uid}");
    with_prefix_replaced(path, Path::new(&own_directory), "$XDG_RUNTIME_DIR")
}

fn with_prefix_replaced(path: &Path, prefix: &Path, label: &str) -> Option<String> {
    path.strip_prefix(prefix).ok()?;
    let text = path.display().to_string();
    let prefix_text = prefix.display().to_string();
    let tail = text.strip_prefix(prefix_text.trim_end_matches(['/', '\\']))?;
    Some(format!("{label}{tail}"))
}

mod pkcs11;
mod stores;

#[cfg(test)]
mod stores_tests;
#[cfg(test)]
mod tests;
