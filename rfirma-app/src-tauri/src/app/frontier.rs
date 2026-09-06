//! **La frontera de errores**: el único sitio donde una situación nuestra
//! (ID-29) se convierte en lo que la sede recibe (ID-288).
//!
//! Dentro se sigue razonando con situaciones —`incorrectPin`, `notAPdf`,
//! `folderMissing`—, que es lo que el catálogo de cadenas traduce a los cinco
//! idiomas y lo que la ventana enseña (ADR-0009). Fuera, hacia la sede, sólo
//! sale un [`WireAnswer`]: un código del catálogo publicado con frase nuestra,
//! `CANCEL`, `MEMORY_ERROR` o `NULL`, y nada más (ID-289).
//!
//! Tres reglas sostienen este módulo:
//!
//! 1. **Aquí se traduce, y en ningún otro sitio.** Ningún módulo construye un
//!    [`WireAnswer`] a partir de un error suyo: lo trae aquí y recoge el
//!    código. Lo único que se salta el paso son los rechazos del propio
//!    protocolo ([`crate::protocol::Refusal`]), que **nacen ya con su código**
//!    porque el código *es* su situación.
//! 2. **El código lo manda la verdad de la situación, no lo que emitiría
//!    AutoFirma** (ID-292): un PDF cifrado es `SAF_33` y uno certificado
//!    `SAF_35`, aunque el original despache los dos con el `SAF_28` que lanza
//!    desde `sign`.
//! 3. **Ningún rechazo nuestro es una cancelación** (ID-293): `CANCEL` es la
//!    persona diciendo que no, y nada más. Lo vigila una prueba.
//!
//! Y el detalle crudo de la situación **no cruza** (ID-291): estas funciones
//! reciben la situación, no el error, así que no hay por dónde colar una ruta,
//! un nombre de documento, un certificado ni los intentos de PIN que quedan.
//! Lo comprueba campo a campo [`super::guards`].

use crate::app::cycle::CycleError;
use crate::app::signing::CycleFailure;
use crate::channel::Situation as ChannelSituation;
use crate::destination::Situation as DestinationSituation;
use crate::ffi::BridgeError;
use crate::memory::Situation as MemorySituation;
use crate::pkcs11::Situation as TokenSituation;
use crate::protocol::{SafCode, WireAnswer};
use crate::rubric::Situation as RubricSituation;
use crate::signing::Refusal as Inadmissible;

/// El código de una situación del token.
///
/// El almacén de claves del original es aquí el token PKCS#11, así que las
/// negativas de acceso van a `SAF_08` y el bloqueo a `SAF_52`, que es lo que el
/// lanzador emite desde `ProtocolInvocationLauncherSign`.
pub fn code_of_token(situation: TokenSituation) -> SafCode {
    match situation {
        TokenSituation::IncorrectPin
        | TokenSituation::ExpiredSession
        | TokenSituation::Pkcs12Unreadable
        | TokenSituation::Unknown => SafCode::CannotAccessKeystore,
        TokenSituation::PinLocked => SafCode::LockedKeystore,
        TokenSituation::TokenAbsent | TokenSituation::ModuleNotFound => SafCode::CannotFindKeystore,
        TokenSituation::CertificateNotFound => SafCode::NoCertificatesInKeystore,
        TokenSituation::KeyNotRsa => SafCode::IncompatibleKeyType,
    }
}

/// El código de una situación de la memoria entre sesiones.
///
/// Lo que la sede necesita saber no es qué fichero nuestro falló, sino si el
/// documento se pudo leer o guardar.
pub fn code_of_memory(situation: MemorySituation) -> SafCode {
    match situation {
        MemorySituation::Unreadable => SafCode::CannotReadData,
        MemorySituation::Unwritable => SafCode::CannotSaveData,
    }
}

/// El código de una situación del destino (ADR-0011).
///
/// Las cuatro son lo mismo visto desde la sede: el firmado no se ha podido
/// guardar.
pub fn code_of_destination(situation: DestinationSituation) -> SafCode {
    match situation {
        DestinationSituation::FolderMissing
        | DestinationSituation::NotAFolder
        | DestinationSituation::FolderUnreadable
        | DestinationSituation::NoFreeName => SafCode::CannotSaveData,
    }
}

