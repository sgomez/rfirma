//! El vocabulario con el que se habla al puente nativo, sin la carga de la biblioteca.

use std::fmt;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use base64::Engine;

use super::{SealMismatch, SessionSeal};

/// Nombre del fichero de la librería nativa compartida (ADR-0004, ADR-0012).
pub const LIBRARY_FILE: &str = "librfirma_crypto.so";

/// Variable de entorno que sobreescribe el directorio de la librería nativa.
pub const LIBRARY_DIRECTORY_VARIABLE: &str = "RFIRMA_LIB_DIR";

/// Procedencia de un directorio candidato para la librería nativa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Directorio indicado en [`LIBRARY_DIRECTORY_VARIABLE`].
    Override,
    /// Directorio relativo al ejecutable.
    RelativeToExecutable,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Override => write!(f, "{LIBRARY_DIRECTORY_VARIABLE}"),
            Self::RelativeToExecutable => write!(f, "relativa al ejecutable"),
        }
    }
}

/// Directorio candidato para la librería nativa y su procedencia.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub(crate) directory: PathBuf,
    pub(crate) origin: Origin,
}

impl Candidate {
    /// Directorio examinado.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Procedencia del candidato.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Ruta esperada del fichero de la librería en este directorio.
    pub fn library_path(&self) -> PathBuf {
        self.directory.join(LIBRARY_FILE)
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.library_path().display(), self.origin)
    }
}

/// Error cuando la librería nativa no se encuentra en ningún directorio candidato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryNotFound {
    pub(crate) looked_at: Vec<Candidate>,
}

impl LibraryNotFound {
    /// Candidatos examinados.
    pub fn looked_at(&self) -> &[Candidate] {
        &self.looked_at
    }
}

impl fmt::Display for LibraryNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no encuentro {LIBRARY_FILE}; he mirado en:")?;
        for candidate in &self.looked_at {
            write!(f, "\n  {candidate}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LibraryNotFound {}

/// Cómo se envuelve una firma XAdES.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XadesVariant {
    /// `XAdES Detached`.
    Detached,
    /// `XAdES Enveloping`.
    Enveloping,
    /// `XAdES Enveloped`.
    Enveloped,
    /// `XAdES-ASiC-S`.
    AsicS,
}

/// Qué se hace con lo que entra: firmar, cofirmar la firma que llega o contrafirmarla.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignatureOperation {
    /// Firma de lo que llega.
    #[default]
    Sign,
    /// Cofirma de la firma que llega.
    Cosign,
    /// Contrafirma de la firma que llega; el objetivo viaja en los `extraParams`.
    Countersign,
}

impl SignatureOperation {
    /// El nombre con el que el puente la espera.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sign => "sign",
            Self::Cosign => "cosign",
            Self::Countersign => "countersign",
        }
    }
}

/// El formato de una firma, con los nombres de `AOSignConstants` del original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Firma PAdES sobre un PDF.
    Pades,
    /// Firma CAdES.
    Cades,
    /// Firma CAdES en un contenedor ASiC-S.
    CadesAsicS,
    /// Firma CMS / PKCS#7.
    Cms,
    /// Firma XAdES en una de sus envolturas.
    Xades(XadesVariant),
    /// Firma de una factura electrónica.
    FacturaE,
}

impl Format {
    /// Todos los formatos del vocabulario, para recorrerlos.
    pub const ALL: [Self; 9] = [
        Self::Pades,
        Self::Cades,
        Self::CadesAsicS,
        Self::Cms,
        Self::Xades(XadesVariant::Detached),
        Self::Xades(XadesVariant::Enveloping),
        Self::Xades(XadesVariant::Enveloped),
        Self::Xades(XadesVariant::AsicS),
        Self::FacturaE,
    ];

