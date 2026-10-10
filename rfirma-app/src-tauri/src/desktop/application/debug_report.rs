//! El informe de `--debug-info`: el texto por secciones que se pega en un informe de fallo, formateado desde un `DebugReport` ya relleno, con las rutas sin el home ni el usuario.

use std::path::{Path, PathBuf};

use crate::identity::domain::readers::Reader;

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
}

/// Una cabecera y una sección por bloque, con el patrón `clave: estado · detalle` y las rutas anonimizadas.
pub fn debug_report_text(report: &DebugReport, owner: &ReportOwner) -> String {
    [
        format!("rFirma {}", report.version),
        section("Sistema", &system_lines(report)),
        section("Integración", &integration_lines(report, owner)),
    ]
    .into_iter()
    .chain(
        report
            .pcsc
            .as_ref()
            .map(|pcsc| section("Lectores", &reader_lines(pcsc))),
    )
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

fn section(title: &str, lines: &[String]) -> String {
    std::iter::once(title.to_owned())
        .chain(lines.iter().map(|line| format!("  {line}")))
        .collect::<Vec<_>>()
        .join("\n")
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
        let rest = path.strip_prefix(&self.home).ok()?;
        Some(Path::new("~").join(rest).display().to_string())
    }
}

fn relative_to_runtime_directory(path: &Path) -> Option<String> {
    let mut below = path.strip_prefix(RUNTIME_DIRECTORY).ok()?.components();
    let uid = below.next()?.as_os_str().to_str()?;
    if !uid.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(
        Path::new("$XDG_RUNTIME_DIR")
            .join(below.as_path())
            .display()
            .to_string(),
    )
}

#[cfg(test)]
mod tests;
