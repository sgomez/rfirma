//! `CurrentUser\MY` leído con CryptoAPI y el resumen firmado con `NCryptSignHash`: la clave no sale de su proveedor (ADR-0001).

use std::ffi::c_void;
use std::ptr;

use windows_sys::core::{w, BOOL, PCWSTR};
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::Security::Cryptography::{
    CertCloseStore, CertEnumCertificatesInStore, CertFindCertificateInStore,
    CertFreeCertificateContext, CertGetCertificateContextProperty, CertOpenStore,
    CryptAcquireCertificatePrivateKey, NCryptFreeObject, NCryptSetProperty, NCryptSignHash,
    BCRYPT_PKCS1_PADDING_INFO, BCRYPT_PSS_PADDING_INFO, BCRYPT_SHA1_ALGORITHM,
    BCRYPT_SHA256_ALGORITHM, BCRYPT_SHA384_ALGORITHM, BCRYPT_SHA512_ALGORITHM, CERT_CONTEXT,
    CERT_FIND_SHA1_HASH, CERT_FRIENDLY_NAME_PROP_ID, CERT_HASH_PROP_ID, CERT_KEY_PROV_INFO_PROP_ID,
    CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W, CERT_STORE_READONLY_FLAG,
    CERT_SYSTEM_STORE_CURRENT_USER, CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG,
    CRYPT_ACQUIRE_WINDOW_HANDLE_FLAG, CRYPT_INTEGER_BLOB, CRYPT_KEY_PROV_INFO, HCERTSTORE,
    NCRYPT_KEY_HANDLE, NCRYPT_PAD_PKCS1_FLAG, NCRYPT_PAD_PSS_FLAG, NCRYPT_WINDOW_HANDLE_PROPERTY,
    PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, IsWindowVisible,
    GA_ROOTOWNER,
};
use x509_cert::der::Decode;
use x509_cert::Certificate;

use super::user_store;
use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::ecdsa;
use crate::identity::domain::error::{Situation, TokenError};

/// La etiqueta de «token» de los certificados de `CurrentUser\MY`.
pub const TOKEN_LABEL: &str = "CurrentUser\\MY";

const ENCODING: u32 = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;

/// Los certificados personales que tienen una clave privada asociada.
pub fn signable_certificates() -> Result<Vec<TokenCertificate>, TokenError> {
    Ok(entries_of(w!("MY"))?
        .into_iter()
        .filter(|entry| entry.has_private_key)
        .map(Entry::into_certificate)
        .collect())
}

/// Los personales y las autoridades del usuario: con ellos se completa la cadena.
pub fn every_certificate() -> Result<Vec<TokenCertificate>, TokenError> {
    let mut found = entries_of(w!("MY"))?;
    for authorities in [w!("CA"), w!("Root")] {
        found.extend(entries_of(authorities).unwrap_or_default());
    }
    Ok(found.into_iter().map(Entry::into_certificate).collect())
}

/// Comprueba que la clave del certificado es de la clase que exige el algoritmo.
pub fn offers(reference: &CertificateRef, algorithm: SignatureAlgorithm) -> Result<(), TokenError> {
    let found = FoundCertificate::of(reference)?;
    let certificate = TokenCertificate::new(reference.clone(), found.der());
    if certificate.key_kind() == Some(algorithm.key_kind()) {
        Ok(())
    } else {
        Err(TokenError::new(
            Situation::MechanismNotOffered,
            format!(
                "la clave de {} no sirve para {}",
                reference.label(),
                algorithm.name()
            ),
        ))
    }
}

/// Resume `data` y firma el resumen con la clave del certificado; Windows pide el PIN si hace falta.
pub fn sign(
    reference: &CertificateRef,
    algorithm: SignatureAlgorithm,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    let found = FoundCertificate::of(reference)?;
    let key = PrivateKey::of(&found)?;
    let digest = ecdsa::digest(algorithm, data)?;
    let signature = key.sign(algorithm, &digest)?;
    match algorithm.key_kind() {
        KeyKind::Rsa => Ok(signature),
        KeyKind::Ec => ecdsa::der_encoded(&signature),
    }
}