    /// El nombre con el que el original lo espera (`AOSignConstants.SIGN_FORMAT_*`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Pades => "PAdES",
            Self::Cades => "CAdES",
            Self::CadesAsicS => "CAdES-ASiC-S",
            Self::Cms => "CMS/PKCS#7",
            Self::Xades(XadesVariant::Detached) => "XAdES Detached",
            Self::Xades(XadesVariant::Enveloping) => "XAdES Enveloping",
            Self::Xades(XadesVariant::Enveloped) => "XAdES Enveloped",
            Self::Xades(XadesVariant::AsicS) => "XAdES-ASiC-S",
            Self::FacturaE => "FacturaE",
        }
    }

    /// El formato si el puente lo resuelve, y si no la situación que lo niega.
    pub fn bridged(self) -> Result<Self, BridgeError> {
        match self {
            Self::Pades
            | Self::Cades
            | Self::CadesAsicS
            | Self::Cms
            | Self::Xades(_)
            | Self::FacturaE => Ok(self),
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Uno de los bloques que el token tiene que firmar, con el identificador que Java exige de vuelta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreSignBlock {
    pub(crate) id: String,
    pub(crate) pre: Vec<u8>,
}

impl PreSignBlock {
    /// Identificador del bloque dentro de la sesión trifásica.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Bytes DER de los atributos firmados.
    pub fn pre_sign(&self) -> &[u8] {
        &self.pre
    }
}

/// Resultado de la prefirma descompuesto en sus partes (ADR-0016).
///
/// Una contrafirma prefirma una hoja o más, así que los bloques a firmar son
/// una lista; PAdES y la firma CAdES son el caso de uno.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreSignature {
    pub(crate) session: String,
    pub(crate) blocks: Vec<PreSignBlock>,
    pub(crate) stamp: SessionSeal,
}

impl PreSignature {
    /// Datos trifásicos de la prefirma para la postfirma.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Los bloques que el token tiene que firmar, en el orden en que llegaron.
    pub fn blocks(&self) -> &[PreSignBlock] {
        &self.blocks
    }

    /// Sello de sesión emitido por la prefirma.
    pub fn stamp(&self) -> &SessionSeal {
        &self.stamp
    }

    /// Firma cada bloque con el mismo secreto ya abierto (ADR-0001).
    pub fn signed_one_by_one<E>(
        &self,
        mut sign: impl FnMut(&[u8]) -> Result<Vec<u8>, E>,
    ) -> Result<TokenSignatures, E> {
        let mut signed = Vec::with_capacity(self.blocks.len());
        for block in &self.blocks {
            signed.push((
                block.id.clone(),
                TokenSignature::from_token(sign(&block.pre)?),
            ));
        }
        Ok(TokenSignatures(signed))
    }

    /// Una firma sintética por bloque, para la prefirma en seco.
    pub fn invented_signatures(&self) -> TokenSignatures {
        TokenSignatures(
            self.blocks
                .iter()
                .map(|block| (block.id.clone(), TokenSignature::invented()))
                .collect(),
        )
    }

    /// Junta la prefirma con las firmas del token, solo si el sello volvió intacto (ADR-0016).
    pub fn sealed_with(
        &self,
        signatures: TokenSignatures,
        returned: &SessionSeal,
    ) -> Result<SealedPreSignature, SealMismatch> {
        self.stamp.verify_unchanged(returned)?;
        Ok(SealedPreSignature {
            session: self.session.clone(),
            signed: signatures
                .0
                .into_iter()
                .map(|(id, signature)| SignedBlock {
                    id,
                    pkcs1_b64: signature.to_pkcs1_base64(),
                })
                .collect(),
            stamp: returned.clone(),
        })
    }
}

/// Longitud en bytes de una firma sintética RSA de 2048 bits.
const INVENTED_PKCS1_BYTES: usize = 256;

/// Firma PKCS#1 producida por el token; el puente nunca la recibe suelta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSignature(Vec<u8>);

impl TokenSignature {
    /// Firma tal como la devolvió el token.
    pub fn from_token(raw: Vec<u8>) -> Self {
        Self(raw)
    }

    /// Firma sintética utilizada exclusivamente en la prefirma en seco.
    pub fn invented() -> Self {
        Self(vec![0; INVENTED_PKCS1_BYTES])
    }

    /// Firma cruda tal como la devolvió el token.
    pub fn raw(&self) -> &[u8] {
        &self.0
    }

    /// Firma codificada en Base64 para el campo PK1.
    pub fn to_pkcs1_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.0)
    }
}

