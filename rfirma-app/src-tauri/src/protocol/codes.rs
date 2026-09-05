//! **El catálogo publicado**: los cincuenta y tres códigos `SAF_00`…`SAF_52`,
//! y lo único que además de ellos entiende el cliente de la sede (ID-289).
//!
//! No hay forma de señalar un fallo que no sea con el prefijo `SAF_`
//! (`docs/research/contrato-protocolo-afirma.md`, §5): lo que no empiece por
//! ahí, no sea `CANCEL`, `MEMORY_ERROR` o `NULL`, el `autoscript.js` lo entrega
//! a la sede **como si fuera una firma**. Por eso el catálogo es un `enum`
//! cerrado y no una cadena: aquí no se acuña ningún código, y no existen los
//! `AF…` ni los `AS…`, que son de `master` (ID-289, ID-294).
//!
//! Detrás del código va **frase nuestra, en castellano fijo** (ID-290), no el
//! texto de `protocolmessages.properties`: la sede enseña la respuesta entera,
//! así que lo que se lee ahí es responsabilidad de rFirma. Donde el original
//! colapsa cualquier fallo de parámetros en un `SAF_03` mudo, la frase **nombra
//! el parámetro** que vino mal ([`Parameter`]).
//!
//! Las frases van **sin acentos**, igual que el catálogo del original tal y
//! como llega al cable: la respuesta viaja por un canal que ninguna sede
//! declara codificado, y el ASCII es lo único que se lee igual en todas.
//!
//! Y el detalle crudo de nuestras situaciones (ID-29) **no está aquí ni puede
//! llegar**: lo que sale al cable se compone de un código de este `enum` y de
//! una frase constante, y nada más (ID-291).

use std::fmt;

/// La cancelación de la persona, que **no es un código**
/// (`ProtocolInvocationLauncherSign.java:104`).
pub const CANCELLED: &str = "CANCEL";

/// La respuesta que el cliente lee como falta de memoria
/// (`autoscript.js:2312`-`2315`).
pub const OUT_OF_MEMORY: &str = "MEMORY_ERROR";

/// La respuesta que el cliente lee como «error desconocido»
/// (`autoscript.js:2324`-`2327`).
pub const NOTHING: &str = "NULL";

/// El parámetro de la llamada que la sede mandó mal.
///
/// Es una lista cerrada de **nombres del protocolo**, no texto libre: lo que
/// sale al cable no puede depender de lo que traía la URL (ID-291).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    /// `ports`: los puertos que la sede sorteó.
    Ports,
    /// `v`: la versión de protocolo que declara la sede.
    ProtocolVersion,
    /// `idsession`: la credencial del canal.
    IdSession,
    /// `mcv`: la versión mínima de cliente que la sede exige.
    MinimumClientVersion,
    /// `dat`: los datos a firmar.
    Data,
    /// `properties`: los `extraParams` de la operacion, filtros incluidos.
    Properties,
}

impl Parameter {
    /// Todos, para las pruebas de totalidad.
    pub const ALL: [Self; 6] = [
        Self::Ports,
        Self::ProtocolVersion,
        Self::IdSession,
        Self::MinimumClientVersion,
        Self::Data,
        Self::Properties,
    ];

