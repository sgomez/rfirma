//! El llavero de macOS por Security.framework: las identidades se listan sin abrir su clave, y el resumen lo firma el sistema (ADR-0001).

use core_foundation::base::{CFType, TCFType};
use core_foundation::error::CFError;
use core_foundation::string::CFString;
use openssl::hash::{hash, MessageDigest};
use security_framework::certificate::SecCertificate;
use security_framework::identity::SecIdentity;
use security_framework::import_export::Pkcs12ImportOptions;
use security_framework::item::{ItemClass, ItemSearchOptions, Limit, Reference, SearchResult};
use security_framework::key::{Algorithm, SecKey};
use security_framework::os::macos::keychain::{CreateOptions, SecKeychain};
use security_framework_sys::item::kSecAttrTokenID;

use super::SEARCH_LIST;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::digest;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::store::{Store, MACOS_STORE_PREFIX};

/// La etiqueta de «token» de los certificados del llavero.
pub const TOKEN_LABEL: &str = "Keychain";

const SYSTEM_ROOTS: &str = "/System/Library/Keychains/SystemRootCertificates.keychain";

const ITEM_NOT_FOUND: i32 = -25300;

/// Las identidades del llavero: los certificados con su clave privada.
pub fn signable_certificates(store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
    let _no_prompt = SecKeychain::disable_user_interaction().map_err(|error| failure(&error))?;
    Ok(found(ItemClass::identity(), &scope_of(store)?)?
        .into_iter()
        .filter_map(|reference| match reference {
            Reference::Identity(identity) => certificate_of_identity(store, &identity),
            _ => None,
        })
        .collect())
}

/// Todos los certificados del llavero y las raíces del sistema: con ellos se completa la cadena.
pub fn every_certificate(store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
    let _no_prompt = SecKeychain::disable_user_interaction().map_err(|error| failure(&error))?;
    let scope = scope_of(store)?;
    let mut certificates = found(ItemClass::certificate(), &scope)?;
    if scope.is_none() {
        if let Ok(roots) = SecKeychain::open(SYSTEM_ROOTS) {
            certificates
                .extend(found(ItemClass::certificate(), &Some(vec![roots])).unwrap_or_default());
        }
    }
    Ok(certificates
        .into_iter()
        .filter_map(|reference| match reference {
            Reference::Certificate(certificate) => {
                Some(certificate_in(store, &certificate.to_der(), None))
            }
            _ => None,
        })
        .collect())
}

/// Comprueba que la clave del certificado es de la clase que exige el algoritmo.
pub fn offers(reference: &CertificateRef, algorithm: SignatureAlgorithm) -> Result<(), TokenError> {
    let (certificate, _) = identity_of(reference)?;
    if certificate.key_kind() == Some(algorithm.key_kind()) {
        return Ok(());
    }
    Err(TokenError::new(
        Situation::MechanismNotOffered,
        format!(
            "la clave de {} no sirve para {}",
            reference.label(),
            algorithm.name()
        ),
    ))
}

/// Resume `data` y firma el resumen con la clave del certificado; macOS pide permiso o el PIN si hace falta.
pub fn sign(
    reference: &CertificateRef,
    algorithm: SignatureAlgorithm,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    let (_, identity) = identity_of(reference)?;
    let key = identity.private_key().map_err(|error| failure(&error))?;
    let digest = digest::digest(algorithm, data)?;
    key.create_signature(over_a_digest(algorithm), &digest)
        .map_err(|error| refused(&error))
}

/// Mete el `.p12` en el llavero de `store`, o en el de inicio de sesión si es la lista de búsqueda, y da las huellas de sus identidades; antes lo prueba aparte y, si no sirve, no deja nada.
pub fn import_pkcs12(
    store: &Store,
    pkcs12: &[u8],
    password: &str,
) -> Result<Vec<Vec<u8>>, TokenError> {
    let thumbprints = only_if_it_brings_a_usable_identity(pkcs12, password)?;
    let mut options = Pkcs12ImportOptions::new();
    options.passphrase(password);
    if let Some(mut keychains) = scope_of(store)? {
        options.keychain(keychains.remove(0));
    }
    options
        .import(pkcs12)
        .map(|_| thumbprints)
        .map_err(|error| rejected_pkcs12(&error))
}

/// Quita del llavero el certificado de `reference` y su clave privada.
pub fn remove(reference: &CertificateRef) -> Result<(), TokenError> {
    let (_, identity) = identity_of(reference)?;
    identity.delete().map_err(|error| failure(&error))
}