/// El código de una situación de la rúbrica (ADR-0012).
///
/// La rúbrica sólo existe para la firma visible, así que todas caen en el
/// código de la firma visible: la sede no tiene nada que hacer con el detalle
/// de si la imagen estaba dañada o el almacén no se pudo escribir.
pub fn code_of_rubric(situation: RubricSituation) -> SafCode {
    match situation {
        RubricSituation::NotAnAcceptedImage
        | RubricSituation::DamagedImage
        | RubricSituation::ImageTooLarge
        | RubricSituation::SourceUnreadable
        | RubricSituation::StoreUnwritable
        | RubricSituation::StoreUnreadable => SafCode::VisibleSignature,
    }
}

/// El código de una situación del canal (ID-292).
///
/// Ninguna de las tres puede contestarse *por* el canal —si el canal no se
/// abre, no hay por dónde—, pero el código existe igual: es el mismo `SAF_45`
/// con el que el original se mata, y el que sale cuando el canal se cae con la
/// conversación ya empezada.
pub fn code_of_channel(situation: ChannelSituation) -> SafCode {
    match situation {
        ChannelSituation::NoDrawnPortIsFree | ChannelSituation::NotListening => {
            SafCode::CannotOpenSocket
        }
        ChannelSituation::MaterialNotUsable => SafCode::CannotAccessSslKeystore,
    }
}

/// El código de un documento que no se puede firmar (ID-292).
///
/// Aquí es donde rFirma se aparta del original a sabiendas: el original sólo
/// sabe emitir `SAF_28` desde `sign`, y estas tres situaciones son tres cosas
/// distintas que la sede puede contarle a la persona.
pub fn code_of_inadmissible(refusal: Inadmissible) -> SafCode {
    match refusal {
        Inadmissible::NotAPdf => SafCode::InvalidPdf,
        Inadmissible::Encrypted => SafCode::PdfWrongPassword,
        Inadmissible::Certified => SafCode::PdfCertified,
    }
}

/// El código de un fallo del puente.
///
/// Uno solo se distingue del resto, y por eso el puente lo distingue
/// (ID-296): un PDF con firmas no registradas **no es un fallo**, es una
/// operación que puede generar firmas no válidas y que la sede tiene que
/// confirmar, que es exactamente lo que dice `SAF_50`. Todo lo demás es la
/// firma que no ha salido.
///
/// El resto se enumera en un brazo compartido en vez de colapsarlo en un `_`:
/// colapsarlo es la decisión correcta —todas son la firma que no sale—, pero
/// con el comodín una variante nueva de [`BridgeError`] se traduciría sola a
/// `SAF_09` sin que nada se pusiera rojo, y la regla del módulo es que una
/// situación nueva no compile hasta que se le decida código.
pub fn code_of_bridge(error: &BridgeError) -> SafCode {
    match error {
        BridgeError::PdfHasUnregisteredSignatures(_) => SafCode::ConfirmationNeeded,
        // La sede declaró una política que no se puede aplicar: no es la firma
        // que no sale, es lo que ella pidió, y el catálogo tiene código para
        // eso (ID-266).
        BridgeError::IncompatiblePolicy(_) => SafCode::InvalidPolicy,
        BridgeError::ExecutablePathUnknown(_)
        | BridgeError::NotFound(_)
        | BridgeError::Load { .. }
        | BridgeError::MissingSymbol { .. }
        | BridgeError::IsolateFailed(_)
        | BridgeError::InvalidArgument(_)
        | BridgeError::NullResponse
        | BridgeError::MalformedResponse(_)
        | BridgeError::Failed(_) => SafCode::SignatureFailed,
    }
}

/// El sello de sesión roto entre prefirma y postfirma (ADR-0016): la firma
/// existe pero no se ha podido ensamblar.
pub fn code_of_broken_seal() -> SafCode {
    SafCode::PostprocessingData
}

