//! El sello de sesión: una sola invariante entre prefirma y postfirma
//! (ADR-0016, ID-17).
//!
//! La postfirma PAdES **regenera el PDF entero**, así que exige recibir lo
//! mismo que la prefirma en tres cosas a la vez: los `extraParams` **efectivos**
//! —no los enviados: `PdfSessionManager` reescribe el `Properties` que recibe y
//! `PAdESTriPhaseSigner:174` no lo clona—, el instante de firma y la zona
//! horaria. Si alguna difiere, la postfirma **completa sin error** y el PDF sale
//! con `Digest Mismatch`: la firma se invalida en silencio.
//!
//! Las tres viajan juntas en un bloque que el puente compone y que rFirma
//! **transporta sin leer**. La comprobación es entonces una comparación de
//! bytes, y no hay manera de olvidarse de un campo porque no hay campos que
//! recordar.

use std::fmt;

/// Bloque opaco que devuelve la prefirma y que la postfirma exige idéntico.
///
/// **Es opaco por diseño.** Aquí no hay ni un `get`, ni un `parse`, ni un
/// campo: en cuanto Rust interpreta el sello puede reconstruirlo, y un sello
/// reconstruible no protege de nada. Si algún día hace falta un valor de
/// dentro, la respuesta es que la prefirma lo devuelva aparte.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionSeal(String);

impl SessionSeal {
    /// Recoge el sello tal y como lo devolvió la prefirma.
    pub fn from_bridge(payload: impl Into<String>) -> Self {
        Self(payload.into())
    }

    /// Devuelve el sello tal cual, para dárselo a la postfirma.
    pub fn as_bridge_payload(&self) -> &str {
        &self.0
    }

    /// Comprueba, **antes de firmar**, que el sello que va a recibir la
    /// postfirma es exactamente el que produjo la prefirma.
    pub fn verify_unchanged(&self, returned: &Self) -> Result<(), SealMismatch> {
        if self == returned {
            Ok(())
        } else {
            Err(SealMismatch)
        }
    }
}

/// `Debug` deliberadamente mudo: ni siquiera en un registro de fallos se
/// enseña el interior del sello, para que nadie se acostumbre a leerlo.
impl fmt::Debug for SessionSeal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionSeal({} bytes)", self.0.len())
    }
}

/// El sello que llega a la postfirma no es el que produjo la prefirma.
///
/// Abortar aquí es lo único que separa un error visible de un PDF firmado que
/// no valida y que nadie sabe por qué.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealMismatch;

impl fmt::Display for SealMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("el sello de sesión de la postfirma no es el de la prefirma")
    }
}

impl std::error::Error for SealMismatch {}

#[cfg(test)]
mod tests {
    use super::{SealMismatch, SessionSeal};

    /// La forma que compone el puente. Vive **solo en las pruebas**: el código
    /// de producción no sabe qué hay aquí dentro, y ese es justo el punto.
    fn seal_of(extra_params: &str, instant: &str, time_zone: &str) -> SessionSeal {
        SessionSeal::from_bridge(format!(
            r#"{{"extraParams":{extra_params},"time":"{instant}","timeZone":"{time_zone}"}}"#
        ))
    }

    fn a_seal() -> SessionSeal {
        seal_of(
            r#"{"signatureSubFilter":"ETSI.CAdES.detached"}"#,
            "2026-08-31T12:00:00",
            "Europe/Madrid",
        )
    }

    #[test]
    fn accepts_the_very_seal_the_presign_returned() {
        assert_eq!(a_seal().verify_unchanged(&a_seal()), Ok(()));
    }

    #[test]
    fn rejects_a_seal_whose_effective_extra_params_changed() {
        let changed = seal_of(
            r#"{"signatureSubFilter":"adbe.pkcs7.detached"}"#,
            "2026-08-31T12:00:00",
            "Europe/Madrid",
        );
        assert_eq!(a_seal().verify_unchanged(&changed), Err(SealMismatch));
    }

    #[test]
    fn rejects_a_seal_whose_instant_changed() {
        let changed = seal_of(
            r#"{"signatureSubFilter":"ETSI.CAdES.detached"}"#,
            "2026-08-31T12:00:01",
            "Europe/Madrid",
        );
        assert_eq!(a_seal().verify_unchanged(&changed), Err(SealMismatch));
    }

    #[test]
    fn rejects_a_seal_whose_time_zone_changed() {
        // El tercero, el que nadie esperaba (#23): el desfase entra dentro del
        // rango firmado, así que un mismo instante en otra zona invalida.
        let changed = seal_of(
            r#"{"signatureSubFilter":"ETSI.CAdES.detached"}"#,
            "2026-08-31T12:00:00",
            "Atlantic/Canary",
        );
        assert_eq!(a_seal().verify_unchanged(&changed), Err(SealMismatch));
    }

    #[test]
    fn carries_an_opaque_seal_through_untouched() {
        // Contenido que rFirma no sabe leer, y que tiene que dar igual.
        let payload = "\u{1}\u{0}no es JSON, ni falta que hace\u{7f}ñ€\n";
        let seal = SessionSeal::from_bridge(payload);
        assert_eq!(seal.as_bridge_payload(), payload);
        assert_eq!(seal.verify_unchanged(&seal.clone()), Ok(()));
    }

    #[test]
    fn does_not_show_the_inside_of_the_seal() {
        let debug = format!("{:?}", a_seal());
        assert!(!debug.contains("Europe/Madrid"), "{debug}");
        assert!(!debug.contains("signatureSubFilter"), "{debug}");
    }

    #[test]
    fn explains_the_mismatch() {
        assert_eq!(
            SealMismatch.to_string(),
            "el sello de sesión de la postfirma no es el de la prefirma"
        );
    }
}