    /// El nombre con el que viaja en la URL del protocolo.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ports => "ports",
            Self::ProtocolVersion => "v",
            Self::IdSession => "idsession",
            Self::MinimumClientVersion => "mcv",
            Self::Data => "dat",
            Self::Properties => "properties",
        }
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Los cincuenta y tres códigos del catálogo publicado, y ni uno más.
///
/// **El orden es el de los números**: el discriminante de cada variante es su
/// `NN`, y es lo que la empareja con su fila de [`CATALOGUE`]. Una variante
/// nueva va en su sitio, no al final, y lo comprueba una prueba.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SafCode {
    /// `SAF_00`, `ERROR_CANNOT_READ_DATA`.
    CannotReadData,
    /// `SAF_01`, `ERROR_NULL_URI`.
    NullUri,
    /// `SAF_02`, `ERROR_UNSUPPORTED_PROTOCOL`.
    UnsupportedProtocol,
    /// `SAF_03`, `ERROR_PARAMS`. El cajón de sastre del original, y el único
    /// que nombra el parámetro que vino mal (ID-290).
    Params,
    /// `SAF_04`, `ERROR_UNSUPPORTED_OPERATION`.
    UnsupportedOperation,
    /// `SAF_05`, `ERROR_CANNOT_SAVE_DATA`.
    CannotSaveData,
    /// `SAF_06`, `ERROR_UNSUPPORTED_FORMAT`.
    UnsupportedFormat,
    /// `SAF_07`, `ERROR_CANNOT_FIND_KEYSTORE`.
    CannotFindKeystore,
    /// `SAF_08`, `ERROR_CANNOT_ACCESS_KEYSTORE`.
    CannotAccessKeystore,
    /// `SAF_09`, `ERROR_SIGNATURE_FAILED`.
    SignatureFailed,
    /// `SAF_10`, `ERROR_NO_CERTIFICATES_SYSTEM`.
    NoCertificatesInSystem,
    /// `SAF_11`, `ERROR_SENDING_RESULT`.
    SendingResult,
    /// `SAF_12`, `ERROR_ENCRIPTING_DATA`.
    EncryptingData,
    /// `SAF_13`, `ERROR_LOCAL_ACCESS_BLOCKED`.
    LocalAccessBlocked,
    /// `SAF_14`, `ERROR_OBSOLETE_APP`.
    ObsoleteApp,
    /// `SAF_15`, `ERROR_DECRYPTING_DATA`.
    DecryptingData,
    /// `SAF_16`, `ERROR_RECOVERING_DATA`.
    RecoveringData,
    /// `SAF_17`, `ERROR_UNKNOWN_SIGNER`.
    UnknownSigner,
    /// `SAF_18`, `ERROR_DECODING_CERTIFICATE`.
    DecodingCertificate,
    /// `SAF_19`, `ERROR_NO_CERTIFICATES_KEYSTORE`.
    NoCertificatesInKeystore,
    /// `SAF_20`, `ERROR_LOCAL_BATCH_SIGN`.
    LocalBatchSign,
    /// `SAF_21`, `ERROR_UNSUPPORTED_PROCEDURE`.
    UnsupportedProcedure,
    /// `SAF_22`, `ERROR_UNSOPPORTED_WEB_PROCEDURE`.
    UnsupportedWebProcedure,
    /// `SAF_23`, `ERROR_INVALID_POLICY`.
    InvalidPolicy,
    /// `SAF_24`, `ERROR_RECOVERING_LOG`.
    RecoveringLog,
    /// `SAF_25`, `ERROR_CANNOT_LOAD_DATA`.
    CannotLoadData,
    /// `SAF_26`, `ERROR_CONTACT_BATCH_SERVICE`.
    ContactBatchService,
    /// `SAF_27`, `ERROR_BATCH_SIGNATURE`.
    BatchSignature,
    /// `SAF_28`, `ERROR_INVALID_PDF`.
    InvalidPdf,
    /// `SAF_29`, `ERROR_INVALID_XML`.
    InvalidXml,
    /// `SAF_30`, `ERROR_INVALID_DATA`.
    InvalidData,
    /// `SAF_31`, `ERROR_NO_SIGN_DATA`.
    NoSignData,
    /// `SAF_32`, `ERROR_FACE_ALREADY_SIGNED`.
    FacturaeAlreadySigned,
    /// `SAF_33`, `ERROR_PDF_WRONG_PASSWORD`.
    PdfWrongPassword,
    /// `SAF_34`, `ERROR_PDF_UNREG_SIGN`.
    PdfUnregisteredSignatures,
    /// `SAF_35`, `ERROR_PDF_CERTIFIED`.
    PdfCertified,
    /// `SAF_36`, `ERROR_CANNOT_FIND_SSL_KEYSTORE`.
    CannotFindSslKeystore,
    /// `SAF_37`, `ERROR_CANNOT_ACCESS_SSL_KEYSTORE`.
    CannotAccessSslKeystore,
    /// `SAF_38`, `ERROR_INVALID_FACTURAE`.
    InvalidFacturae,
    /// `SAF_39`, `ERROR_INVALID_SIGNATURE`.
    InvalidSignature,
    /// `SAF_40`, `ERROR_RECOVER_SERVER_DOCUMENT`.
    RecoverServerDocument,
    /// `SAF_41`, `ERROR_MINIMUM_VERSION_NON_SATISTIED`.
    MinimumVersionNonSatisfied,
    /// `SAF_42`, `ERROR_POSTPROCESSING_DATA`.
    PostprocessingData,
    /// `SAF_43`, `ERROR_VISIBLE_SIGNATURE`.
    VisibleSignature,
    /// `SAF_44`, `ERROR_SIGN_WITHOUT_DATA`.
    SignWithoutData,
    /// `SAF_45`, `ERROR_CANNOT_OPEN_SOCKET`.
    CannotOpenSocket,
    /// `SAF_46`, `ERROR_INVALID_SESSION_ID`.
    InvalidSessionId,
    /// `SAF_47`, `ERROR_EXTERNAL_REQUEST_TO_SOCKET`.
    ExternalRequestToSocket,
    /// `SAF_48`, `ERROR_PDF_SHADOW_ATTACK`. **Está en el catálogo y rFirma no
    /// lo emite nunca** (ID-295): `PdfShadowAttackException` es de `master`, y
    /// la 1.9.2 contra la que se firma no la lanza. Lo vigila una guarda.
    PdfShadowAttack,
    /// `SAF_49`, `ERROR_SIGNING_LTS_SIGNATURE`.
    SigningLtsSignature,
    /// `SAF_50`, `ERROR_CONFIRMATION_NEEDED`.
    ConfirmationNeeded,
    /// `SAF_51`, `ERROR_INCOMPATIBLE_KEY_TYPE`.
    IncompatibleKeyType,
    /// `SAF_52`, `ERROR_LOCKED_KEYSTORE`.
    LockedKeystore,
}