fn only_if_it_brings_a_usable_identity(
    pkcs12: &[u8],
    password: &str,
) -> Result<Vec<Vec<u8>>, TokenError> {
    let scratch = ScratchKeychain::create()?;
    let imported = Pkcs12ImportOptions::new()
        .passphrase(password)
        .keychain(scratch.keychain.clone())
        .import(pkcs12)
        .map_err(|error| rejected_pkcs12(&error))?;
    let mut thumbprints = Vec::new();
    for identity in imported.iter().filter_map(|item| item.identity.as_ref()) {
        let certificate = identity.certificate().map_err(|error| failure(&error))?;
        let der = certificate.to_der();
        let candidate = certificate_in(&Store::module(""), &der, None);
        if candidate.key_kind().is_none() {
            return Err(TokenError::new(
                Situation::KeyKindUnsupported,
                format!(
                    "{}: la clave no es RSA ni de curva eliptica",
                    candidate.reference().label()
                ),
            ));
        }
        thumbprints.push(candidate.reference().cka_id().unwrap_or_default().to_vec());
    }
    if !thumbprints.is_empty() {
        return Ok(thumbprints);
    }
    Err(TokenError::new(
        Situation::Pkcs12NoPrivateKey,
        "el fichero no ha dejado ningun certificado con clave privada dentro",
    ))
}

fn rejected_pkcs12(error: &security_framework::base::Error) -> TokenError {
    match error.code() {
        WRONG_PKCS12_PASSWORD | AUTH_FAILED => TokenError::new(
            Situation::IncorrectPkcs12Password,
            format!(
                "la contrasena del .p12 no es la correcta ({})",
                error.code()
            ),
        ),
        _ => failure(error),
    }
}

const WRONG_PKCS12_PASSWORD: i32 = -25264;
const AUTH_FAILED: i32 = -25293;

/// Un llavero desechable donde probar un `.p12` sin tocar el de la persona.
struct ScratchKeychain {
    directory: std::path::PathBuf,
    keychain: SecKeychain,
}

impl ScratchKeychain {
    fn create() -> Result<Self, TokenError> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let directory =
            std::env::temp_dir().join(format!("rfirma-p12-{}-{unique}", std::process::id()));
        std::fs::create_dir(&directory).map_err(|error| {
            TokenError::new(
                Situation::Unknown,
                format!("no se ha podido preparar un llavero temporal: {error}"),
            )
        })?;
        let created = CreateOptions::new()
            .password(SCRATCH_PASSWORD)
            .create(directory.join("scratch.keychain-db"));
        match created {
            Ok(keychain) => Ok(Self {
                directory,
                keychain,
            }),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&directory);
                Err(failure(&error))
            }
        }
    }
}

