//! Ciclo trifásico de firma PAdES: prefirma en Java, firma en Rust y postfirma en Java (ADR-0001, ADR-0016).

use base64::Engine;

use crate::identity::adapters::pkcs11;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
use crate::signing::domain::bridge::{BridgeError, PostSignRequest, PreSignRequest, PreSignature};
use crate::signing::domain::{
    to_java_properties, AdmissibleDocument, CompletedCycle, Refusal, SealMismatch, SessionSeal,
    SignatureConfig,
};
use crate::signing::ports::Bridge;

pub use crate::signing::domain::TokenSignature;

/// Conjunto vacío de parámetros adicionales para firmas locales.
pub static NOTHING_FROM_A_SITE: std::collections::BTreeMap<String, String> =
    std::collections::BTreeMap::new();

/// Algoritmo de firma compatible con el mecanismo del token PKCS#11 (ADR-0001).
pub const ALGORITHM: &str = "SHA256withRSA";

const CHAIN_SEPARATOR: &str = ";";

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Lo que hace falta para abrir un ciclo de firma.
#[derive(Clone, Copy, Debug)]
pub struct SigningRequest<'a> {
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
    pdf_b64: String,
    chain_b64: String,
    presigned: PreSignature,
    certificate: CertificateRef,
    already_signed_before: bool,
}

/// Fase 1: ejecuta la prefirma PAdES enviando documento y parámetros al puente.
pub fn presign(bridge: &impl Bridge, request: SigningRequest<'_>) -> Result<OpenCycle, CycleError> {
    let pdf_b64 = base64(request.document.bytes());
    let chain_b64 = request
        .chain
        .iter()
        .map(|der| base64(der))
        .collect::<Vec<_>>()
        .join(CHAIN_SEPARATOR);
    let extra_params = to_java_properties(&super::policies::merged_with(
        request.from_the_site.clone(),
        request.config.extra_params(),
    ));

    let presigned = bridge.presign(PreSignRequest {
        pdf_b64: &pdf_b64,
        algorithm: ALGORITHM,
        certificate_chain_b64: &chain_b64,
        extra_params: &extra_params,
    })?;

    Ok(OpenCycle {
        pdf_b64,
        chain_b64,
        presigned,
        certificate: request.certificate.clone(),
        already_signed_before: request.document.already_signed(),
    })
}

impl OpenCycle {
    /// Bytes que el token debe firmar, sin hashear.
    pub fn to_be_signed(&self) -> &[u8] {
        self.presigned.pre_sign()
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

    /// Fase 2: firma los bytes en el token PKCS#11 (ADR-0001).
    pub fn sign_on_token(&self, pin: &str) -> Result<TokenSignature, CycleError> {
        let signature = pkcs11::sign(&self.certificate, pin, self.presigned.pre_sign())?;
        Ok(TokenSignature::from_token(signature))
    }

    /// Fase 3: sella la prefirma con la firma del token y ensambla el PDF firmado (ADR-0016).
    pub fn postsign(
        &self,
        bridge: &impl Bridge,
        signature: &TokenSignature,
        returned: &SessionSeal,
    ) -> Result<CompletedCycle, CycleError> {
        let sealed = self.presigned.sealed_with(signature, returned)?;
        let pdf = bridge.postsign(PostSignRequest {
            pdf_b64: &self.pdf_b64,
            certificate_chain_b64: &self.chain_b64,
            sealed: &sealed,
        })?;
        Ok(sealed.completed_with(pdf))
    }
}

impl std::fmt::Debug for OpenCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenCycle")
            .field("certificate", &self.certificate)
            .field("to_be_signed_bytes", &self.presigned.pre_sign().len())
            .field("cosigning", &self.already_signed_before)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
