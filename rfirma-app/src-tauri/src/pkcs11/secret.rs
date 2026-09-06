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
mod tests {
    use super::*;

    #[test]
    fn a_store_that_asks_for_no_session_needs_no_secret() {
        assert_eq!(StoreSecret::of_token(false, false), StoreSecret::NotNeeded);
    }

    #[test]
    fn a_store_that_asks_for_no_session_needs_no_secret_even_with_a_keypad() {
        assert_eq!(StoreSecret::of_token(false, true), StoreSecret::NotNeeded);
    }

    #[test]
    fn a_store_that_asks_for_a_session_has_its_secret_typed_on_screen() {
        assert_eq!(
            StoreSecret::of_token(true, false),
            StoreSecret::TypedOnScreen {
                attempts_left: None
            }
        );
    }

    #[test]
    fn a_reader_with_its_own_keypad_is_told_apart_from_the_screen() {
        assert_eq!(
            StoreSecret::of_token(true, true),
            StoreSecret::TypedOnTheReaderKeypad
        );
    }

    #[test]
    fn the_attempts_left_are_empty_because_pkcs11_never_counts_them() {
        let StoreSecret::TypedOnScreen { attempts_left } = StoreSecret::of_token(true, false)
        else {
            panic!("un almacen con sesion y sin teclado pide el secreto por pantalla");
        };
        assert_eq!(attempts_left, None);
    }

    #[test]
    fn the_two_secrets_that_can_be_asked_for_are_admitted() {
        assert_eq!(
            StoreSecret::NotNeeded.admitted(),
            Ok(StoreSecret::NotNeeded)
        );
        let on_screen = StoreSecret::TypedOnScreen {
            attempts_left: None,
        };
        assert_eq!(on_screen.admitted(), Ok(on_screen));
    }

    #[test]
    fn the_secret_of_a_reader_keypad_is_refused_instead_of_being_asked_on_screen() {
        assert_eq!(
            StoreSecret::TypedOnTheReaderKeypad.admitted(),
            Err(SecretOnTheReaderKeypad)
        );
    }

    #[test]
    fn the_refusal_names_its_own_situation_and_says_why() {
        assert_eq!(
            SecretOnTheReaderKeypad.situation(),
            "secretOnTheReaderKeypad"
        );
        assert!(SecretOnTheReaderKeypad
            .to_string()
            .contains("teclado del lector"));
    }
}