/// La situación del catálogo que corresponde a un código de error de CryptoAPI, CNG o la tarjeta.
pub fn situation_of(code: u32) -> Situation {
    match code {
        0x8010_006B | 0x8009_0033 => Situation::IncorrectPin,
        0x8010_006C => Situation::PinLocked,
        0x8010_006E | 0x8010_0002 | 0x8009_0036 | 0x8007_04C7 => Situation::PinEntryCancelled,
        0x8010_000C | 0x8010_0069 | 0x8010_002E | 0x8010_0017 => Situation::TokenAbsent,
        0x8009_0016 | 0x8009_000D | 0x8009_2004 => Situation::CertificateNotFound,
        0x8009_0029 | 0x8009_0008 => Situation::MechanismNotOffered,
        _ => Situation::Unknown,
    }
}

fn failure(code: u32, doing: &str) -> TokenError {
    let situation = situation_of(code);
    let detail = if situation == Situation::PinEntryCancelled {
        format!("has cancelado la petición del PIN de Windows (0x{code:08X})")
    } else {
        format!("Windows ha devuelto 0x{code:08X} al {doing}")
    };
    TokenError::new(situation, detail)
}

fn last_failure(doing: &str) -> TokenError {
    let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    failure(code as u32, doing)
}

struct Entry {
    der: Vec<u8>,
    thumbprint: Vec<u8>,
    friendly_name: Option<String>,
    has_private_key: bool,
    key_provider: Option<String>,
}

impl Entry {
    unsafe fn of(context: *const CERT_CONTEXT) -> Self {
        let encoded = &*context;
        let key_info = property(context, CERT_KEY_PROV_INFO_PROP_ID);
        Self {
            der: std::slice::from_raw_parts(encoded.pbCertEncoded, encoded.cbCertEncoded as usize)
                .to_vec(),
            thumbprint: property(context, CERT_HASH_PROP_ID).unwrap_or_default(),
            friendly_name: property(context, CERT_FRIENDLY_NAME_PROP_ID)
                .map(|bytes| utf16_text(&bytes))
                .filter(|name| !name.is_empty()),
            has_private_key: key_info.is_some(),
            key_provider: key_info.as_deref().and_then(|info| provider_named_in(info)),
        }
    }

    fn into_certificate(self) -> TokenCertificate {
        let label = self
            .friendly_name
            .or_else(|| subject_of(&self.der))
            .unwrap_or_else(|| hex(&self.thumbprint));
        let reference = CertificateRef::new(user_store(), TOKEN_LABEL, label, self.thumbprint);
        let reference = match self.key_provider {
            Some(provider) => reference.with_key_provider(provider),
            None => reference,
        };
        TokenCertificate::new(reference, self.der)
    }
}

/// El proveedor que nombra `CERT_KEY_PROV_INFO_PROP_ID`: se lee de la propiedad, sin abrir la clave.
unsafe fn provider_named_in(key_info: &[u8]) -> Option<String> {
    if key_info.len() < std::mem::size_of::<CRYPT_KEY_PROV_INFO>() {
        return None;
    }
    let info = ptr::read_unaligned(key_info.as_ptr().cast::<CRYPT_KEY_PROV_INFO>());
    let name = info.pwszProvName.cast_const();
    if name.is_null() {
        return None;
    }
    let units: Vec<u16> = (0..)
        .map(|offset| ptr::read_unaligned(name.add(offset)))
        .take_while(|unit| *unit != 0)
        .collect();
    Some(String::from_utf16_lossy(&units)).filter(|provider| !provider.is_empty())
}

