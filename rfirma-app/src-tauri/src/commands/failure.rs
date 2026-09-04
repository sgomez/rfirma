//! **Cómo se le cuenta a la ventana que algo ha salido mal.**
//!
//! Un fallo no cruza como el error del módulo que lo produjo: cruza como un
//! [`Failure`], que es la forma del ID-29 —una **situación** nuestra, en
//! `camelCase`, que el catálogo traduce a cada idioma, y el texto original
//! **crudo** al lado— y la misma que ya tiene `TokenFailure` en TypeScript.
//!
//! Aquí viven el tipo, las conversiones desde cada error de dominio y los
//! nombres en `camelCase` con los que el catálogo los traduce (ID-80). Va
//! aparte de [`super::views`] por tamaño: son dos ficheros porque ninguno de
//! los dos puede pasar de 400 líneas, no porque sean dos cosas distintas.

use serde::Serialize;

use crate::app::cycle;
use crate::destination::DestinationError;
use crate::ffi::BridgeError;
use crate::isolate::IsolateGone;
use crate::memory::{MemoryError, Situation as MemorySituation};
use crate::pkcs11::{SecretOnTheReaderKeypad, Situation, TokenError};
use crate::rubric::{RubricError, Situation as RubricSituation};
use crate::signing::{Refusal, SealMismatch};

/// Lo que la ventana recibe cuando algo sale mal.
///
/// Es la forma del ID-29 y la misma que ya tiene `TokenFailure` en TypeScript:
/// una **situación** nuestra, que el catálogo traduce a cada idioma, y el
/// texto original **crudo** al lado, sin traducir ni recortar, para poder
/// pegarlo en un informe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    /// El nombre de la situación, en `camelCase`, tal cual lo espera la unión
    /// de TypeScript.
    pub situation: String,
    /// El detalle crudo. Nunca vacío.
    pub detail: String,
    /// Cuántos intentos de PIN quedan, cuando el módulo lo dice.
    pub attempts_left: Option<u32>,
}

impl Failure {
    /// Un fallo con su situación y su detalle.
    ///
    /// Es público porque los casos de uso de [`crate::app`] fracasan con este
    /// tipo: el ID-79 les deja decidir **que** algo no se puede hacer, y a este
    /// módulo, cómo se le cuenta a la ventana.
    pub fn new(situation: &str, detail: impl Into<String>) -> Self {
        Self {
            situation: situation.to_owned(),
            detail: detail.into(),
            attempts_left: None,
        }
    }
}

/// El nombre en `camelCase` de una situación del token, que es la clave con la
/// que el catálogo la traduce.
pub fn situation_name(situation: Situation) -> &'static str {
    match situation {
        Situation::IncorrectPin => "incorrectPin",
        Situation::PinLocked => "pinLocked",
        Situation::TokenAbsent => "tokenAbsent",
        Situation::ExpiredSession => "expiredSession",
        Situation::ModuleNotFound => "moduleNotFound",
        Situation::CertificateNotFound => "certificateNotFound",
        Situation::Pkcs12Unreadable => "pkcs12Unreadable",
        Situation::KeyNotRsa => "keyNotRsa",
        Situation::Unknown => "unknown",
    }
}

impl From<TokenError> for Failure {
    fn from(error: TokenError) -> Self {
        Self::new(situation_name(error.situation()), error.detail())
    }
}

impl From<SecretOnTheReaderKeypad> for Failure {
    fn from(refusal: SecretOnTheReaderKeypad) -> Self {
        Self::new(refusal.situation(), refusal.to_string())
    }
}

impl From<MemoryError> for Failure {
    fn from(error: MemoryError) -> Self {
        let situation = match error.situation() {
            MemorySituation::Unreadable => "settingsUnreadable",
            MemorySituation::Unwritable => "settingsUnwritable",
        };
        Self::new(situation, error.detail().to_owned())
    }
}

impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        Self::new(refusal.situation(), refusal.to_string())
    }
}

impl From<SealMismatch> for Failure {
    fn from(error: SealMismatch) -> Self {
        Self::new("sealMismatch", error.to_string())
    }
}

impl From<BridgeError> for Failure {
    fn from(error: BridgeError) -> Self {
        Self::new("bridgeFailed", error.to_string())
    }
}

impl From<IsolateGone> for Failure {
    fn from(error: IsolateGone) -> Self {
        Self::new("bridgeFailed", error.to_string())
    }
}

/// El nombre en `camelCase` de una situación de la rúbrica.
fn rubric_situation_name(situation: RubricSituation) -> &'static str {
    match situation {
        RubricSituation::NotAnAcceptedImage => "notAnAcceptedImage",
        RubricSituation::DamagedImage => "damagedImage",
        RubricSituation::ImageTooLarge => "imageTooLarge",
        RubricSituation::SourceUnreadable => "sourceUnreadable",
        RubricSituation::StoreUnwritable => "storeUnwritable",
        RubricSituation::StoreUnreadable => "storeUnreadable",
    }
}

impl From<&RubricError> for Failure {
    fn from(error: &RubricError) -> Self {
        Self::new(rubric_situation_name(error.situation()), error.detail())
    }
}

impl From<RubricError> for Failure {
    fn from(error: RubricError) -> Self {
        Self::from(&error)
    }
}

/// El nombre en `camelCase` de una situación del destino.
fn destination_situation_name(situation: crate::destination::Situation) -> &'static str {
    use crate::destination::Situation as Where;
    match situation {
        Where::FolderMissing => "folderMissing",
        Where::NotAFolder => "notAFolder",
        Where::FolderUnreadable => "folderUnreadable",
        Where::NoFreeName => "noFreeName",
    }
}

impl From<DestinationError> for Failure {
    fn from(error: DestinationError) -> Self {
        Self::new(
            destination_situation_name(error.situation()),
            error.detail(),
        )
    }
}

impl From<cycle::CycleError> for Failure {
    fn from(error: cycle::CycleError) -> Self {
        match error {
            cycle::CycleError::Inadmissible(refusal) => refusal.into(),
            cycle::CycleError::Bridge(error) => error.into(),
            cycle::CycleError::Token(error) => error.into(),
            cycle::CycleError::Seal(error) => error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{situation_name, Failure};
    use crate::pkcs11::Situation;

    #[test]
    fn every_token_situation_has_a_camel_case_name_for_the_catalogue() {
        let all = [
            Situation::IncorrectPin,
            Situation::PinLocked,
            Situation::TokenAbsent,
            Situation::ExpiredSession,
            Situation::ModuleNotFound,
            Situation::CertificateNotFound,
            Situation::Pkcs12Unreadable,
            Situation::KeyNotRsa,
            Situation::Unknown,
        ];
        for situation in all {
            let name = situation_name(situation);
            assert!(!name.is_empty());
            assert!(
                !name.contains('_') && name.chars().next().is_some_and(char::is_lowercase),
                "«{name}» no está en camelCase"
            );
        }
    }

    #[test]
    fn a_failure_keeps_the_raw_detail_of_the_token() {
        let failure: Failure = crate::pkcs11::TokenError::new(
            Situation::CertificateNotFound,
            "el token no tiene ninguna clave privada etiquetada X",
        )
        .into();

        assert_eq!(failure.situation, "certificateNotFound");
        assert_eq!(
            failure.detail,
            "el token no tiene ninguna clave privada etiquetada X"
        );
    }
}