/// **El catálogo, fila a fila**: el código tal y como viaja por el cable y la
/// frase nuestra que lo acompaña (ID-289, ID-290).
///
/// La fila `n` es la del código `SAF_nn`, y la variante de [`SafCode`] que la
/// nombra es la `n`-ésima: la tabla y el `enum` se emparejan por posición, y
/// una prueba lo comprueba fila a fila.
const CATALOGUE: [(&str, &str); 53] = [
    ("SAF_00", "No se han podido leer los datos a firmar"),
    ("SAF_01", "La llamada no trae ninguna direccion"),
    ("SAF_02", "La llamada no es del protocolo afirma"),
    ("SAF_03", "Error en los parametros de entrada"),
    ("SAF_04", "Operacion no soportada"),
    ("SAF_05", "No se ha podido guardar el documento firmado"),
    ("SAF_06", "Formato de firma no soportado"),
    ("SAF_07", "No se encuentra ningun almacen de claves"),
    ("SAF_08", "No se ha podido acceder al almacen de claves"),
    ("SAF_09", "No se ha podido completar la firma electronica"),
    (
        "SAF_10",
        "No hay certificados de firma instalados en el sistema",
    ),
    ("SAF_11", "No se ha podido enviar el resultado"),
    ("SAF_12", "No se han podido cifrar los datos a enviar"),
    (
        "SAF_13",
        "Se ha pedido leer un fichero local y se ha bloqueado",
    ),
    (
        "SAF_14",
        "La version instalada de la aplicacion es obsoleta",
    ),
    ("SAF_15", "No se han podido descifrar los datos"),
    (
        "SAF_16",
        "No se han podido recuperar los datos del servidor intermedio",
    ),
    (
        "SAF_17",
        "Los datos no son una firma electronica reconocida",
    ),
    ("SAF_18", "No se ha podido leer el certificado de firma"),
    (
        "SAF_19",
        "No hay ningun certificado utilizable en el almacen",
    ),
    ("SAF_20", "No se ha podido procesar el lote de firma"),
    (
        "SAF_21",
        "Este tramite no es compatible con la version instalada",
    ),
    (
        "SAF_22",
        "El tramite web no es compatible con la version instalada",
    ),
    ("SAF_23", "Politica de firma no valida o incompatible"),
    ("SAF_24", "No se ha podido obtener el registro"),
    ("SAF_25", "No se han podido cargar los datos"),
    (
        "SAF_26",
        "No se ha podido contactar con el servicio de firma de lotes",
    ),
    (
        "SAF_27",
        "El servicio informo de un error en la firma del lote",
    ),
    ("SAF_28", "El fichero no es un PDF o es un PDF no soportado"),
    (
        "SAF_29",
        "Las firmas XAdES Enveloped solo se hacen sobre XML",
    ),
    (
        "SAF_30",
        "El formato de los datos no sirve para el tipo de firma pedido",
    ),
    (
        "SAF_31",
        "Los datos no se corresponden con un objeto de firma",
    ),
    ("SAF_32", "La factura ya tiene firma y no admite mas firmas"),
    ("SAF_33", "La contrasena del PDF no es valida o falta"),
    ("SAF_34", "El PDF contiene firmas no registradas"),
    ("SAF_35", "El PDF esta certificado y no admite mas firmas"),
    ("SAF_36", "No se encuentra el almacen de claves SSL"),
    ("SAF_37", "No se ha podido acceder al almacen de claves SSL"),
    (
        "SAF_38",
        "El fichero no es una factura electronica reconocida",
    ),
    ("SAF_39", "La firma de entrada no es valida"),
    ("SAF_40", "No se ha podido recuperar el documento"),
    (
        "SAF_41",
        "El tramite exige una version mas reciente de la aplicacion",
    ),
    ("SAF_42", "No se ha podido postprocesar la firma"),
    ("SAF_43", "No se ha podido estampar la firma visible"),
    (
        "SAF_44",
        "La firma no contiene los datos y la configuracion no lo admite",
    ),
    ("SAF_45", "No se ha podido abrir el canal de comunicacion"),
    ("SAF_46", "Id de sesion invalido"),
    (
        "SAF_47",
        "Peticion al canal desde una direccion externa o sin identificar",
    ),
    ("SAF_48", "Posible PDF Shadow Attack"),
    ("SAF_49", "No se puede multifirmar una firma de archivo"),
    (
        "SAF_50",
        "La operacion puede generar firmas no validas y necesita confirmacion",
    ),
    (
        "SAF_51",
        "El tipo de clave del certificado no esta soportado",
    ),
    ("SAF_52", "El almacen de claves esta bloqueado"),
];

