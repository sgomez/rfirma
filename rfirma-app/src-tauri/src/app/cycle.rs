//! Ciclo trifásico de firma PAdES: prefirma en Java, firma en Rust y postfirma en Java (ADR-0001, ADR-0016).

use base64::Engine;

use crate::ffi::{BridgeError, NativeBridge, PostSignRequest, PreSignRequest};
use crate::pkcs11::{self, CertificateRef, TokenError};
use crate::signing::{
    to_java_properties, AdmissibleDocument, Refusal, SealMismatch, SessionSeal, SignatureConfig,
};

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

/// Cuántos bytes ocupa el `PK1` inventado de [`TokenSignature::invented`].
///
/// Son los de una firma RSA de 2048 bits, que es la de los certificados de
/// firma que rFirma maneja. La longitud no cambia lo que se pinta —el hueco que
/// el PAdES reserva para la firma se rellena igual—, así que aquí no hace falta
/// leerle el módulo al certificado ni hacer criptografía RSA en Rust, que es
/// justo lo que el proyecto delega en el token.
const INVENTED_PKCS1_BYTES: usize = 256;

/// Firma producida por el token PKCS#11.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSignature(Vec<u8>);

impl TokenSignature {
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
        base64(&self.0)
    }
}

/// Ciclo de firma iniciado a la espera de la firma del token (ADR-0016).
pub struct OpenCycle {
    pdf_b64: String,
    chain_b64: String,
    session: String,
    to_be_signed: Vec<u8>,
    seal: SessionSeal,
    certificate: CertificateRef,
    already_signed_before: bool,
}

/// Fase 1: ejecuta la prefirma PAdES enviando documento y parámetros al puente.
pub fn presign(
    bridge: &NativeBridge,
    request: SigningRequest<'_>,
) -> Result<OpenCycle, CycleError> {
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
        session: presigned.session().to_owned(),
        to_be_signed: presigned.pre_sign().to_vec(),
        seal: presigned.stamp().clone(),
        certificate: request.certificate.clone(),
        already_signed_before: request.document.already_signed(),
    })
}

impl OpenCycle {
    /// Bytes que el token debe firmar, sin hashear.
    pub fn to_be_signed(&self) -> &[u8] {
        &self.to_be_signed
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
        self.seal.clone()
    }

    /// Fase 2: firma los bytes en el token PKCS#11 (ADR-0001).
    pub fn sign_on_token(&self, pin: &str) -> Result<TokenSignature, CycleError> {
        let signature = pkcs11::sign(&self.certificate, pin, &self.to_be_signed)?;
        Ok(TokenSignature(signature))
    }

    /// Fase 3: verifica el sello de sesión y ensambla el PDF firmado (ADR-0016).
    pub fn postsign(
        &self,
        bridge: &NativeBridge,
        signature: &TokenSignature,
        returned: &SessionSeal,
    ) -> Result<Vec<u8>, CycleError> {
        self.seal.verify_unchanged(returned)?;

        Ok(bridge.postsign(PostSignRequest {
            pdf_b64: &self.pdf_b64,
            certificate_chain_b64: &self.chain_b64,
            stamp: returned,
            session: &self.session,
            pkcs1_b64: &signature.to_pkcs1_base64(),
        })?)
    }
}

impl std::fmt::Debug for OpenCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenCycle")
            .field("certificate", &self.certificate)
            .field("to_be_signed_bytes", &self.to_be_signed.len())
            .field("cosigning", &self.already_signed_before)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionSeal, TokenSignature, ALGORITHM};
    use std::collections::BTreeSet;

    const BORDER: &str = include_str!("../ffi.rs");

    fn production_half(source: &str) -> &str {
        source
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(source)
    }

    fn identifiers(source: &str) -> BTreeSet<&str> {
        source
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|word| !word.is_empty())
            .collect()
    }

    fn entry_points() -> BTreeSet<String> {
        BORDER
            .match_indices("autofirma_")
            .map(|(start, _)| {
                BORDER[start..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect()
            })
            .collect()
    }

    #[test]
    fn java_has_no_entry_point_for_the_signing_phase() {
        let expected: BTreeSet<String> = [
            "autofirma_expand_extra_params",
            "autofirma_filter_certificates",
            "autofirma_free_string",
            "autofirma_pades_postsign",
            "autofirma_pades_presign",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        assert_eq!(
            entry_points(),
            expected,
            "la frontera con Java ha cambiado de puntos de entrada: si uno de \
             ellos firma, la clave privada ha entrado en el isolate (ADR-0001)"
        );
    }

    #[test]
    fn the_pin_has_no_way_across_the_border() {
        let words = identifiers(BORDER);
        for forbidden in ["pin", "private_key", "AuthPin", "cryptoki"] {
            assert!(
                !words.contains(forbidden),
                "«{forbidden}» aparece en la frontera FFI: la fase 2 se estaría \
                 delegando a Java, contra el ADR-0001"
            );
        }
    }

    #[test]
    fn only_the_pkcs11_module_talks_to_the_token() {
        let cycle = production_half(include_str!("cycle.rs"));

        assert!(cycle.contains("pkcs11::sign(&self.certificate, pin"));
        assert!(!identifiers(cycle).contains("sign_on_bridge"));
        assert_eq!(cycle.matches("bridge.").count(), 2);
    }

    #[test]
    fn the_algorithm_matches_the_pkcs11_mechanism() {
        let token_side = include_str!("../pkcs11/mod.rs");

        assert_eq!(ALGORITHM, "SHA256withRSA");
        assert!(token_side.contains("Mechanism::Sha256RsaPkcs"));
    }

    #[test]
    fn the_signature_travels_to_the_postsign_in_base64() {
        let signature = TokenSignature(vec![0x30, 0x82, 0x01, 0x00]);

        assert_eq!(signature.raw(), [0x30, 0x82, 0x01, 0x00]);
        assert_eq!(signature.to_pkcs1_base64(), "MIIBAA==");
    }

    #[test]
    fn a_seal_that_came_back_changed_is_refused_before_anything_else() {
        let issued = SessionSeal::from_bridge("el sello de la prefirma");
        let tampered = SessionSeal::from_bridge("el sello de la prefirma.");

        assert!(issued.verify_unchanged(&tampered).is_err());
        assert!(issued.verify_unchanged(&issued.clone()).is_ok());
    }

    #[test]
    fn the_postsign_compares_the_seal_before_crossing_the_border() {
        let cycle = production_half(include_str!("cycle.rs"));
        let body = cycle
            .split_once("pub fn postsign(")
            .expect("la postfirma sigue aquí")
            .1;
        let check = body
            .find("verify_unchanged")
            .expect("la postfirma comprueba el sello");
        let crossing = body.find("bridge.postsign").expect("y luego cruza");

        assert!(
            check < crossing,
            "el sello se comprueba después de cruzar: el PDF ya saldría mal"
        );
    }
}
