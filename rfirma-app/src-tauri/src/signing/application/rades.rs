//! `Rades`, el `Bridge` de CMS y CAdES hecho en Rust sobre rades: prefirma y postfirma sin puente, y lo que rades no hace rechazado antes del PIN (ADR-0001, ADR-0016).

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ::rades::cades::{self, Certificates, Content, ContentHints, Options};
use ::rades::{
    Certificate, DigestAlgorithm, ExplicitPolicy, ObjectIdentifier, PolicyHashAlgorithm,
    PolicyQualifier, SignaturePolicy, SignatureValue, SigningTime,
};
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignBlock, PreSignRequest, PreSignature,
    SignatureOperation,
};
use crate::signing::domain::SessionSeal;
use crate::signing::ports::Bridge;
use crate::site::domain::protocol::pairs_of;

const CHAIN_SEPARATOR: char = ';';
const MODE: &str = "mode";
const ONLY_THE_SIGNER: &str = "includeOnlySignningCertificate";
const PRECALCULATED_HASH: &str = "precalculatedHashAlgorithm";
const CONTENT_TYPE_OID: &str = "contentTypeOid";
const CONTENT_DESCRIPTION: &str = "contentDescription";
const INCLUDE_MIME_TYPE: &str = "includeMimeTypeAttribute";
const MIME_TYPE: &str = "mimeType";
const ALWAYS_TRUE: [&str; 2] = ["signingCertificateV2", "includeSigningTimeAttribute"];
const PROTOCOL_ONLY: [&str; 3] = ["format", "headless", "mandatoryCertSelection"];
const CMS_KEYS: [&str; 2] = [MODE, ONLY_THE_SIGNER];
const EXP_POLICY: &str = "expPolicy";
const POLICY_IDENTIFIER: &str = "policyIdentifier";
const POLICY_HASH: &str = "policyIdentifierHash";
const POLICY_HASH_ALGORITHM: &str = "policyIdentifierHashAlgorithm";
const POLICY_QUALIFIER: &str = "policyQualifier";
const OID_URN: &str = "urn:oid:";
const CADES_KEYS: [&str; 10] = [
    EXP_POLICY,
    POLICY_IDENTIFIER,
    POLICY_HASH,
    POLICY_HASH_ALGORITHM,
    POLICY_QUALIFIER,
    PRECALCULATED_HASH,
    CONTENT_TYPE_OID,
    CONTENT_DESCRIPTION,
    INCLUDE_MIME_TYPE,
    MIME_TYPE,
];

/// El `Bridge` de rades, con el reloj del que toma la hora de firma.
pub struct Rades {
    unix_seconds: fn() -> u64,
}

impl Rades {
    /// El de producción: la hora de firma, del reloj del sistema.
    pub fn on_the_system_clock() -> Self {
        Self {
            unix_seconds: || {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |since| since.as_secs())
            },
        }
    }

    /// Uno que firma siempre a la hora que da `unix_seconds`.
    pub fn at(unix_seconds: fn() -> u64) -> Self {
        Self { unix_seconds }
    }

    /// Si rades firma ese formato en lugar del puente.
    pub fn signs(format: Format) -> bool {
        matches!(format, Format::Cms | Format::Cades)
    }
}

/// Lo que la postfirma necesita para rehacer la fase 1, ya que no recibe ni `extraParams` ni algoritmo.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    signing_time: u64,
    algorithm: String,
    extra_params: String,
}

impl Bridge for Rades {
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError> {
        if request.operation != SignatureOperation::Sign {
            return Err(BridgeError::UnsupportedOperation(
                request.format,
                request.operation,
            ));
        }
        let session = Session {
            signing_time: (self.unix_seconds)(),
            algorithm: request.algorithm.to_owned(),
            extra_params: request.extra_params.to_owned(),
        };
        let data_to_sign = phase_one(
            request.format,
            &session,
            request.document_b64,
            request.certificate_chain_b64,
        )?;
        let session = serde_json::to_string(&session)
            .map_err(|error| BridgeError::Failed(error.to_string()))?;
        Ok(PreSignature {
            stamp: seal_of(&session),
            session,
            blocks: vec![PreSignBlock {
                id: hex_sha256(&data_to_sign),
                pre: data_to_sign,
            }],
        })
    }

    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError> {
        let sealed = request.sealed;
        if *sealed.stamp() != seal_of(sealed.session()) {
            return Err(BridgeError::Failed(
                "la sesión no es la que selló la prefirma".to_owned(),
            ));
        }
        let session: Session = serde_json::from_str(sealed.session())
            .map_err(|error| BridgeError::MalformedResponse(error.to_string()))?;
        let pending = pending_of(
            request.format,
            &session,
            request.document_b64,
            request.certificate_chain_b64,
        )?;
        let [signed] = sealed.signed() else {
            return Err(BridgeError::MalformedResponse(format!(
                "una firma CMS lleva un PKCS#1, y llegaron {}",
                sealed.signed().len()
            )));
        };
        let [data_to_sign] = pending.data_to_sign() else {
            return Err(BridgeError::Failed(
                "rades pide más de un bloque".to_owned(),
            ));
        };
        if signed.id() != hex_sha256(data_to_sign.bytes()) {
            return Err(BridgeError::Failed(
                "los datos a firmar no son los que firmó el token".to_owned(),
            ));
        }
        let value = decoded(signed.pkcs1_b64(), "el PKCS#1")?;
        pending
            .complete(&[SignatureValue::new(value)])
            .map_err(rades_failed)
    }
}

