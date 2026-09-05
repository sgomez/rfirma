//! Por qué se rechaza lo que pide la sede: el código del catálogo que sale al
//! cable, y el detalle crudo que **se queda aquí**.
//!
//! El código y su frase son de [`super::codes`], que es el catálogo cerrado
//! (ID-289). Lo que este tipo añade es lo de dentro: el detalle técnico sin
//! traducir, que sirve para el registro y para que una prueba diga qué falló, y
//! que **no cruza el socket jamás** (ID-291). El original tampoco lo manda: la
//! sede nunca sabe *qué* parámetro estaba mal. rFirma sí lo dice, pero por el
//! nombre del parámetro ([`Parameter`]) y nunca por el detalle.

use std::fmt;

use super::codes::{Parameter, SafCode, WireAnswer};

/// Un rechazo: lo que va a la sede y el detalle que se queda aquí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    code: SafCode,
    blame: Option<Parameter>,
    detail: String,
}

impl Refusal {
    /// Un rechazo con su detalle técnico, sin traducir y sin salir al cable.
    pub fn new(code: SafCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            blame: None,
            detail: detail.into(),
        }
    }

    /// `SAF_03` sin un parámetro concreto al que señalar: la llamada entera no
    /// se puede interpretar.
    pub fn params(detail: impl Into<String>) -> Self {
        Self::new(SafCode::Params, detail)
    }

    /// `SAF_03` **nombrando el parámetro** que la sede mandó mal (ID-290).
    pub fn about(blame: Parameter, detail: impl Into<String>) -> Self {
        Self {
            code: SafCode::Params,
            blame: Some(blame),
            detail: detail.into(),
        }
    }

    /// El código que la sede recibe.
    pub fn code(&self) -> SafCode {
        self.code
    }

    /// El parámetro que lo provocó, cuando el rechazo es de uno.
    pub fn blame(&self) -> Option<Parameter> {
        self.blame
    }

    /// **Lo único de este rechazo que sale al cable** (ID-288): el código y su
    /// frase, con el parámetro nombrado si lo hay.
    pub fn answer(&self) -> WireAnswer {
        WireAnswer::Refused {
            code: self.code,
            blame: self.blame,
        }
    }

    /// El detalle técnico crudo. Nunca vacío, nunca traducido, nunca enviado.
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
mod tests {
    use super::*;

    #[test]
    fn a_refusal_keeps_its_untranslated_detail_next_to_the_code() {
        let refusal = Refusal::about(Parameter::Ports, "el parametro 'ports' no es numerico");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.detail(), "el parametro 'ports' no es numerico");
        assert!(refusal.to_string().starts_with("SAF_03: "));
    }

    /// Y lo que sale al cable **no lleva el detalle**, sólo el código, su frase
    /// y el nombre del parámetro (ID-291).
    #[test]
    fn the_untranslated_detail_is_not_part_of_what_goes_out() {
        let refusal = Refusal::about(Parameter::IdSession, "idsession='../../etc/passwd'");

        let line = refusal.answer().on_the_wire();

        assert!(!line.contains("passwd"), "«{line}» lleva el detalle crudo");
        assert!(line.ends_with("el parametro que falla es 'idsession'"));
    }
}
