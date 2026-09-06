//! Solicitud y modalidades del secreto de acceso a almacenes PKCS#11.

use std::fmt;

/// Modalidad de solicitud del secreto que desbloquea la clave privada de un almacén.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreSecret {
    /// El almacén no exige autenticación previa.
    NotNeeded,
    /// El secreto se introduce interactivamente por pantalla.
    TypedOnScreen {
        /// Intentos restantes si el módulo los proporciona.
        attempts_left: Option<u32>,
    },
    /// El secreto se introduce en el teclado físico del lector.
    TypedOnTheReaderKeypad,
}

impl StoreSecret {
    /// Determina la modalidad a partir de las banderas del token.
    pub fn of_token(login_required: bool, protected_authentication_path: bool) -> Self {
        match (login_required, protected_authentication_path) {
            (false, _) => Self::NotNeeded,
            (true, true) => Self::TypedOnTheReaderKeypad,
            (true, false) => Self::TypedOnScreen {
                attempts_left: None,
            },
        }
    }

    /// Valida que la modalidad de secreto esté admitida para la firma.
    pub fn admitted(self) -> Result<Self, SecretOnTheReaderKeypad> {
        match self {
            Self::TypedOnTheReaderKeypad => Err(SecretOnTheReaderKeypad),
            admitted => Ok(admitted),
        }
    }
}

/// Rechazo emitido cuando el almacén requiere introducción de PIN en el lector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecretOnTheReaderKeypad;

impl SecretOnTheReaderKeypad {
    /// Clave identificadora de la situación en el catálogo de errores.
    pub fn situation(self) -> &'static str {
        "secretOnTheReaderKeypad"
    }
}

impl fmt::Display for SecretOnTheReaderKeypad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "el secreto de este almacen se teclea en el teclado del lector, \
             y rfirma todavia no sabe pedirlo asi",
        )
    }
}

impl std::error::Error for SecretOnTheReaderKeypad {}

#[cfg(test)]
mod tests;