fn phase_one(
    format: Format,
    session: &Session,
    document_b64: &str,
    chain_b64: &str,
) -> Result<Vec<u8>, BridgeError> {
    let pending = pending_of(format, session, document_b64, chain_b64)?;
    match pending.data_to_sign() {
        [only] => Ok(only.bytes().to_vec()),
        blocks => Err(BridgeError::Failed(format!(
            "rades pide {} bloques y {format} firma uno",
            blocks.len()
        ))),
    }
}

fn pending_of(
    format: Format,
    session: &Session,
    document_b64: &str,
    chain_b64: &str,
) -> Result<cades::PendingSignature, BridgeError> {
    let algorithm = rades_algorithm(&session.algorithm)?;
    let parameters = admitted(format, &session.extra_params)?;
    let document = decoded(document_b64, "el documento")?;
    let mut chain = chain_of(chain_b64)?;
    if chain.is_empty() {
        return Err(BridgeError::InvalidArgument("la cadena de certificados"));
    }
    let signer = chain.remove(0);
    let content = content_of(&parameters, &document)?;
    let certificates = if parameters
        .get(ONLY_THE_SIGNER)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        Certificates::Signer
    } else {
        Certificates::Chain(chain)
    };
    let signing_time =
        SigningTime::from_unix_seconds(session.signing_time).map_err(rades_failed)?;
    match format {
        Format::Cades => {
            let options = options_of(&parameters)?.certificates(certificates);
            cades::sign(content, &signer, algorithm, signing_time, &options)
        }
        _ => cades::sign_bare(content, &signer, algorithm, signing_time, &certificates),
    }
    .map_err(rades_failed)
}

fn content_of<'a>(
    parameters: &BTreeMap<String, String>,
    document: &'a [u8],
) -> Result<Content<'a>, BridgeError> {
    if let Some(name) = parameters.get(PRECALCULATED_HASH) {
        return Ok(Content::Digest {
            algorithm: digest_algorithm(name)?,
            digest: document,
        });
    }
    match parameters.get(MODE).map(String::as_str) {
        None | Some("implicit") => Ok(Content::Attached(document)),
        Some("explicit") => Ok(Content::Detached(document)),
        Some(other) => Err(BridgeError::NotSupported(format!("{MODE}={other}"))),
    }
}

fn digest_algorithm(name: &str) -> Result<DigestAlgorithm, BridgeError> {
    match name.replace('-', "").to_ascii_uppercase().as_str() {
        "SHA256" => Ok(DigestAlgorithm::Sha256),
        "SHA384" => Ok(DigestAlgorithm::Sha384),
        "SHA512" => Ok(DigestAlgorithm::Sha512),
        _ => Err(BridgeError::NotSupported(format!(
            "{PRECALCULATED_HASH}={name}"
        ))),
    }
}

fn options_of(parameters: &BTreeMap<String, String>) -> Result<Options, BridgeError> {
    let mut options = Options::default();
    if let Some(policy) = policy_of(parameters)? {
        options = options.signature_policy(policy);
    }
    if let Some(oid) = parameters.get(CONTENT_TYPE_OID) {
        let content_type = ObjectIdentifier::new(oid)
            .map_err(|_| BridgeError::NotSupported(format!("{CONTENT_TYPE_OID}={oid}")))?;
        let description = parameters.get(CONTENT_DESCRIPTION).cloned();
        options = options.content_hints(ContentHints::new(description, content_type));
    }
    let wants_mime_type = parameters
        .get(INCLUDE_MIME_TYPE)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    if let Some(mime_type) = parameters.get(MIME_TYPE).filter(|_| wants_mime_type) {
        options = options.mime_type(mime_type);
    }
    Ok(options)
}

fn policy_of(
    parameters: &BTreeMap<String, String>,
) -> Result<Option<SignaturePolicy>, BridgeError> {
    let Some(identifier) = parameters.get(POLICY_IDENTIFIER) else {
        if let Some(qualifier) = parameters.get(POLICY_QUALIFIER) {
            return Err(BridgeError::NotSupported(format!(
                "{POLICY_QUALIFIER}={qualifier} sin {POLICY_IDENTIFIER}"
            )));
        }
        return match parameters.get(EXP_POLICY).map(String::as_str) {
            None => Ok(None),
            Some("FirmaAGE" | "FirmaAGE19") => Ok(Some(SignaturePolicy::age_v1_9())),
            Some(other) => Err(BridgeError::NotSupported(format!("{EXP_POLICY}={other}"))),
        };
    };
    let missing = |key: &str| BridgeError::NotSupported(format!("{POLICY_IDENTIFIER} sin {key}"));
    let oid = ObjectIdentifier::new(identifier.strip_prefix(OID_URN).unwrap_or(identifier))
        .map_err(|_| BridgeError::NotSupported(format!("{POLICY_IDENTIFIER}={identifier}")))?;
    let hash = parameters
        .get(POLICY_HASH)
        .ok_or_else(|| missing(POLICY_HASH))?;
    let algorithm = parameters
        .get(POLICY_HASH_ALGORITHM)
        .ok_or_else(|| missing(POLICY_HASH_ALGORITHM))?;
    let hash = decoded(hash, "el hash de la política")?;
    let mut policy = ExplicitPolicy::new(oid, policy_hash_algorithm(algorithm)?, hash);
    if let Some(url) = parameters.get(POLICY_QUALIFIER) {
        policy = policy.qualifier(PolicyQualifier::Spuri(url.clone()));
    }
    Ok(Some(policy.into()))
}