fn subject_of(der: &[u8]) -> Option<String> {
    Certificate::from_der(der)
        .ok()
        .map(|certificate| certificate.tbs_certificate().subject().to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn utf16_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .take_while(|unit| *unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

unsafe fn property(context: *const CERT_CONTEXT, id: u32) -> Option<Vec<u8>> {
    let mut size = 0u32;
    if CertGetCertificateContextProperty(context, id, ptr::null_mut(), &mut size) == 0 {
        return None;
    }
    let mut buffer = vec![0u8; size as usize];
    if CertGetCertificateContextProperty(context, id, buffer.as_mut_ptr().cast(), &mut size) == 0 {
        return None;
    }
    buffer.truncate(size as usize);
    Some(buffer)
}

struct SystemStore(HCERTSTORE);

impl SystemStore {
    fn open(name: PCWSTR) -> Result<Self, TokenError> {
        let handle = unsafe {
            CertOpenStore(
                CERT_STORE_PROV_SYSTEM_W,
                0,
                0,
                CERT_SYSTEM_STORE_CURRENT_USER
                    | CERT_STORE_READONLY_FLAG
                    | CERT_STORE_OPEN_EXISTING_FLAG,
                name.cast::<c_void>(),
            )
        };
        if handle.is_null() {
            Err(last_failure("abrir el almacen de certificados del usuario"))
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for SystemStore {
    fn drop(&mut self) {
        unsafe { CertCloseStore(self.0, 0) };
    }
}

fn entries_of(name: PCWSTR) -> Result<Vec<Entry>, TokenError> {
    let store = SystemStore::open(name)?;
    let mut found = Vec::new();
    let mut context: *const CERT_CONTEXT = ptr::null();
    loop {
        context = unsafe { CertEnumCertificatesInStore(store.0, context) };
        if context.is_null() {
            return Ok(found);
        }
        found.push(unsafe { Entry::of(context) });
    }
}

struct FoundCertificate {
    context: *const CERT_CONTEXT,
    _store: SystemStore,
}

impl FoundCertificate {
    fn of(reference: &CertificateRef) -> Result<Self, TokenError> {
        let not_there = || {
            TokenError::new(
                Situation::CertificateNotFound,
                format!("el almacen del usuario ya no tiene {}", reference.label()),
            )
        };
        let thumbprint = reference.cka_id().ok_or_else(not_there)?;
        let store = SystemStore::open(w!("MY"))?;
        let blob = CRYPT_INTEGER_BLOB {
            cbData: thumbprint.len() as u32,
            pbData: thumbprint.as_ptr().cast_mut(),
        };
        let context = unsafe {
            CertFindCertificateInStore(
                store.0,
                ENCODING,
                0,
                CERT_FIND_SHA1_HASH,
                (&raw const blob).cast::<c_void>(),
                ptr::null(),
            )
        };
        if context.is_null() {
            return Err(not_there());
        }
        Ok(Self {
            context,
            _store: store,
        })
    }

    fn der(&self) -> Vec<u8> {
        unsafe {
            let encoded = &*self.context;
            std::slice::from_raw_parts(encoded.pbCertEncoded, encoded.cbCertEncoded as usize)
                .to_vec()
        }
    }
}

impl Drop for FoundCertificate {
    fn drop(&mut self) {
        unsafe { CertFreeCertificateContext(self.context) };
    }
}

struct PrivateKey {
    handle: NCRYPT_KEY_HANDLE,
    owned: bool,
}

impl PrivateKey {
    fn of(certificate: &FoundCertificate) -> Result<Self, TokenError> {
        let mut handle = 0;
        let mut spec = 0;
        let mut owned = 0;
        let owner = owner_window();
        let (flags, parameter): (u32, *const c_void) = match &owner {
            Some(window) => (
                CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG | CRYPT_ACQUIRE_WINDOW_HANDLE_FLAG,
                ptr::from_ref(window).cast(),
            ),
            None => (CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG, ptr::null()),
        };
        let acquired = unsafe {
            CryptAcquireCertificatePrivateKey(
                certificate.context,
                flags,
                parameter,
                &mut handle,
                &mut spec,
                &mut owned,
            )
        };
        if acquired == 0 {
            return Err(last_failure("abrir la clave privada del certificado"));
        }
        if let Some(window) = &owner {
            unsafe {
                NCryptSetProperty(
                    handle,
                    NCRYPT_WINDOW_HANDLE_PROPERTY,
                    ptr::from_ref(window).cast(),
                    std::mem::size_of::<HWND>() as u32,
                    0,
                )
            };
        }
        Ok(Self {
            handle,
            owned: owned != 0,
        })
    }

    fn sign(&self, algorithm: SignatureAlgorithm, digest: &[u8]) -> Result<Vec<u8>, TokenError> {
        let hash = hash_name(algorithm);
        let pkcs1 = BCRYPT_PKCS1_PADDING_INFO { pszAlgId: hash };
        let pss = BCRYPT_PSS_PADDING_INFO {
            pszAlgId: hash,
            cbSalt: digest.len() as u32,
        };
        let (padding, flags): (*const c_void, u32) = match padding_of(algorithm) {
            Padding::Pkcs1 => ((&raw const pkcs1).cast(), NCRYPT_PAD_PKCS1_FLAG),
            Padding::Pss => ((&raw const pss).cast(), NCRYPT_PAD_PSS_FLAG),
            Padding::None => (ptr::null(), 0),
        };
        let mut size = 0u32;
        let status = unsafe {
            NCryptSignHash(
                self.handle,
                padding,
                digest.as_ptr(),
                digest.len() as u32,
                ptr::null_mut(),
                0,
                &mut size,
                flags,
            )
        };
        succeeded(status, "medir la firma")?;
        let mut signature = vec![0u8; size as usize];
        let status = unsafe {
            NCryptSignHash(
                self.handle,
                padding,
                digest.as_ptr(),
                digest.len() as u32,
                signature.as_mut_ptr(),
                size,
                &mut size,
                flags,
            )
        };
        succeeded(status, "firmar el resumen")?;
        signature.truncate(size as usize);
        Ok(signature)
    }
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        if self.owned {
            unsafe { NCryptFreeObject(self.handle) };
        }
    }
}

/// La ventana de rFirma sobre la que Windows hace modal su petición del PIN.
fn owner_window() -> Option<HWND> {
    let foreground = unsafe { GetForegroundWindow() };
    if !foreground.is_null() && is_ours(foreground) {
        return Some(unsafe { GetAncestor(foreground, GA_ROOTOWNER) });
    }
    let mut found: HWND = ptr::null_mut();
    unsafe {
        EnumWindows(
            Some(first_visible_of_ours),
            ptr::from_mut(&mut found) as LPARAM,
        )
    };
    (!found.is_null()).then_some(found)
}

fn is_ours(window: HWND) -> bool {
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(window, &mut process) };
    process == std::process::id()
}

unsafe extern "system" fn first_visible_of_ours(window: HWND, found: LPARAM) -> BOOL {
    if unsafe { IsWindowVisible(window) } != 0 && is_ours(window) {
        unsafe { *(found as *mut HWND) = window };
        return 0;
    }
    1
}

fn succeeded(status: i32, doing: &str) -> Result<(), TokenError> {
    if status == 0 {
        Ok(())
    } else {
        Err(failure(status as u32, doing))
    }
}

/// El relleno con el que CNG cumple el algoritmo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Padding {
    /// PKCS#1 v1.5.
    Pkcs1,
    /// PSS con MGF1 y sal del tamaño del resumen.
    Pss,
    /// Ninguno: ECDSA.
    None,
}

/// El relleno que le toca al algoritmo.
pub fn padding_of(algorithm: SignatureAlgorithm) -> Padding {
    match algorithm {
        SignatureAlgorithm::Sha256RsaPss
        | SignatureAlgorithm::Sha384RsaPss
        | SignatureAlgorithm::Sha512RsaPss => Padding::Pss,
        _ if algorithm.key_kind() == KeyKind::Ec => Padding::None,
        _ => Padding::Pkcs1,
    }
}

pub fn hash_name(algorithm: SignatureAlgorithm) -> PCWSTR {
    match algorithm {
        SignatureAlgorithm::Sha1Rsa | SignatureAlgorithm::Sha1Ecdsa => BCRYPT_SHA1_ALGORITHM,
        SignatureAlgorithm::Sha256Rsa
        | SignatureAlgorithm::Sha256RsaPss
        | SignatureAlgorithm::Sha256Ecdsa => BCRYPT_SHA256_ALGORITHM,
        SignatureAlgorithm::Sha384Rsa
        | SignatureAlgorithm::Sha384RsaPss
        | SignatureAlgorithm::Sha384Ecdsa => BCRYPT_SHA384_ALGORITHM,
        SignatureAlgorithm::Sha512Rsa
        | SignatureAlgorithm::Sha512RsaPss
        | SignatureAlgorithm::Sha512Ecdsa => BCRYPT_SHA512_ALGORITHM,
    }
}
