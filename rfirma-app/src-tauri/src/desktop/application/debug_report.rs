//! El informe de `--debug-info`: el texto que se pega en un informe de fallo, formateado desde un `DebugReport` ya relleno y sin ninguna ruta del equipo.

use crate::site::domain::protocol::IMPLEMENTED_AUTOFIRMA_VERSION;

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

/// Todo lo que el informe cuenta, recogido ya del entorno.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugReport {
    /// Versión de rFirma.
    pub version: String,
    /// Canal de distribución.
    pub channel: String,
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
    pub native_library: NativeLibraryStatus,
}

/// Una clave y un valor por línea, en castellano.
pub fn debug_report_text(report: &DebugReport) -> String {
    let mut lines = vec![
        format!("Versión de rFirma: {}", report.version),
        format!("Compatible con AutoFirma: {IMPLEMENTED_AUTOFIRMA_VERSION}"),
        format!("Canal de instalación: {}", report.channel),
        format!("Sistema operativo: {}", report.operating_system),
        format!("Arquitectura: {}", report.architecture),
    ];
    if let Some(linux) = &report.linux {
        lines.push(format!("Distribución: {}", linux.distribution));
        lines.push(format!("Entorno de escritorio: {}", linux.desktop));
        lines.push(format!("Tipo de sesión: {}", linux.session));
    }
    lines.push(format!("Idioma del sistema: {}", report.locale));
    lines.push(format!(
        "Manejador de afirma://: {}",
        handler_sentence(&report.protocol_handler)
    ));
    lines.push(format!(
        "Biblioteca nativa: {}",
        library_sentence(report.native_library)
    ));
    lines.join("\n")
}

fn handler_sentence(handler: &ProtocolHandlerStatus) -> String {
    match handler {
        ProtocolHandlerStatus::NotQueryableFromTheSandbox => {
            "no se puede consultar desde el sandbox de flatpak".to_owned()
        }
        ProtocolHandlerStatus::NoneRegistered => "ninguno registrado".to_owned(),
        ProtocolHandlerStatus::Registered(id) => id.clone(),
    }
}

fn library_sentence(status: NativeLibraryStatus) -> &'static str {
    match status {
        NativeLibraryStatus::Loaded => "cargada",
        NativeLibraryStatus::NotFound => "no encontrada",
        NativeLibraryStatus::IncompatibleSymbols => "símbolos incompatibles",
        NativeLibraryStatus::NotLoadable => "encontrada, pero no se puede cargar",
    }
}

#[cfg(test)]
mod tests;
