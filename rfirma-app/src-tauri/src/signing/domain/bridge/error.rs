//! Los errores de la frontera con el puente nativo, sin la frontera.

use std::fmt;
use std::os::raw::c_int;
use std::path::PathBuf;

use super::{Format, LibraryNotFound};

/// Errores posibles al cruzar la frontera FFI con el puente nativo.
#[derive(Debug)]
pub enum BridgeError {
    /// No se puede determinar la ruta del ejecutable.
    ExecutablePathUnknown(String),
    /// No hay librería que cargar.
    NotFound(LibraryNotFound),
    /// Error de carga dinámica de la librería.
    Load {
        /// Fichero que se intentó abrir.
        path: PathBuf,
        /// Detalle devuelto por el cargador dinámico.
        detail: String,
    },
    /// Falta un símbolo esperado en la librería.
    MissingSymbol {
        /// Símbolo ausente.
        symbol: String,
        /// Detalle devuelto por el cargador dinámico.
        detail: String,
    },
    /// Error al crear el isolate de GraalVM.
    IsolateFailed(c_int),
    /// Argumento con byte nulo no convertible a CString.
    InvalidArgument(&'static str),
    /// El puente ha devuelto un puntero nulo.
    NullResponse,
    /// Respuesta con formato no válido devuelta por el puente.
    MalformedResponse(String),
    /// Fallo devuelto por el puente nativo.
    Failed(String),
    /// La política de firma no se puede aplicar al formato solicitado.
    IncompatiblePolicy(String),
    /// El PDF contiene firmas no registradas en su diccionario.
    PdfHasUnregisteredSignatures(String),
    /// El puente no resuelve todavía ese formato de firma.
    FormatNotBridged(Format),
    /// El firmador del original rechaza los datos por no ser lo que el formato pide.
    DataRejected(DataRejection, String),
}

/// Por qué el firmador del original rechaza los datos que se le dan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRejection {
    /// Lo que se da a PAdES no es un PDF que el firmador pueda leer.
    InvalidPdf,
    /// Lo que se da a una firma XML no es XML.
    InvalidXml,
    /// Los datos no casan con el formato de firma pedido.
    InvalidData,
    /// Lo que se da a una multifirma no es una firma.
    NoSignData,
    /// La factura ya está firmada y no admite más firmas.
    FacturaeAlreadySigned,
    /// Lo que se da a FacturaE no es una factura.
    InvalidFacturae,
    /// La firma previa no trae los datos ni una huella del algoritmo pedido.
    SignWithoutData,
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutablePathUnknown(detail) => {
                write!(f, "no puedo saber dónde está el ejecutable: {detail}")
            }
            Self::NotFound(error) => write!(f, "{error}"),
            Self::Load { path, detail } => {
                write!(f, "no puedo cargar {}: {detail}", path.display())
            }
            Self::MissingSymbol { symbol, detail } => {
                write!(f, "la librería no exporta {symbol}: {detail}")
            }
            Self::IsolateFailed(code) => write!(f, "graal_create_isolate ha devuelto {code}"),
            Self::InvalidArgument(name) => write!(f, "{name} lleva un \\0 dentro"),
            Self::NullResponse => write!(f, "el puente ha devuelto NULL"),
            Self::MalformedResponse(detail) => write!(f, "respuesta ilegible del puente: {detail}"),
            Self::Failed(detail) => write!(f, "el puente ha fallado: {detail}"),
            Self::IncompatiblePolicy(detail) => {
                write!(f, "la politica de firma no se puede aplicar: {detail}")
            }
            Self::PdfHasUnregisteredSignatures(detail) => {
                write!(f, "el PDF trae firmas no registradas: {detail}")
            }
            Self::FormatNotBridged(format) => {
                write!(f, "el puente no atiende el formato {format}")
            }
            Self::DataRejected(_, detail) => {
                write!(f, "el firmador rechaza los datos: {detail}")
            }
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<LibraryNotFound> for BridgeError {
    fn from(error: LibraryNotFound) -> Self {
        Self::NotFound(error)
    }
}
