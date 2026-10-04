//! Canal de distribución en el que corre el proceso (ADR-0015).

/// Canal de distribución en el que corre el proceso (ADR-0015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Instalación nativa sin aislamiento (.deb o .rpm).
    Native,
    /// Instalación en contenedor flatpak.
    Flatpak,
    /// Instalación de Windows, servida desde el servidor propio.
    Windows,
}

impl Channel {
    /// Nombre del canal tal como lo ve la persona usuaria.
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "nativo",
            Self::Flatpak => "flatpak",
            Self::Windows => "windows",
        }
    }

    /// Si la versión anunciada se puede instalar desde la aplicación.
    pub fn installs_from_the_app(self) -> bool {
        self == Self::Windows
    }
}