impl SafCode {
    /// El catálogo entero, en el orden de sus números.
    pub const ALL: [Self; 53] = [
        Self::CannotReadData,
        Self::NullUri,
        Self::UnsupportedProtocol,
        Self::Params,
        Self::UnsupportedOperation,
        Self::CannotSaveData,
        Self::UnsupportedFormat,
        Self::CannotFindKeystore,
        Self::CannotAccessKeystore,
        Self::SignatureFailed,
        Self::NoCertificatesInSystem,
        Self::SendingResult,
        Self::EncryptingData,
        Self::LocalAccessBlocked,
        Self::ObsoleteApp,
        Self::DecryptingData,
        Self::RecoveringData,
        Self::UnknownSigner,
        Self::DecodingCertificate,
        Self::NoCertificatesInKeystore,
        Self::LocalBatchSign,
        Self::UnsupportedProcedure,
        Self::UnsupportedWebProcedure,
        Self::InvalidPolicy,
        Self::RecoveringLog,
        Self::CannotLoadData,
        Self::ContactBatchService,
        Self::BatchSignature,
        Self::InvalidPdf,
        Self::InvalidXml,
        Self::InvalidData,
        Self::NoSignData,
        Self::FacturaeAlreadySigned,
        Self::PdfWrongPassword,
        Self::PdfUnregisteredSignatures,
        Self::PdfCertified,
        Self::CannotFindSslKeystore,
        Self::CannotAccessSslKeystore,
        Self::InvalidFacturae,
        Self::InvalidSignature,
        Self::RecoverServerDocument,
        Self::MinimumVersionNonSatisfied,
        Self::PostprocessingData,
        Self::VisibleSignature,
        Self::SignWithoutData,
        Self::CannotOpenSocket,
        Self::InvalidSessionId,
        Self::ExternalRequestToSocket,
        Self::PdfShadowAttack,
        Self::SigningLtsSignature,
        Self::ConfirmationNeeded,
        Self::IncompatibleKeyType,
        Self::LockedKeystore,
    ];

