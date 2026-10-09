//! Los certificados que enseñan ahora las tarjetas del lector, leídos de sus claves por el KSP de tarjeta sin abrirlas ni pedir el PIN (ADR-0048).

use std::ffi::c_void;
use std::ptr;

use windows_sys::Win32::Security::Cryptography::{
    NCryptEnumKeys, NCryptFreeBuffer, NCryptFreeObject, NCryptGetProperty, NCryptKeyName,
    NCryptOpenKey, NCryptOpenStorageProvider, MS_SMART_CARD_KEY_STORAGE_PROVIDER,
    NCRYPT_CERTIFICATE_PROPERTY, NCRYPT_KEY_HANDLE, NCRYPT_PROV_HANDLE, NCRYPT_SILENT_FLAG,
};

/// Los certificados, en DER, de las claves que el KSP de tarjeta enseña ahora; sin tarjeta o sin servicio, ninguno.
pub fn certificates_on_the_cards() -> Vec<Vec<u8>> {
    let Some(provider) = Provider::open() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut state: *mut c_void = ptr::null_mut();
    loop {
        let mut name: *mut NCryptKeyName = ptr::null_mut();
        let status = unsafe {
            NCryptEnumKeys(
                provider.0,
                ptr::null(),
                &mut name,
                &mut state,
                NCRYPT_SILENT_FLAG,
            )
        };
        if status != 0 || name.is_null() {
            break;
        }
        if let Some(der) = provider.certificate_of(unsafe { &*name }) {
            found.push(der);
        }
        unsafe { NCryptFreeBuffer(name.cast()) };
    }
    if !state.is_null() {
        unsafe { NCryptFreeBuffer(state) };
    }
    found
}

struct Provider(NCRYPT_PROV_HANDLE);

impl Provider {
    fn open() -> Option<Self> {
        let mut handle = 0;
        let status = unsafe {
            NCryptOpenStorageProvider(&mut handle, MS_SMART_CARD_KEY_STORAGE_PROVIDER, 0)
        };
        (status == 0).then_some(Self(handle))
    }

    fn certificate_of(&self, key: &NCryptKeyName) -> Option<Vec<u8>> {
        let mut handle: NCRYPT_KEY_HANDLE = 0;
        let status = unsafe {
            NCryptOpenKey(
                self.0,
                &mut handle,
                key.pszName,
                key.dwLegacyKeySpec,
                NCRYPT_SILENT_FLAG,
            )
        };
        if status != 0 {
            return None;
        }
        let certificate = certificate_property(handle);
        unsafe { NCryptFreeObject(handle) };
        certificate
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        unsafe { NCryptFreeObject(self.0) };
    }
}

fn certificate_property(key: NCRYPT_KEY_HANDLE) -> Option<Vec<u8>> {
    let mut size = 0u32;
    let status = unsafe {
        NCryptGetProperty(
            key,
            NCRYPT_CERTIFICATE_PROPERTY,
            ptr::null_mut(),
            0,
            &mut size,
            NCRYPT_SILENT_FLAG,
        )
    };
    if status != 0 || size == 0 {
        return None;
    }
    let mut der = vec![0u8; size as usize];
    let status = unsafe {
        NCryptGetProperty(
            key,
            NCRYPT_CERTIFICATE_PROPERTY,
            der.as_mut_ptr(),
            size,
            &mut size,
            NCRYPT_SILENT_FLAG,
        )
    };
    if status != 0 {
        return None;
    }
    der.truncate(size as usize);
    Some(der)
}
