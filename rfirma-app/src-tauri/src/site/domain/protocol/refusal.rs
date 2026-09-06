//! Rechazos a la sede: código del catálogo para el cable y detalle local (ADR-0009).

use std::fmt;

use super::codes::{Parameter, SafCode, WireAnswer};

/// Situación del rechazo para la presentación en interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefusalSituation {
    /// La sede pide añadir una página en blanco al documento.
    AppendedSignaturePage,
    /// Un criterio de filtro que no está en la lista blanca.
    UnsupportedFilter,
    /// La sede declara una versión del protocolo no soportada.
    UnsupportedProtocolVersion,
    /// La petición de firma no trae formato.
    MissingFormat,
    /// Ya hay un trámite de sede en curso.
    ErrandInFlight,
    /// Cualquier otra situación no clasificada individualmente.
    #[default]
    Unknown,
}

/// Un rechazo: lo que va a la sede y el detalle que se queda aquí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    code: SafCode,
    blame: Option<Parameter>,
    situation: RefusalSituation,
    detail: String,
}

impl Refusal {
    /// Un rechazo con su detalle técnico, sin traducir y sin salir al cable.
    pub fn new(code: SafCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            blame: None,
            situation: RefusalSituation::Unknown,
            detail: detail.into(),
        }
    }

    /// Rechazo SAF_03 sin un parámetro concreto al que señalar.
    pub fn params(detail: impl Into<String>) -> Self {
        Self::new(SafCode::Params, detail)
    }

    /// Rechazo SAF_03 indicando el parámetro que la sede mandó mal.
    pub fn about(blame: Parameter, detail: impl Into<String>) -> Self {
        Self {
            code: SafCode::Params,
            blame: Some(blame),
            situation: RefusalSituation::Unknown,
            detail: detail.into(),
        }
    }

    /// Clasifica el rechazo con su situación para la interfaz.
    #[must_use = "devuelve el rechazo clasificado, no lo modifica en su sitio"]
    pub fn because(mut self, situation: RefusalSituation) -> Self {
        self.situation = situation;
        self
    }

    /// Qué situación es, para la ventana y nunca para el cable.
    pub fn situation(&self) -> RefusalSituation {
        self.situation
    }

    /// El código que la sede recibe.
    pub fn code(&self) -> SafCode {
        self.code
    }

    /// El parámetro que lo provocó, cuando el rechazo es de uno.
    pub fn blame(&self) -> Option<Parameter> {
        self.blame
    }

    /// Respuesta que sale al cable: código y frase con el parámetro si lo hay.
    pub fn answer(&self) -> WireAnswer {
        WireAnswer::Refused {
            code: self.code,
            blame: self.blame,
        }
    }

    /// El detalle técnico crudo.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for Refusal {}

#[cfg(test)]
mod tests;