/// El código de un tramo trifásico que no ha salido.
///
/// Es la traducción del trayecto de la sede que **no** puede decidirse por el
/// sitio donde se falla (ID-292): la prefirma y la postfirma fallan por cosas
/// muy distintas —el token que no está o está bloqueado, el documento que no
/// se puede firmar, la política que la sede declaró y no se puede aplicar, el
/// sello del ADR-0016 que no cuadra— y cada una tiene ya su código aquí
/// arriba. Colapsarlas todas en `SAF_09` dejaría a la sede sin poder decirle a
/// la persona qué arreglar.
///
/// Sólo dos brazos deciden código propio, y los dos son lo mismo visto de
/// cerca: un almacén al que no se ha podido llegar, y un puente que no está.
pub fn code_of_cycle(failure: &CycleFailure) -> SafCode {
    match failure {
        CycleFailure::DocumentUnreadable(_) => SafCode::CannotReadData,
        CycleFailure::Cycle(CycleError::Inadmissible(refusal)) => code_of_inadmissible(*refusal),
        CycleFailure::Cycle(CycleError::Bridge(error)) => code_of_bridge(error),
        CycleFailure::Cycle(CycleError::Token(error)) => code_of_token(error.situation()),
        CycleFailure::Cycle(CycleError::Seal(_)) => code_of_broken_seal(),
        // El secreto se teclea en el teclado del lector: desde la sede eso es
        // un almacén al que no se puede acceder.
        CycleFailure::SecretOnTheReaderKeypad(_) => SafCode::CannotAccessKeystore,
        // Sin hilo del isolate no hay puente, y sin puente no hay firma.
        CycleFailure::Gone(_) => SafCode::SignatureFailed,
    }
}

