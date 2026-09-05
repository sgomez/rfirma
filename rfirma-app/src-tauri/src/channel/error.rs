//! Los fallos del canal **se clasifican, no se traducen** (ADR-0009), igual
//! que los del material que lo cifra en [`crate::tls::error`].
//!
//! Son tres situaciones y no más, porque desde fuera sólo hay tres remedios
//! distintos: ningún puerto de los que sorteó la sede se ha podido atar, el
//! material del saludo TLS no sirve, y el escuchador no ha llegado a escuchar.
//! Ninguna de las tres es un `SAF_`: un canal que no se abre no tiene por
//! dónde contestar, así que lo que sale de aquí va a la ventana.

use std::fmt;

/// Situación del canal que la persona puede entender, y que el catálogo
/// traduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// Ninguno de los puertos que la sede sorteó estaba libre. En el original
    /// es `SAF_45` en un diálogo y **matar la aplicación entera**
    /// (`ProtocolInvocationLauncher.java:248`-`250`); aquí es un aviso en la
    /// ventana y la aplicación sigue viva.
    NoDrawnPortIsFree,
    /// El certificado del servidor local no se puede usar para el saludo TLS.
    /// Es material recién fabricado, así que esto es la pila de TLS diciendo
    /// que no.
    MaterialNotUsable,
    /// El escuchador estaba atado pero no ha llegado a escuchar: el sistema
    /// operativo diciendo que no en medio.
    NotListening,
}

/// Un fallo del canal: la situación traducible y el detalle crudo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelError {
    situation: Situation,
    detail: String,
}

impl ChannelError {
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

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for ChannelError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = ChannelError::new(Situation::NoDrawnPortIsFree, "Address already in use");

        assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
        assert_eq!(error.detail(), "Address already in use");
        assert!(error.to_string().contains("NoDrawnPortIsFree"));
        assert!(error.to_string().contains("Address already in use"));
    }
}