/// Las firmas del token de una prefirma: una por bloque, todas con el mismo secreto (ADR-0001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSignatures(Vec<(String, TokenSignature)>);

/// El PKCS#1 de uno de los bloques, con el identificador que la postfirma exige.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedBlock {
    id: String,
    pkcs1_b64: String,
}

impl SignedBlock {
    /// Identificador del bloque dentro de la sesión trifásica.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Firma PKCS#1 en Base64 sobre los atributos firmados de ese bloque.
    pub fn pkcs1_b64(&self) -> &str {
        &self.pkcs1_b64
    }
}

/// Prefirma ya firmada por el token con su sello comprobado: lo único que acepta la postfirma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedPreSignature {
    session: String,
    signed: Vec<SignedBlock>,
    stamp: SessionSeal,
}

impl SealedPreSignature {
    /// Datos trifásicos de la prefirma.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Los PKCS#1 de la sesión, uno por bloque prefirmado.
    pub fn signed(&self) -> &[SignedBlock] {
        &self.signed
    }

    /// Sello que la prefirma emitió y la postfirma exige idéntico.
    pub fn stamp(&self) -> &SessionSeal {
        &self.stamp
    }

    /// Cierra el ciclo con el documento que devolvió la postfirma.
    pub fn completed_with(self, signed_document: Vec<u8>) -> CompletedCycle {
        CompletedCycle { signed_document }
    }
}

/// Ciclo trifásico terminado: solo existe si hubo prefirma, firma y postfirma con el sello intacto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedCycle {
    signed_document: Vec<u8>,
}

impl CompletedCycle {
    /// Bytes del documento firmado.
    pub fn signed_document(&self) -> &[u8] {
        &self.signed_document
    }

    /// Bytes del documento firmado, en propiedad.
    pub fn into_signed_document(self) -> Vec<u8> {
        self.signed_document
    }
}

/// Parámetros para la llamada de prefirma.
#[derive(Clone, Copy, Debug)]
pub struct PreSignRequest<'a> {
    /// Formato de la firma que se pide.
    pub format: Format,
    /// Qué se hace con el documento que entra.
    pub operation: SignatureOperation,
    /// Documento de entrada en Base64.
    pub document_b64: &'a str,
    /// Algoritmo de firma.
    pub algorithm: &'a str,
    /// Cadena de certificados en Base64 separada por punto y coma.
    pub certificate_chain_b64: &'a str,
    /// Parámetros adicionales en formato de propiedades.
    pub extra_params: &'a str,
}

/// Parámetros para la llamada de postfirma (ADR-0016).
#[derive(Clone, Copy, Debug)]
pub struct PostSignRequest<'a> {
    /// Mismo formato que recibió la prefirma.
    pub format: Format,
    /// Mismo documento de entrada que recibió la prefirma, en Base64.
    pub document_b64: &'a str,
    /// Misma cadena de certificados.
    pub certificate_chain_b64: &'a str,
    /// Prefirma firmada por el token con el sello ya comprobado.
    pub sealed: &'a SealedPreSignature,
}

/// Parámetros para acotar un listado con el filtro de la sede.
#[derive(Clone, Copy, Debug)]
pub struct FilterRequest<'a> {
    /// Expresión de la sede en formato de propiedades.
    pub filter_properties: &'a str,
    /// Certificados en Base64 del DER separados por punto y coma.
    pub certificates_b64: &'a str,
}

/// Parámetros para expandir la política de firma que declara la sede.
#[derive(Clone, Copy, Debug)]
pub struct ExpandRequest<'a> {
    /// Parámetros de la sede en formato de propiedades.
    pub extra_params: &'a str,
    /// Formato de firma.
    pub format: &'a str,
}

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
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<LibraryNotFound> for BridgeError {
    fn from(error: LibraryNotFound) -> Self {
        Self::NotFound(error)
    }
}

#[cfg(test)]
mod tests;
