//! Ciclo trifásico de firma, parametrizado por formato: prefirma en Java, firma en Rust y postfirma en Java (ADR-0001, ADR-0016).

use base64::Engine;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
use crate::signing::domain::bridge::{
    BridgeError, PostSignRequest, PreSignBlock, PreSignRequest, PreSignature, SignatureOperation,
};
use crate::signing::domain::{
    to_java_properties, AdmissibleDocument, CompletedCycle, Format, Refusal, SealMismatch,
    SessionSeal, SignatureConfig,
};
use crate::signing::ports::{Bridge, Signer};

use crate::signing::domain::TokenSignatures;

/// Conjunto vacío de parámetros adicionales para firmas locales.
pub static NOTHING_FROM_A_SITE: std::collections::BTreeMap<String, String> =
    std::collections::BTreeMap::new();

/// El algoritmo que rFirma pide hoy, el mismo para el puente y para el token (ADR-0001).
pub const ALGORITHM: SignatureAlgorithm = SignatureAlgorithm::Sha256Rsa;

const CHAIN_SEPARATOR: &str = ";";

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Lo que hace falta para abrir un ciclo de firma.
#[derive(Clone, Copy, Debug)]
pub struct SigningRequest<'a> {
    /// Formato de la firma que se pide.
    pub format: Format,
    /// Qué se hace con el documento: firmarlo, cofirmarlo o contrafirmarlo.
    pub operation: SignatureOperation,
    /// Documento admitido para firmar.
    pub document: AdmissibleDocument<'a>,
    /// Cadena de certificados en DER con el del firmante primero.
    pub chain: &'a [Vec<u8>],
    /// Configuración de la firma visible y parámetros.
    pub config: &'a SignatureConfig,
    /// Parámetros adicionales declarados por la sede.
    pub from_the_site: &'a std::collections::BTreeMap<String, String>,
    /// Referencia al certificado con el que se firmará.
    pub certificate: &'a CertificateRef,
}

/// Errores posibles durante el ciclo trifásico de firma (ADR-0016).
#[derive(Debug)]
pub enum CycleError {
    /// El documento no se puede firmar, y se sabía antes de pedir el PIN.
    Inadmissible(Refusal),
    /// La prefirma o la postfirma han fallado al otro lado de la frontera.
    Bridge(BridgeError),
    /// El token ha rechazado la operación de firma.
    Token(TokenError),
    /// El sello devuelto no coincide con el emitido por la prefirma.
    Seal(SealMismatch),
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inadmissible(refusal) => write!(f, "{refusal}"),
            Self::Bridge(error) => write!(f, "{error}"),
            Self::Token(error) => write!(f, "{error}"),
            Self::Seal(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CycleError {}

impl From<Refusal> for CycleError {
    fn from(refusal: Refusal) -> Self {
        Self::Inadmissible(refusal)
    }
}

impl From<BridgeError> for CycleError {
    fn from(error: BridgeError) -> Self {
        Self::Bridge(error)
    }
}

impl From<TokenError> for CycleError {
    fn from(error: TokenError) -> Self {
        Self::Token(error)
    }
}

impl From<SealMismatch> for CycleError {
    fn from(error: SealMismatch) -> Self {
        Self::Seal(error)
    }
}

/// Ciclo de firma iniciado a la espera de la firma del token (ADR-0016).
pub struct OpenCycle {
    format: Format,
    document_b64: String,
    chain_b64: String,
    presigned: PreSignature,
    certificate: CertificateRef,
    already_signed_before: bool,
}

/// Lo que rFirma añade de su cosecha: recuadro y rúbrica, que solo lee un firmador PDF.
fn added_by_rfirma(
    format: Format,
    config: &SignatureConfig,
) -> std::collections::BTreeMap<String, String> {
    match format {
        Format::Pades => config.extra_params(),
        _ => std::collections::BTreeMap::new(),
    }
}

/// Fase 1: ejecuta la prefirma enviando formato, documento y parámetros al puente.
pub fn presign<B: Bridge + ?Sized>(
    bridge: &B,
    request: SigningRequest<'_>,
) -> Result<OpenCycle, CycleError> {
    let document_b64 = base64(request.document.bytes());
    let chain_b64 = request
        .chain
        .iter()
        .map(|der| base64(der))
        .collect::<Vec<_>>()
        .join(CHAIN_SEPARATOR);
    let extra_params = to_java_properties(&crate::signing::domain::merged_with(
        request.from_the_site.clone(),
        added_by_rfirma(request.format, request.config),
    ));

    let presigned = bridge.presign(PreSignRequest {
        format: request.format,
        operation: request.operation,
        document_b64: &document_b64,
        algorithm: ALGORITHM.name(),
        certificate_chain_b64: &chain_b64,
        extra_params: &extra_params,
    })?;

    Ok(OpenCycle {
        format: request.format,
        document_b64,
        chain_b64,
        presigned,
        certificate: request.certificate.clone(),
        already_signed_before: request.document.already_signed(),
    })
}

impl OpenCycle {
    /// Bloques que el token debe firmar, sin hashear; una contrafirma trae más de uno.
    pub fn to_be_signed(&self) -> &[PreSignBlock] {
        self.presigned.blocks()
    }

    /// Certificado con el que se abrió el ciclo.
    pub fn certificate(&self) -> &CertificateRef {
        &self.certificate
    }

    /// Indica si el documento ya contenía firmas previas.
    pub fn is_cosigning(&self) -> bool {
        self.already_signed_before
    }

    /// Copia del sello de sesión para transportarlo a la postfirma (ADR-0016).
    pub fn seal_in_transit(&self) -> SessionSeal {
        self.presigned.stamp().clone()
    }

    /// Fase 2: firma cada bloque en el token PKCS#11, con el secreto pedido una sola vez (ADR-0001).
    pub fn sign_on_token(
        &self,
        signer: &dyn Signer,
        pin: &str,
    ) -> Result<TokenSignatures, CycleError> {
        Ok(self
            .presigned
            .signed_one_by_one(|pre| signer.sign(&self.certificate, pin, ALGORITHM, pre))?)
    }

    /// Las firmas sintéticas de la prefirma en seco, una por bloque.
    pub fn invented_signatures(&self) -> TokenSignatures {
        self.presigned.invented_signatures()
    }

    /// Fase 3: sella la prefirma con las firmas del token y ensambla el documento firmado (ADR-0016).
    pub fn postsign<B: Bridge + ?Sized>(
        &self,
        bridge: &B,
        signatures: TokenSignatures,
        returned: &SessionSeal,
    ) -> Result<CompletedCycle, CycleError> {
        let sealed = self.presigned.sealed_with(signatures, returned)?;
        let signed_document = bridge.postsign(PostSignRequest {
            format: self.format,
            document_b64: &self.document_b64,
            certificate_chain_b64: &self.chain_b64,
            sealed: &sealed,
        })?;
        Ok(sealed.completed_with(signed_document))
    }
}

impl std::fmt::Debug for OpenCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenCycle")
            .field("format", &self.format)
            .field("certificate", &self.certificate)
            .field("blocks_to_be_signed", &self.presigned.blocks().len())
            .field("cosigning", &self.already_signed_before)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
