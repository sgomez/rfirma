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

#[cfg(test)]
mod tests {
    use super::*;

    fn from_rv(rv: RvError) -> TokenError {
        TokenError::from(Error::Pkcs11(rv, Function::Login))
    }

    #[test]
    fn the_named_codes_map_to_distinct_situations() {
        let situations = [
            from_rv(RvError::PinIncorrect).situation(),
            from_rv(RvError::PinLocked).situation(),
            from_rv(RvError::TokenNotPresent).situation(),
            from_rv(RvError::SessionHandleInvalid).situation(),
        ];

        assert_eq!(
            situations,
            [
                Situation::IncorrectPin,
                Situation::PinLocked,
                Situation::TokenAbsent,
                Situation::ExpiredSession,
            ]
        );
    }

    #[test]
    fn every_mapped_code_keeps_its_raw_ckr_apart_and_untranslated() {
        for (rv, expected) in [
            (RvError::PinIncorrect, "CKR_PIN_INCORRECT"),
            (RvError::PinLocked, "CKR_PIN_LOCKED"),
            (RvError::TokenNotPresent, "CKR_TOKEN_NOT_PRESENT"),
            (RvError::DeviceRemoved, "CKR_DEVICE_REMOVED"),
            (RvError::SessionHandleInvalid, "CKR_SESSION_HANDLE_INVALID"),
        ] {
            let error = from_rv(rv);
            assert_eq!(error.ckr(), Some(expected));
            assert!(
                error.detail().starts_with(expected),
                "el detalle deberia empezar por el codigo crudo: {}",
                error.detail()
            );
        }
    }

    #[test]
    fn an_unknown_code_falls_back_to_the_generic_situation_and_still_shows_itself() {
        let error = from_rv(RvError::UnknownErrorCode(0x0ded));

        assert_eq!(error.situation(), Situation::Unknown);
        assert_eq!(error.ckr(), Some("CKR_UNKNOWN(0xded)"));
        assert!(error.detail().contains("0xded"));
    }

    #[test]
    fn a_vendor_code_also_shows_itself_instead_of_disappearing() {
        let error = from_rv(RvError::VendorDefined(0x8000_0042));

        assert_eq!(error.situation(), Situation::Unknown);
        assert_eq!(error.ckr(), Some("CKR_VENDOR_DEFINED+0x80000042"));
    }

    #[test]
    fn the_detail_names_the_pkcs11_function_that_failed() {
        assert_eq!(
            from_rv(RvError::PinIncorrect).detail(),
            "CKR_PIN_INCORRECT (C_Login)"
        );
    }

    #[test]
    fn a_failure_of_ours_carries_no_ckr_but_still_carries_a_detail() {
        let error = TokenError::new(Situation::CertificateNotFound, "no hay ninguna etiqueta X");

        assert_eq!(error.ckr(), None);
        assert!(!error.detail().is_empty());
        assert!(error.to_string().contains("CertificateNotFound"));
    }
}
