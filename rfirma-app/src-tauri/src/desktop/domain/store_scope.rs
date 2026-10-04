//! A qué almacenes acota `-store` la búsqueda de certificados, con los nombres de la línea de órdenes (ADR-0022, ADR-0041); no abre ninguno.

use std::fmt;

use crate::desktop::domain::platform::Platform;

use super::command_line::{documented, STORE};

const AUTO: &str = "auto";
const MOZILLA: &str = "mozilla";
const PKCS11: &str = "pkcs11";
const WINDOWS: &str = "windows";
const NOT_SUPPORTED: [&str; 5] = ["pkcs12", "dni", "dnie", WINDOWS, "mac"];
const NOT_SUPPORTED_ON_WINDOWS: [&str; 4] = ["pkcs12", "dni", "dnie", "mac"];

/// Dónde se buscan los certificados.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum StoreScope {
    /// En todos los almacenes.
    #[default]
    Everywhere,
    /// Solo en la familia NSS: los navegadores, el del sistema y el Almacén de rFirma.
    Nss,
    /// Solo en el almacén de Windows.
    Windows,
    /// Solo en este módulo PKCS#11, si es uno ya descubierto.
    Module(String),
}

/// Por qué un valor de `-store` no se atiende.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreRefusal {
    /// `-store` sin valor.
    Missing,
    /// Un almacén del original que rFirma no abre.
    NotSupported(String),
    /// Un nombre que el original no reconoce.
    Unknown(String),
    /// `pkcs11` sin la ruta del módulo.
    ModuleWithoutPath,
}

impl fmt::Display for StoreRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => write!(
                formatter,
                "{} necesita el nombre de un almacén",
                documented(STORE)
            ),
            Self::NotSupported(name) => {
                write!(formatter, "rfirma no soporta el almacén «{name}»")
            }
            Self::Unknown(name) => write!(formatter, "«{name}» no es un almacén conocido"),
            Self::ModuleWithoutPath => {
                write!(
                    formatter,
                    "el almacén pkcs11 necesita la ruta: pkcs11:<ruta>"
                )
            }
        }
    }
}

/// El ámbito que pide el valor de `-store`, o todos los almacenes si la orden no lo lleva.
pub fn scope_named_by(
    arguments: &[String],
    platform: Platform,
) -> Result<StoreScope, StoreRefusal> {
    let Some(position) = arguments.iter().position(|argument| argument == STORE) else {
        return Ok(StoreScope::Everywhere);
    };
    let value = arguments.get(position + 1).ok_or(StoreRefusal::Missing)?;
    scope_of(value, platform)
}

fn scope_of(value: &str, platform: Platform) -> Result<StoreScope, StoreRefusal> {
    let (name, library) = match value.split_once(':') {
        Some((name, library)) => (name, Some(library.trim())),
        None => (value, None),
    };
    let lowered = name.trim().to_ascii_lowercase();
    let on_windows = platform == Platform::Windows;
    match (lowered.as_str(), library) {
        (AUTO | WINDOWS, None) if on_windows => Ok(StoreScope::Windows),
        (AUTO | MOZILLA, None) if !on_windows => Ok(StoreScope::Nss),
        (PKCS11, Some(library)) if !library.is_empty() => {
            Ok(StoreScope::Module(library.to_owned()))
        }
        (PKCS11, _) => Err(StoreRefusal::ModuleWithoutPath),
        (name, _)
            if on_windows && (name == MOZILLA || NOT_SUPPORTED_ON_WINDOWS.contains(&name)) =>
        {
            Err(StoreRefusal::NotSupported(value.to_owned()))
        }
        (name, _) if name == AUTO || name == MOZILLA || NOT_SUPPORTED.contains(&name) => {
            Err(StoreRefusal::NotSupported(value.to_owned()))
        }
        _ => Err(StoreRefusal::Unknown(value.to_owned())),
    }
}

#[cfg(test)]
mod tests;