fn policy_hash_algorithm(name: &str) -> Result<PolicyHashAlgorithm, BridgeError> {
    let normalized = name.to_ascii_lowercase().replace('-', "");
    let tail = normalized.rsplit(['#', '/']).next().unwrap_or_default();
    match tail {
        "sha1" => Ok(PolicyHashAlgorithm::Sha1),
        "sha256" => Ok(PolicyHashAlgorithm::Digest(DigestAlgorithm::Sha256)),
        "sha384" => Ok(PolicyHashAlgorithm::Digest(DigestAlgorithm::Sha384)),
        "sha512" => Ok(PolicyHashAlgorithm::Digest(DigestAlgorithm::Sha512)),
        _ => Err(BridgeError::NotSupported(format!(
            "{POLICY_HASH_ALGORITHM}={name}"
        ))),
    }
}

fn admitted(format: Format, extra_params: &str) -> Result<BTreeMap<String, String>, BridgeError> {
    let parameters: BTreeMap<String, String> = pairs_of(extra_params).into_iter().collect();
    let cades = format == Format::Cades;
    for (key, value) in &parameters {
        let key_str = key.as_str();
        let known = CMS_KEYS.contains(&key_str)
            || (cades
                && (CADES_KEYS.contains(&key_str)
                    || PROTOCOL_ONLY.contains(&key_str)
                    || ALWAYS_TRUE.contains(&key_str)));
        if !known {
            return Err(BridgeError::NotSupported(key.clone()));
        }
        if ALWAYS_TRUE.contains(&key_str) && !value.eq_ignore_ascii_case("true") {
            return Err(BridgeError::NotSupported(format!("{key}={value}")));
        }
    }
    Ok(parameters)
}

fn rades_algorithm(name: &str) -> Result<::rades::SignatureAlgorithm, BridgeError> {
    use ::rades::SignatureAlgorithm::{Ecdsa, RsaPkcs1v15, RsaPss};
    use DigestAlgorithm::{Sha256, Sha384, Sha512};
    let not_supported = || BridgeError::NotSupported(name.to_owned());
    Ok(
        match SignatureAlgorithm::from_name(name).ok_or_else(not_supported)? {
            SignatureAlgorithm::Sha1Rsa | SignatureAlgorithm::Sha1Ecdsa => {
                return Err(not_supported())
            }
            SignatureAlgorithm::Sha256Rsa => RsaPkcs1v15(Sha256),
            SignatureAlgorithm::Sha384Rsa => RsaPkcs1v15(Sha384),
            SignatureAlgorithm::Sha512Rsa => RsaPkcs1v15(Sha512),
            SignatureAlgorithm::Sha256RsaPss => RsaPss(Sha256),
            SignatureAlgorithm::Sha384RsaPss => RsaPss(Sha384),
            SignatureAlgorithm::Sha512RsaPss => RsaPss(Sha512),
            SignatureAlgorithm::Sha256Ecdsa => Ecdsa(Sha256),
            SignatureAlgorithm::Sha384Ecdsa => Ecdsa(Sha384),
            SignatureAlgorithm::Sha512Ecdsa => Ecdsa(Sha512),
        },
    )
}

fn chain_of(chain_b64: &str) -> Result<Vec<Certificate>, BridgeError> {
    chain_b64
        .split(CHAIN_SEPARATOR)
        .filter(|one| !one.is_empty())
        .map(|one| {
            Certificate::from_der(&decoded(one, "un certificado de la cadena")?)
                .map_err(rades_failed)
        })
        .collect()
}

/// El sello se deriva de la sesión, así que alterarla entre las dos fases lo rompe (ADR-0016).
fn seal_of(session: &str) -> SessionSeal {
    SessionSeal::from_bridge(hex_sha256(session.as_bytes()))
}

fn hex_sha256(bytes: &[u8]) -> String {
    openssl::sha::sha256(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decoded(b64: &str, what: &'static str) -> Result<Vec<u8>, BridgeError> {
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| BridgeError::InvalidArgument(what))
}

fn rades_failed(error: ::rades::Error) -> BridgeError {
    BridgeError::Failed(error.to_string())
}

#[cfg(test)]
mod tests;
