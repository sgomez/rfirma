//! **El secreto que desbloquea la clave privada de un almacén**, y cuál de las
//! tres formas de pedirlo le toca a cada uno (ID-189).
//!
//! Antes era una `String` y punto: la ventana abría el diálogo, la persona
//! tecleaba, y `C_Login` recibía lo tecleado. Eso da por sentado que **todos**
//! los almacenes piden secreto y que **todos** lo piden por pantalla, y ninguna
//! de las dos cosas es cierta. Lo que manda son dos banderas de
//! `CK_TOKEN_INFO`, medidas en `docs/research/token-flags-login.md`:
//!
//! | `CKF_LOGIN_REQUIRED` | `CKF_PROTECTED_AUTHENTICATION_PATH` | Secreto |
//! |---|---|---|
//! | `false` | — | [`StoreSecret::NotNeeded`] |
//! | `true` | `false` | [`StoreSecret::TypedOnScreen`] |
//! | `true` | `true` | [`StoreSecret::TypedOnTheReaderKeypad`] |
//!
//! # El teclado del lector se detecta y se rechaza
//!
//! La tercera fila **no se implementa** (ID-189). Un lector con teclado propio
//! quiere `C_Login(User, NULL)` y que la persona teclee en el aparato, y eso no
//! se ha podido medir contra ningún hardware: la rama sería una lectura de la
//! especificación, no un hecho. Firmarla con el secreto tecleado en pantalla
//! sería peor que no intentarlo, así que se reconoce y se rechaza con
//! [`SecretOnTheReaderKeypad`] **antes de cruzar la frontera**, y nadie teclea
//! nada para nada.
//!
//! # El contador de reintentos nace vacío y se queda vacío
//!
//! [`StoreSecret::TypedOnScreen`] lleva un contador opcional que **siempre**
//! está vacío, y eso es estructural, no provisional (ID-191): la información de
//! token de PKCS#11 no tiene contador de reintentos. Tiene tres banderas de
//! aviso —«quedan pocos», «último intento», «bloqueado»— y ni un número, ni con
//! una tarjeta delante. El tipo opcional es el correcto y no hay nada que
//! rellenar: si algún día un módulo lo dijera, cabría aquí sin cambiar la forma.

use std::fmt;

/// **Cómo se le pide a un almacén el secreto que desbloquea su clave privada.**
///
/// Sale del token, viaja en la salida de la prefirma y llega a la ventana, que
/// es quien decide si abre el diálogo o no (ID-189).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreSecret {
    /// El almacén no exige sesión: se firma directo, sin diálogo y **sin
    /// llamar a `C_Login`**.
    ///
    /// Ni PIN, ni cadena vacía, ni `NULL`. Es lo que hace SunPKCS11 y por tanto
    /// AutoFirma, que es el oráculo.
    NotNeeded,
    /// El secreto se teclea en pantalla, que es el caso corriente: un perfil de
    /// NSS con contraseña maestra, o un módulo PKCS#11 con PIN.
    TypedOnScreen {
        /// Cuántos intentos quedan, cuando el módulo lo dice. **Siempre
        /// vacío**: PKCS#11 no lo dice nunca (ID-191).
        attempts_left: Option<u32>,
    },
    /// El secreto se teclea en el teclado del propio lector, y rfirma no sabe
    /// pedirlo así. Se detecta para poder rechazarlo, no para atenderlo.
    TypedOnTheReaderKeypad,
}

impl StoreSecret {
    /// Lo que dicen las dos banderas de `CK_TOKEN_INFO` de la ranura elegida.
    ///
    /// Se toman como dos `bool` y no como el `TokenInfo` de `cryptoki` para que
    /// la regla se pueda probar sin token: es una tabla de verdad de dos
    /// entradas, y montar un SoftHSM para comprobarla no comprobaría nada más.
    pub fn of_token(login_required: bool, protected_authentication_path: bool) -> Self {
        match (login_required, protected_authentication_path) {
            (false, _) => Self::NotNeeded,
            (true, true) => Self::TypedOnTheReaderKeypad,
            (true, false) => Self::TypedOnScreen {
                attempts_left: None,
            },
        }
    }

    /// El mismo secreto, si rfirma sabe pedirlo; el rechazo, si es el del
    /// teclado del lector.
    ///
    /// Es la única puerta por la que un secreto entra en el recorrido de la
    /// firma: llamarla es lo que garantiza que no se intenta firmar contra un
    /// lector con teclado propio.
    pub fn admitted(self) -> Result<Self, SecretOnTheReaderKeypad> {
        match self {
            Self::TypedOnTheReaderKeypad => Err(SecretOnTheReaderKeypad),
            admitted => Ok(admitted),
        }
    }
}

/// El almacén pide el secreto por el teclado del lector, que es lo único que
/// rfirma no sabe hacer (ID-189).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecretOnTheReaderKeypad;

impl SecretOnTheReaderKeypad {
    /// El nombre de la situación en el catálogo, que es lo que cruza a la
    /// ventana.
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

    /// **Grada A**: es una tabla de verdad de dos banderas. Sin token, sin
    /// librería nativa y sin red.
    #[test]
    fn a_store_that_asks_for_no_session_needs_no_secret() {
        assert_eq!(StoreSecret::of_token(false, false), StoreSecret::NotNeeded);
    }

    /// Y sigue sin necesitarlo aunque el lector anuncie teclado propio: sin
    /// sesión no hay nada que desbloquear, así que no hay dónde teclearlo.
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

    /// El contador está vacío porque no hay de dónde sacarlo, y no porque
    /// falte escribirlo (ID-191).
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

    /// El rechazo lleva su propia situación y su propia frase: no se cuela por
    /// «cualquier otra cosa» del catálogo del token.
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
