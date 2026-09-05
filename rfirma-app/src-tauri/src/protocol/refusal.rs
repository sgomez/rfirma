//! Por qué se rechaza lo que pide la sede, en el único idioma que el cliente
//! publicado entiende: un código `SAF_NN`.
//!
//! No hay forma de señalar un fallo que no sea con el prefijo `SAF_`
//! (`docs/research/contrato-protocolo-afirma.md`, §5): lo que no empiece por
//! ahí, no sea `CANCEL` y no sea una respuesta válida, el `autoscript.js` lo
//! toma por una firma. Por eso el rechazo **nace ya con su código**, y no se
//! traduce después.
//!
//! Aquí sólo están los códigos que rfirma sabe producir hoy: los tres del
//! parseo de la invocación y los cuatro que nacen dentro del canal ya abierto.
//! El catálogo
//! entero —los cincuenta y tres— y la frontera que los escribe en el cable son
//! del #349; cuando llegue, este enumerado se subsume en él y esta caja
//! desaparece. Mientras tanto, el detalle viaja aparte y **no** se envía: el
//! original tampoco lo manda (la sede nunca sabe *qué* parámetro estaba mal),
//! pero sirve para el registro y para que una prueba diga qué falló.

use std::fmt;

/// Los códigos de error del protocolo que rfirma sabe producir hoy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafCode {
    /// `SAF_03`, `ERROR_PARAMS`: error en los parámetros de entrada. Es el
    /// cajón de sastre del original, y aquí cubre desde un `ports` no numérico
    /// hasta un `mcv` que no parsea.
    Params,
    /// `SAF_21`, `ERROR_UNSUPPORTED_PROCEDURE`: la versión de protocolo que
    /// declara la sede no es la que se habla.
    UnsupportedProcedure,
    /// `SAF_41`, `ERROR_MINIMUM_VERSION_NON_SATISTIED`: la sede exige una
    /// versión de cliente mayor que la que se implementa.
    MinimumVersionNonSatisfied,
    /// `SAF_04`, `ERROR_UNSUPPORTED_OPERATION`: la operación que pide la URL no
    /// es ninguna de las que se atienden.
    UnsupportedOperation,
    /// `SAF_46`, `ERROR_INVALID_SESSION_ID`: el mensaje no repite la credencial
    /// del canal, o repite otra (`AfirmaWebSocketServerV4.java:72`-`78`).
    InvalidSessionId,
    /// `SAF_47`, `ERROR_EXTERNAL_REQUEST_TO_SOCKET`: la petición no viene del
    /// *loopback* (`AfirmaWebSocketServerV4.java:61`-`68`).
    ExternalRequestToSocket,
}

impl SafCode {
    /// El código tal y como viaja por el cable.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Params => "SAF_03",
            Self::UnsupportedProcedure => "SAF_21",
            Self::MinimumVersionNonSatisfied => "SAF_41",
            Self::UnsupportedOperation => "SAF_04",
            Self::InvalidSessionId => "SAF_46",
            Self::ExternalRequestToSocket => "SAF_47",
        }
    }

    /// La descripción con la que el código viaja por el canal.
    ///
    /// El formato es `SAF_NN: <descripción>`
    /// (`ProtocolInvocationLauncherErrorManager.getErrorMessage`, `:183`-`185`),
    /// y la sede enseña **la respuesta entera**, así que el texto es el del
    /// catálogo del original y no una redacción propia. Sólo están los códigos
    /// que rfirma produce hoy; el catálogo de los cincuenta y tres es del #349.
    pub fn description(self) -> &'static str {
        match self {
            Self::Params => "Error en los parametros de entrada",
            Self::UnsupportedProcedure => {
                "La version de Autofirma instalada no es compatible con este tramite"
            }
            Self::MinimumVersionNonSatisfied => {
                "El tramite requiere una version mas reciente de Autofirma"
            }
            Self::UnsupportedOperation => "Operacion no soportada",
            Self::InvalidSessionId => "Id de sesion invalido",
            Self::ExternalRequestToSocket => {
                "Peticion al socket desde IP externa o sin identificar"
            }
        }
    }

    /// La línea entera tal y como sale al cable: `SAF_NN: <descripción>`.
    pub fn on_the_wire(self) -> String {
        format!("{}: {}", self.as_str(), self.description())
    }
}

impl fmt::Display for SafCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Un rechazo: el código que va a la sede y el detalle que se queda aquí.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    code: SafCode,
    detail: String,
}

impl Refusal {
    /// Un rechazo con su detalle técnico, sin traducir y sin salir al cable.
    pub fn new(code: SafCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// `SAF_03`: cualquier parámetro que no se pueda interpretar.
    pub fn params(detail: impl Into<String>) -> Self {
        Self::new(SafCode::Params, detail)
    }

    /// El código que la sede recibe.
    pub fn code(&self) -> SafCode {
        self.code
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
    fn every_code_is_the_literal_the_published_client_recognises() {
        assert_eq!(SafCode::Params.as_str(), "SAF_03");
        assert_eq!(SafCode::UnsupportedProcedure.as_str(), "SAF_21");
        assert_eq!(SafCode::MinimumVersionNonSatisfied.as_str(), "SAF_41");
        assert_eq!(SafCode::UnsupportedOperation.as_str(), "SAF_04");
        assert_eq!(SafCode::InvalidSessionId.as_str(), "SAF_46");
        assert_eq!(SafCode::ExternalRequestToSocket.as_str(), "SAF_47");
    }

    /// Lo que la sede recibe es la línea entera, y lo único que el cliente
    /// publicado mira son los cuatro primeros caracteres (§5 del informe).
    #[test]
    fn a_code_travels_as_the_whole_line_the_published_client_recognises() {
        let line = SafCode::InvalidSessionId.on_the_wire();

        assert_eq!(line, "SAF_46: Id de sesion invalido");
        assert!(line.starts_with("SAF_"));
        assert!(
            line.len() > 4,
            "una linea de cuatro caracteres no es un error para el cliente publicado"
        );
    }

    #[test]
    fn a_refusal_keeps_its_untranslated_detail_next_to_the_code() {
        let refusal = Refusal::params("el parametro 'ports' no es numerico");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.detail(), "el parametro 'ports' no es numerico");
        assert!(refusal.to_string().starts_with("SAF_03: "));
    }
}