    /// El código y su frase, en un solo sitio: dos tablas separadas se
    /// desalinean, y aquí la pareja es el contrato. La fila se busca por el
    /// **número del código**, que es el discriminante de la variante.
    const fn entry(self) -> (&'static str, &'static str) {
        CATALOGUE[self as usize]
    }

    /// El código tal y como viaja por el cable.
    pub fn as_str(self) -> &'static str {
        self.entry().0
    }

    /// La frase nuestra que acompaña al código (ID-290).
    pub fn phrase(self) -> &'static str {
        self.entry().1
    }
}

impl fmt::Display for SafCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// **Todo lo que rFirma puede contestarle a la sede cuando no sale una firma**,
/// y nada más (ID-289).
///
/// Es el tipo por el que pasa la frontera: quien escribe en el socket escribe
/// [`WireAnswer::on_the_wire`] y no una cadena suya.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireAnswer {
    /// Un código del catálogo, con el parámetro que lo provocó cuando lo hay.
    Refused {
        /// El código del catálogo.
        code: SafCode,
        /// El parámetro que la sede mandó mal, si el rechazo es de uno.
        blame: Option<Parameter>,
    },
    /// La persona ha cancelado. **Ningún rechazo nuestro es esto** (ID-293).
    Cancelled,
    /// No hay memoria para el resultado.
    OutOfMemory,
    /// La operación no ha dado resultado y no hay más que decir.
    Nothing,
}

impl WireAnswer {
    /// Un rechazo sin parámetro al que señalar.
    pub fn refused(code: SafCode) -> Self {
        Self::Refused { code, blame: None }
    }

    /// Un rechazo que sabe **qué parámetro** vino mal (ID-290).
    pub fn refused_because_of(code: SafCode, blame: Parameter) -> Self {
        Self::Refused {
            code,
            blame: Some(blame),
        }
    }