impl Drop for ScratchKeychain {
    fn drop(&mut self) {
        unsafe {
            SecKeychainDelete(self.keychain.as_concrete_TypeRef().cast());
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

const SCRATCH_PASSWORD: &str = "rfirma-scratch";

#[link(name = "Security", kind = "framework")]
extern "C" {
    fn SecKeychainDelete(keychain: *mut std::ffi::c_void) -> i32;
}

/// La situación del catálogo que corresponde a un error de Security.framework, CryptoTokenKit o PC/SC.
pub fn situation_of(domain: &str, code: i64) -> Situation {
    match (domain, code as i32) {
        (OS_STATUS_DOMAIN, -128) | (TOKEN_DOMAIN, -4) => Situation::PinEntryCancelled,
        (OS_STATUS_DOMAIN, -25293 | WRONG_CHV) | (TOKEN_DOMAIN, -5) => Situation::IncorrectPin,
        (OS_STATUS_DOMAIN, CHV_BLOCKED) => Situation::PinLocked,
        (OS_STATUS_DOMAIN, NO_SMARTCARD | REMOVED_CARD) | (TOKEN_DOMAIN, -7) => {
            Situation::TokenAbsent
        }
        (OS_STATUS_DOMAIN, ITEM_NOT_FOUND) | (TOKEN_DOMAIN, -6) => Situation::CertificateNotFound,
        _ => Situation::Unknown,
    }
}

const OS_STATUS_DOMAIN: &str = "NSOSStatusErrorDomain";
const TOKEN_DOMAIN: &str = "CryptoTokenKit";
const WRONG_CHV: i32 = 0x8010_006B_u32 as i32;
const CHV_BLOCKED: i32 = 0x8010_006C_u32 as i32;
const NO_SMARTCARD: i32 = 0x8010_000C_u32 as i32;
const REMOVED_CARD: i32 = 0x8010_0069_u32 as i32;

fn refused(error: &CFError) -> TokenError {
    let domain = error.domain().to_string();
    let situation = situation_of(&domain, error.code() as i64);
    let detail = if situation == Situation::PinEntryCancelled {
        format!(
            "has cancelado la petición de macOS ({domain} {})",
            error.code()
        )
    } else {
        format!(
            "macOS no ha firmado ({domain} {}): {}",
            error.code(),
            error.description()
        )
    };
    TokenError::new(situation, detail)
}

/// El algoritmo de Security.framework que firma un resumen ya calculado.
fn over_a_digest(algorithm: SignatureAlgorithm) -> Algorithm {
    match algorithm {
        SignatureAlgorithm::Sha1Rsa => Algorithm::RSASignatureDigestPKCS1v15SHA1,
        SignatureAlgorithm::Sha256Rsa => Algorithm::RSASignatureDigestPKCS1v15SHA256,
        SignatureAlgorithm::Sha384Rsa => Algorithm::RSASignatureDigestPKCS1v15SHA384,
        SignatureAlgorithm::Sha512Rsa => Algorithm::RSASignatureDigestPKCS1v15SHA512,
        SignatureAlgorithm::Sha256RsaPss => Algorithm::RSASignatureDigestPSSSHA256,
        SignatureAlgorithm::Sha384RsaPss => Algorithm::RSASignatureDigestPSSSHA384,
        SignatureAlgorithm::Sha512RsaPss => Algorithm::RSASignatureDigestPSSSHA512,
        SignatureAlgorithm::Sha1Ecdsa => Algorithm::ECDSASignatureDigestX962SHA1,
        SignatureAlgorithm::Sha256Ecdsa => Algorithm::ECDSASignatureDigestX962SHA256,
        SignatureAlgorithm::Sha384Ecdsa => Algorithm::ECDSASignatureDigestX962SHA384,
        SignatureAlgorithm::Sha512Ecdsa => Algorithm::ECDSASignatureDigestX962SHA512,
    }
}

/// La identidad del llavero cuyo certificado tiene la huella de `reference`.
fn identity_of(reference: &CertificateRef) -> Result<(TokenCertificate, SecIdentity), TokenError> {
    let not_there = || {
        TokenError::new(
            Situation::CertificateNotFound,
            format!("el llavero ya no tiene {}", reference.label()),
        )
    };
    let thumbprint = reference.cka_id().ok_or_else(not_there)?;
    let store = reference.store();
    let found = {
        let _no_prompt =
            SecKeychain::disable_user_interaction().map_err(|error| failure(&error))?;
        found(ItemClass::identity(), &scope_of(&store)?)?
    };
    found
        .into_iter()
        .find_map(|reference| match reference {
            Reference::Identity(identity) => {
                let der = identity.certificate().ok()?.to_der();
                let certificate = certificate_in(&store, &der, None);
                (certificate.reference().cka_id() == Some(thumbprint))
                    .then_some((certificate, identity))
            }
            _ => None,
        })
        .ok_or_else(not_there)
}

/// Los llaveros donde se busca: ninguno en concreto es la lista de búsqueda de la persona usuaria.
fn scope_of(store: &Store) -> Result<Option<Vec<SecKeychain>>, TokenError> {
    let named = store
        .path()
        .to_str()
        .and_then(|path| path.strip_prefix(MACOS_STORE_PREFIX))
        .unwrap_or(SEARCH_LIST);
    if named == SEARCH_LIST {
        return Ok(None);
    }
    SecKeychain::open(named)
        .map(|keychain| Some(vec![keychain]))
        .map_err(|error| failure(&error))
}

fn found(class: ItemClass, scope: &Option<Vec<SecKeychain>>) -> Result<Vec<Reference>, TokenError> {
    let mut search = ItemSearchOptions::new();
    search.class(class).load_refs(true).limit(Limit::All);
    if let Some(keychains) = scope {
        search.keychains(keychains);
    }
    match search.search() {
        Ok(results) => Ok(results
            .into_iter()
            .filter_map(|result| match result {
                SearchResult::Ref(reference) => Some(reference),
                _ => None,
            })
            .collect()),
        Err(error) if error.code() == ITEM_NOT_FOUND => Ok(Vec::new()),
        Err(error) => Err(failure(&error)),
    }
}

fn certificate_of_identity(store: &Store, identity: &SecIdentity) -> Option<TokenCertificate> {
    let certificate = identity.certificate().ok()?;
    let token = identity
        .private_key()
        .ok()
        .and_then(|key| token_holding(&key));
    Some(certificate_in(store, &certificate.to_der(), token))
}

fn certificate_in(store: &Store, der: &[u8], token: Option<String>) -> TokenCertificate {
    let thumbprint = hash(MessageDigest::sha1(), der)
        .map(|digest| digest.to_vec())
        .unwrap_or_default();
    let label = SecCertificate::from_der(der)
        .map(|certificate| certificate.subject_summary())
        .ok()
        .filter(|summary| !summary.is_empty())
        .unwrap_or_else(|| hex(&thumbprint));
    let reference = CertificateRef::new(store.clone(), TOKEN_LABEL, label, thumbprint);
    let reference = match token {
        Some(token) => reference.with_key_provider(token),
        None => reference,
    };
    TokenCertificate::new(reference, der.to_vec())
}

/// El token del sistema que guarda la clave, leído de sus atributos; una clave de software no tiene.
fn token_holding(key: &SecKey) -> Option<String> {
    let attributes = key.attributes();
    let value = attributes.find(unsafe { kSecAttrTokenID }.cast())?;
    let value = unsafe { CFType::wrap_under_get_rule(*value) };
    value.downcast::<CFString>().map(|token| token.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn failure(error: &security_framework::base::Error) -> TokenError {
    TokenError::new(
        Situation::Unknown,
        format!("el llavero de macOS ha fallado ({}): {error}", error.code()),
    )
}