/// **La cancelación de la persona**, y el único sitio del que sale (ID-293).
pub fn cancelled() -> WireAnswer {
    WireAnswer::Cancelled
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Todas las situaciones del ID-29, cada una por su traducción: es la
    /// lista sobre la que se comprueban totalidad y exactitud (TD-57).
    fn every_code_of_ours() -> Vec<SafCode> {
        let mut codes = Vec::new();
        for situation in [
            TokenSituation::IncorrectPin,
            TokenSituation::PinLocked,
            TokenSituation::TokenAbsent,
            TokenSituation::ExpiredSession,
            TokenSituation::ModuleNotFound,
            TokenSituation::CertificateNotFound,
            TokenSituation::Pkcs12Unreadable,
            TokenSituation::KeyNotRsa,
            TokenSituation::Unknown,
        ] {
            codes.push(code_of_token(situation));
        }
        for situation in [MemorySituation::Unreadable, MemorySituation::Unwritable] {
            codes.push(code_of_memory(situation));
        }
        for situation in [
            DestinationSituation::FolderMissing,
            DestinationSituation::NotAFolder,
            DestinationSituation::FolderUnreadable,
            DestinationSituation::NoFreeName,
        ] {
            codes.push(code_of_destination(situation));
        }
        for situation in [
            RubricSituation::NotAnAcceptedImage,
            RubricSituation::DamagedImage,
            RubricSituation::ImageTooLarge,
            RubricSituation::SourceUnreadable,
            RubricSituation::StoreUnwritable,
            RubricSituation::StoreUnreadable,
        ] {
            codes.push(code_of_rubric(situation));
        }
        for situation in [
            ChannelSituation::NoDrawnPortIsFree,
            ChannelSituation::MaterialNotUsable,
            ChannelSituation::NotListening,
        ] {
            codes.push(code_of_channel(situation));
        }
        for refusal in [
            Inadmissible::NotAPdf,
            Inadmissible::Encrypted,
            Inadmissible::Certified,
        ] {
            codes.push(code_of_inadmissible(refusal));
        }
        codes.push(code_of_bridge(&BridgeError::Failed(
            "lo que dijera Java".to_owned(),
        )));
        codes.push(code_of_bridge(&BridgeError::PdfHasUnregisteredSignatures(
            "lo que dijera Java".to_owned(),
        )));
        codes.push(code_of_bridge(&BridgeError::IncompatiblePolicy(
            "lo que dijera Java".to_owned(),
        )));
        codes.push(code_of_broken_seal());
        codes
    }

    /// **Totalidad**: toda situación del ID-29 tiene código, y el compilador lo
    /// exige —cada traducción es un `match` cerrado sobre su enumerado, así que
    /// una situación nueva no compila hasta que se decide su código— (TD-57).
    ///
    /// Lo que esta prueba añade es lo que el compilador no ve: que ninguna
    /// traducción se salga del catálogo publicado.
    #[test]
    fn every_situation_of_ours_lands_inside_the_published_catalogue() {
        for code in every_code_of_ours() {
            assert!(
                SafCode::ALL.contains(&code),
                "{code:?} no esta en el catalogo publicado"
            );
            let line = WireAnswer::refused(code).on_the_wire();
            assert!(
                line.starts_with("SAF_") && line.len() > 4,
                "«{line}» no la lee el cliente publicado como un error"
            );
        }
    }

    /// **`SAF_48` no se emite nunca** (ID-295): `PdfShadowAttackException` es
    /// de `master` y la 1.9.2 no la lanza.
    #[test]
    fn the_shadow_attack_code_is_never_produced() {
        assert!(
            !every_code_of_ours().contains(&SafCode::PdfShadowAttack),
            "SAF_48 no existe en la 1.9.2 y no puede salir de aqui"
        );
    }

    /// **Ningún rechazo nuestro es una cancelación** (ID-293).
    #[test]
    fn no_refusal_of_ours_is_a_cancellation() {
        for code in every_code_of_ours() {
            let answer = WireAnswer::refused(code);
            assert_ne!(answer, WireAnswer::Cancelled);
            assert_ne!(answer.on_the_wire(), "CANCEL");
        }
        assert_eq!(cancelled().on_the_wire(), "CANCEL");
    }

    /// Y la del ID-292: el PDF cifrado y el certificado **no** son el mismo
    /// código, aunque el original los despache a los dos con `SAF_28`.
    #[test]
    fn the_three_pdf_situations_get_three_different_codes() {
        assert_eq!(
            code_of_inadmissible(Inadmissible::NotAPdf),
            SafCode::InvalidPdf
        );
        assert_eq!(
            code_of_inadmissible(Inadmissible::Encrypted),
            SafCode::PdfWrongPassword
        );
        assert_eq!(
            code_of_inadmissible(Inadmissible::Certified),
            SafCode::PdfCertified
        );
    }

    /// Y la del ID-296: las firmas no registradas piden confirmación, no son
    /// un fallo de firma.
    #[test]
    fn a_pdf_with_unregistered_signatures_asks_for_confirmation() {
        let error = BridgeError::PdfHasUnregisteredSignatures("da igual el texto".to_owned());

        assert_eq!(code_of_bridge(&error), SafCode::ConfirmationNeeded);
        assert_eq!(
            code_of_bridge(&BridgeError::Failed("otra cosa".to_owned())),
            SafCode::SignatureFailed
        );
    }

    /// **ID-292 sobre el tramo trifásico**: lo que falla después del
    /// consentimiento **no** es todo la misma cosa, y por eso no sale con el
    /// mismo código. Sin esto, quien firma sin la tarjeta puesta recibe «no se
    /// ha podido completar la firma» en vez de «no se ha podido encontrar el
    /// almacén», y la sede no puede decirle qué arreglar.
    #[test]
    fn what_breaks_after_the_consent_keeps_its_own_code() {
        for (failure, expected) in [
            (
                CycleFailure::Cycle(CycleError::Token(crate::pkcs11::TokenError::new(
                    TokenSituation::TokenAbsent,
                    "no hay tarjeta",
                ))),
                SafCode::CannotFindKeystore,
            ),
            (
                CycleFailure::Cycle(CycleError::Bridge(BridgeError::IncompatiblePolicy(
                    "la politica de la sede".to_owned(),
                ))),
                SafCode::InvalidPolicy,
            ),
            (
                CycleFailure::Cycle(CycleError::Inadmissible(Inadmissible::Encrypted)),
                SafCode::PdfWrongPassword,
            ),
            (
                CycleFailure::DocumentUnreadable("ya no esta".to_owned()),
                SafCode::CannotReadData,
            ),
            (
                CycleFailure::Cycle(CycleError::Bridge(BridgeError::Failed(
                    "lo que dijera Java".to_owned(),
                ))),
                SafCode::SignatureFailed,
            ),
        ] {
            assert_eq!(code_of_cycle(&failure), expected, "{failure:?}");
        }
    }

    /// Y el sello del ADR-0016 es el caso que más se distingue: la firma
    /// **existe** y lo que no ha salido es el ensamblado, que es lo que dice
    /// `SAF_42` y no `SAF_09`.
    #[test]
    fn a_broken_session_seal_is_not_a_signature_that_did_not_come_out() {
        let broken = CycleFailure::Cycle(CycleError::Seal(crate::signing::SealMismatch));

        assert_eq!(code_of_cycle(&broken), SafCode::PostprocessingData);
        assert_eq!(code_of_cycle(&broken), code_of_broken_seal());
    }
}
