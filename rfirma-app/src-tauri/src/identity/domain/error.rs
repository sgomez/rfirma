//! Clasificación de errores del token PKCS#11 (ADR-0009).

use cryptoki::context::Function;
use cryptoki::error::{Error, RvError};

/// Situación interpretable por el usuario que el catálogo traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El PIN no coincide con el del token.
    IncorrectPin,
    /// El token ha bloqueado el PIN tras demasiados intentos fallidos.
    PinLocked,
    /// No se detecta el token criptográfico.
    TokenAbsent,
    /// La sesión con el token ha caducado.
    ExpiredSession,
    /// No se ha podido cargar el módulo PKCS#11.
    ModuleNotFound,
    /// No se ha encontrado el certificado indicado.
    CertificateNotFound,
    /// El almacén PKCS#12 no se ha podido leer o la clave es incorrecta.
    Pkcs12Unreadable,
    /// El certificado no contiene una clave RSA compatible.
    KeyNotRsa,
    /// Error no clasificado con código crudo.
    Unknown,
}

/// Fallo en la comunicación con el token criptográfico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenError {
    situation: Situation,
    ckr: Option<String>,
    detail: String,
}

impl TokenError {
    /// Construye un error no originado directamente en un código PKCS#11.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            ckr: None,
            detail: detail.into(),
        }
    }

    /// Situación clasificada para la interfaz.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// Código `CKR_*` original cuando el fallo procede del token.
    pub fn ckr(&self) -> Option<&str> {
        self.ckr.as_deref()
    }

    /// Detalle técnico crudo sin traducir.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for TokenError {}

impl From<Error> for TokenError {
    fn from(error: Error) -> Self {
        match error {
            Error::Pkcs11(rv, function) => {
                let ckr = ckr_name(rv);
                Self {
                    situation: classify(rv),
                    detail: format!("{ckr} ({})", function_name(function)),
                    ckr: Some(ckr),
                }
            }
            Error::LibraryLoading(e) => Self::new(Situation::ModuleNotFound, e.to_string()),
            other => Self::new(Situation::Unknown, other.to_string()),
        }
    }
}

fn classify(rv: RvError) -> Situation {
    match rv {
        RvError::PinIncorrect => Situation::IncorrectPin,
        RvError::PinLocked => Situation::PinLocked,
        RvError::TokenNotPresent | RvError::DeviceRemoved => Situation::TokenAbsent,
        RvError::SessionHandleInvalid | RvError::SessionClosed => Situation::ExpiredSession,
        _ => Situation::Unknown,
    }
}

fn ckr_name(rv: RvError) -> String {
    match rv {
        RvError::VendorDefined(code) => format!("CKR_VENDOR_DEFINED+{code:#x}"),
        RvError::UnknownErrorCode(code) => format!("CKR_UNKNOWN({code:#x})"),
        named => {
            let mut out = String::from("CKR");
            for character in format!("{named:?}").chars() {
                if character.is_ascii_uppercase() {
                    out.push('_');
                }
                out.push(character.to_ascii_uppercase());
            }
            out
        }
    }
}

fn function_name(function: Function) -> String {
    format!("C_{function:?}")
}

/// Detalle del error cuando la biblioteca `libnss3.so` no está disponible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NssUnavailable {
    detail: String,
}

impl NssUnavailable {
    /// Construye un error con el detalle correspondiente.
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Detalle del motivo por el que la biblioteca no está disponible.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for NssUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for NssUnavailable {}

#[cfg(test)]
mod tests;
