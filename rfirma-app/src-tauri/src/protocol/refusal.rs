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

/// **Qué situación es este rechazo**, clasificada y no redactada (ADR-0009,
/// ID-29).
///
/// Es lo que la ventana de sede necesita para nombrar el rechazo en el idioma
/// de la persona: el código `SAF_NN` es del cable y su frase la escribe el
/// catálogo del original, así que ninguno de los dos sirve para lo que se pinta
/// aquí dentro. Se clasifica **donde se rechaza** y no por el código, porque un
/// mismo `SAF_03` cubre situaciones que la ventana cuenta distinto.
///
/// [`Self::Unknown`] es el valor por defecto y **no es un descuido**: la
/// mayoría de los rechazos no tienen una frase propia que decir, y el detalle
/// crudo —lo único accionable de esa pantalla— viaja aparte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefusalSituation {
    /// `signaturePages=append`: la sede pide añadir una página en blanco, que
    /// es modificar el documento antes de firmarlo (ID-284).
    AppendedSignaturePage,
    /// Un criterio de filtro que no está en la lista blanca (ID-256).
    UnsupportedFilter,
    /// La sede declara una versión del protocolo que aquí no se habla (ID-251).
    UnsupportedProtocolVersion,
    /// La petición de firma no trae `format`.
    MissingFormat,
    /// Ya hay un trámite de sede vivo: no se atienden dos a la vez (ID-280).
    ErrandInFlight,
    /// Cualquier otro: la ventana lo cuenta en general y enseña el detalle.
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
            situation: RefusalSituation::Unknown,
            detail: detail.into(),
        }
    }

    /// **El mismo rechazo, dicho por su situación** (ID-341): lo que sale al
    /// cable no cambia ni un carácter, y lo que cambia es cómo lo nombra la
    /// ventana.
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

    /// **Clasificar no toca el cable** (ID-341): la situación es de la ventana,
    /// y el código y el parámetro salen igual que sin ella.
    #[test]
    fn the_situation_of_a_refusal_changes_nothing_that_goes_out() {
        let plain = Refusal::about(Parameter::Properties, "'signaturePages=append'");
        let classified = plain
            .clone()
            .because(RefusalSituation::AppendedSignaturePage);

        assert_eq!(plain.situation(), RefusalSituation::Unknown);
        assert_eq!(
            classified.situation(),
            RefusalSituation::AppendedSignaturePage
        );
        assert_eq!(classified.answer(), plain.answer());
        assert_eq!(classified.detail(), plain.detail());
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