    /// La línea entera tal y como sale al cable.
    ///
    /// `SAF_NN: <frase>`, y con el parámetro nombrado detrás cuando el rechazo
    /// es de uno. Nada de aquí sale de una entrada: ni el detalle crudo, ni la
    /// ruta, ni el nombre del documento, ni el certificado (ID-291).
    pub fn on_the_wire(self) -> String {
        match self {
            Self::Refused { code, blame: None } => format!("{}: {}", code.as_str(), code.phrase()),
            Self::Refused {
                code,
                blame: Some(parameter),
            } => format!(
                "{}: {}; el parametro que falla es '{}'",
                code.as_str(),
                code.phrase(),
                parameter.name()
            ),
            Self::Cancelled => CANCELLED.to_owned(),
            Self::OutOfMemory => OUT_OF_MEMORY.to_owned(),
            Self::Nothing => NOTHING.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// **Exactitud**: el catálogo es el publicado, `SAF_00`…`SAF_52`, y no hay
    /// ni un código acuñado (ID-289, TD-57).
    #[test]
    fn the_catalogue_is_the_fifty_three_published_codes_and_nothing_else() {
        let literals: BTreeSet<&str> = SafCode::ALL.iter().map(|code| code.as_str()).collect();

        assert_eq!(literals.len(), 53, "hay codigos repetidos en el catalogo");
        let expected: BTreeSet<String> = (0..53).map(|number| format!("SAF_{number:02}")).collect();
        let expected: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
        assert_eq!(literals, expected);
    }

    /// Y el emparejamiento entre el `enum` y la tabla es por posición, así que
    /// se comprueba fila a fila: una variante metida en el sitio equivocado
    /// desplazaría todas las frases sin que nada más se pusiera rojo.
    #[test]
    fn each_variant_sits_on_the_row_of_its_own_number() {
        for (number, code) in SafCode::ALL.iter().enumerate() {
            assert_eq!(*code as usize, number, "{code:?} no esta en su sitio");
            assert_eq!(code.as_str(), format!("SAF_{number:02}"));
        }
    }

    /// **Totalidad**: cada código tiene frase nuestra, y ninguna es la cadena
    /// vacía ni lleva acentos que el cable no garantiza (ID-290).
    #[test]
    fn every_code_carries_a_phrase_of_ours_in_plain_ascii() {
        for code in SafCode::ALL {
            let phrase = code.phrase();
            assert!(!phrase.is_empty(), "{code:?} no tiene frase");
            assert!(phrase.is_ascii(), "«{phrase}» no es ASCII");
            assert!(
                !phrase.ends_with('.'),
                "«{phrase}» acaba en punto y el parametro se nombra detras"
            );
        }
    }

    /// Lo que la sede recibe es la línea entera, y el cliente publicado sólo
    /// mira los cuatro primeros caracteres (§5 del informe).
    #[test]
    fn every_refusal_travels_as_a_line_the_published_client_recognises() {
        for code in SafCode::ALL {
            let line = WireAnswer::refused(code).on_the_wire();

            assert!(line.starts_with("SAF_"), "«{line}» no la lee como error");
            assert!(
                line.len() > 4,
                "«{line}» tiene cuatro caracteres y no es un error para el cliente"
            );
        }
    }

    /// Y donde el original deja un `SAF_03` mudo, la respuesta **nombra el
    /// parámetro** (ID-290).
    #[test]
    fn a_bad_parameter_is_named_behind_the_code() {
        let line = WireAnswer::refused_because_of(SafCode::Params, Parameter::Ports).on_the_wire();

        assert_eq!(
            line,
            "SAF_03: Error en los parametros de entrada; el parametro que falla es 'ports'"
        );
    }

    /// Las tres respuestas que no son códigos van **desnudas**: cualquier
    /// adorno las convertiría en una firma a ojos del cliente (§5).
    #[test]
    fn the_three_answers_that_are_not_codes_travel_bare() {
        assert_eq!(WireAnswer::Cancelled.on_the_wire(), "CANCEL");
        assert_eq!(WireAnswer::OutOfMemory.on_the_wire(), "MEMORY_ERROR");
        assert_eq!(WireAnswer::Nothing.on_the_wire(), "NULL");
    }

    #[test]
    fn every_parameter_is_named_as_the_protocol_names_it() {
        for parameter in Parameter::ALL {
            let name = parameter.name();
            assert!(!name.is_empty());
            assert!(
                name.chars().all(|letter| letter.is_ascii_lowercase()),
                "«{name}» no es un nombre de parametro del protocolo"
            );
        }
    }
}
