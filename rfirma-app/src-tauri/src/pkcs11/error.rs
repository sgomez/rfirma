//! Los errores del token **no se traducen: se clasifican** (ID-29, ADR-0009).
//!
//! `cryptoki` devuelve códigos (`CKR_PIN_INCORRECT`, `CKR_TOKEN_NOT_PRESENT`) y
//! ninguno de ellos se enseña como mensaje. Aquí se convierten en una
//! [`Situation`] —una situación *nuestra*, que el catálogo de cadenas traduce a
//! cada idioma— y el código original viaja aparte, **sin traducir**, para
//! poder pegarlo en un informe de fallo.
//!
//! Lo que no sepamos clasificar cae en [`Situation::Unknown`], que no es un
//! agujero: sigue llevando el `CKR_*` crudo, así que un código desconocido se
//! ve, se copia y se puede buscar.

use cryptoki::context::Function;
use cryptoki::error::{Error, RvError};

/// Situación que el usuario puede entender, y que el catálogo traduce.
///
/// Cinco casos con nombre propio más el genérico: son los que el recorrido de
/// firma sabe explicar y, en varios de ellos, sugerir qué hacer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El PIN no coincide con el del token. Quedan intentos.
    IncorrectPin,
    /// El token ha bloqueado el PIN tras demasiados intentos fallidos.
    PinLocked,
    /// No hay token: la tarjeta no está insertada, o se ha retirado a mitad.
    TokenAbsent,
    /// La sesión con el token ya no vale y hay que volver a abrirla.
    ExpiredSession,
    /// El módulo PKCS#11 no se ha podido cargar desde la ruta indicada.
    ModuleNotFound,
    /// El token está, pero no tiene ningún objeto con esa etiqueta.
    CertificateNotFound,
    /// Cualquier otra cosa. Enseña el código crudo y nada más.
    Unknown,
}

/// Un fallo hablando con el token: la situación traducible y el detalle crudo.
///
/// [`TokenError::detail`] nunca está vacío y **nunca** está traducido: es lo que
/// se pega en un informe de fallo. Cuando el fallo viene del propio token,
/// [`TokenError::ckr`] lo devuelve además aislado (`CKR_PIN_LOCKED`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenError {
    situation: Situation,
    ckr: Option<String>,
    detail: String,
}

impl TokenError {
    /// Un fallo que no viene de un código PKCS#11 (no encontrar la etiqueta
    /// pedida, por ejemplo).
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            ckr: None,
            detail: detail.into(),
        }
    }

    /// La situación que la interfaz enseña, ya clasificada.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// El `CKR_*` original, sin traducir, cuando el fallo viene del token.
    pub fn ckr(&self) -> Option<&str> {
        self.ckr.as_deref()
    }

    /// El detalle técnico crudo. Nunca vacío, nunca traducido.
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
            // La única familia con código PKCS#11 detrás.
            Error::Pkcs11(rv, function) => {
                let ckr = ckr_name(rv);
                Self {
                    situation: classify(rv),
                    detail: format!("{ckr} ({})", function_name(function)),
                    ckr: Some(ckr),
                }
            }
            // Cargar el .so es lo primero que se hace, así que un fallo aquí es
            // siempre «la ruta del módulo no vale», nunca un problema del token.
            Error::LibraryLoading(e) => Self::new(Situation::ModuleNotFound, e.to_string()),
            other => Self::new(Situation::Unknown, other.to_string()),
        }
    }
}

/// El mapeo del ID-29, y la razón de ser de este módulo.
fn classify(rv: RvError) -> Situation {
    match rv {
        RvError::PinIncorrect => Situation::IncorrectPin,
        RvError::PinLocked => Situation::PinLocked,
        // Retirar la tarjeta a mitad de operación es, para quien la retiró, el
        // mismo hecho que no haberla puesto.
        RvError::TokenNotPresent | RvError::DeviceRemoved => Situation::TokenAbsent,
        RvError::SessionHandleInvalid | RvError::SessionClosed => Situation::ExpiredSession,
        _ => Situation::Unknown,
    }
}

/// `RvError::PinIncorrect` → `"CKR_PIN_INCORRECT"`.
///
/// `cryptoki` no expone el nombre canónico —su `Display` es un párrafo de la
/// especificación en inglés, justo lo que el ADR-0009 prohíbe enseñar—, pero sus
/// variantes son el nombre del estándar en CamelCase, así que se recupera
/// deshaciendo la conversión. Los dos casos con carga útil se escriben aparte
/// porque su `Debug` lleva el número dentro.
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

/// `Function::Login` → `"C_Login"`, que es como lo nombra la especificación y
/// como lo buscará quien reciba el informe de fallo.
fn function_name(function: Function) -> String {
    format!("C_{function:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: es aritmética de enums, no necesita token.
    fn from_rv(rv: RvError) -> TokenError {
        TokenError::from(Error::Pkcs11(rv, Function::Login))
    }

    /// Cuatro de los cinco casos con nombre. El quinto —módulo no encontrado—
    /// no nace de un `CKR_*` sino de un `dlopen` fallido, así que lo cubre
    /// `a_module_that_is_not_there_is_not_a_token_error`, en el fichero de
    /// grada B, donde se puede provocar de verdad.
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
