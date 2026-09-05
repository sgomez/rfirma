//! Los fallos de registrar la confianza **se clasifican, no se traducen**
//! (ADR-0009), igual que los del material del canal en [`crate::tls::error`].
//!
//! Son tres situaciones y no más, porque desde fuera solo hay tres remedios
//! distintos: no está NSS, no se ha podido abrir el almacén de la persona —el
//! caso del flatpak sin el permiso del ID-228— y el certificado ha entrado pero
//! sus bits de confianza no se han escrito.
//!
//! **Ninguna de las tres para un trámite** (ID-224): quien las recibe cuenta
//! cuántos almacenes han quedado sin la CA y lo dice **al terminar**.

use std::fmt;

/// Situación que la persona puede entender, y que el catálogo traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// No está `libnss3.so`, que es quien abre un almacén NSS para escribir.
    NssMissing,
    /// El almacén NSS de la persona no se ha podido abrir en lectura y
    /// escritura. Es lo que pasa en el flatpak cuando falta el permiso del
    /// ID-228, y también cuando el perfil está a medio crear.
    StoreUnreachable,
    /// El certificado de la CA local ha entrado en el almacén, pero sus bits de
    /// confianza no se han podido escribir. Sin ellos el navegador no confía,
    /// así que **no cuenta como instalado**.
    TrustNotWritten,
}

/// Un fallo al registrar la CA local: la situación traducible y el detalle
/// crudo.
///
/// [`TrustError::detail`] nunca está vacío y **nunca** está traducido: es lo
/// que se pega en un informe de fallo, con el nombre de la llamada de NSS que
/// falló dentro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustError {
    situation: Situation,
    detail: String,
}

impl TrustError {
    /// Un fallo con su detalle técnico, sin traducir.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// La situación que la interfaz enseña, ya clasificada.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// El detalle técnico crudo. Nunca vacío, nunca traducido.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for TrustError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = TrustError::new(Situation::StoreUnreachable, "SECMOD_OpenUserDB");

        assert_eq!(error.situation(), Situation::StoreUnreachable);
        assert_eq!(error.detail(), "SECMOD_OpenUserDB");
        assert!(error.to_string().contains("StoreUnreachable"));
        assert!(error.to_string().contains("SECMOD_OpenUserDB"));
    }
}
