//! Borrado de un certificado y su clave del Almacén de rFirma, por su `CKA_ID` (ADR-0034).

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::Path;

use super::nss::{
    failed, module_spec, nss_library, store_pin, symbol, CertList, CertListNode, PrCList, SecItem,
    PR_TRUE, SEC_SUCCESS,
};
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;

type LowLevelKeyId = extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut SecItem;
type FreeItem = extern "C" fn(*mut SecItem, c_int);

/// Borra del Almacén de rFirma en `directory` el certificado con el `CKA_ID` de `reference`, y su clave.
pub fn remove_certificate(
    directory: &Path,
    reference: &CertificateRef,
    pin: &ProtectedSecret,
) -> Result<(), TokenError> {
    let nss = nss_library()
        .map_err(|err| TokenError::new(Situation::ModuleNotFound, err.detail().to_owned()))?;

    type NoDbInit = extern "C" fn(*const c_char) -> c_int;
    type Shutdown = extern "C" fn() -> c_int;
    type OpenUserDb = extern "C" fn(*const c_char) -> *mut c_void;
    type CloseUserDb = extern "C" fn(*mut c_void) -> c_int;
    type NeedUserInit = extern "C" fn(*mut c_void) -> c_int;
    type CheckUserPassword = extern "C" fn(*mut c_void, *const c_char) -> c_int;
    type FreeSlot = extern "C" fn(*mut c_void);
    type ListCertsInSlot = extern "C" fn(*mut c_void) -> *mut CertList;
    type DeleteTokenCertAndKey = extern "C" fn(*mut c_void, *mut c_void) -> c_int;
    type DestroyCertList = extern "C" fn(*mut CertList);

    let nss_no_db_init: NoDbInit = symbol(nss, b"NSS_NoDB_Init\0")?;
    let nss_shutdown: Shutdown = symbol(nss, b"NSS_Shutdown\0")?;
    let open_user_db: OpenUserDb = symbol(nss, b"SECMOD_OpenUserDB\0")?;
    let close_user_db: CloseUserDb = symbol(nss, b"SECMOD_CloseUserDB\0")?;
    let need_user_init: NeedUserInit = symbol(nss, b"PK11_NeedUserInit\0")?;
    let check_user_password: CheckUserPassword = symbol(nss, b"PK11_CheckUserPassword\0")?;
    let free_slot: FreeSlot = symbol(nss, b"PK11_FreeSlot\0")?;
    let list_certs_in_slot: ListCertsInSlot = symbol(nss, b"PK11_ListCertsInSlot\0")?;
    let low_level_key_id: LowLevelKeyId = symbol(nss, b"PK11_GetLowLevelKeyIDForCert\0")?;
    let free_item: FreeItem = symbol(nss, b"SECITEM_FreeItem\0")?;
    let delete_cert_and_key: DeleteTokenCertAndKey = symbol(nss, b"PK11_DeleteTokenCertAndKey\0")?;
    let destroy_cert_list: DestroyCertList = symbol(nss, b"CERT_DestroyCertList\0")?;

    let spec = CString::new(module_spec(directory)).map_err(|_| {
        TokenError::new(
            Situation::ModuleNotFound,
            "la ruta del almacen lleva un cero dentro",
        )
    })?;
    let pin = store_pin(pin)?;
    let wanted = reference
        .cka_id()
        .ok_or_else(certificate_without_a_cka_id)?;

    if nss_no_db_init(std::ptr::null()) != SEC_SUCCESS {
        return Err(failed("NSS_NoDB_Init"));
    }

    let outcome = (|| {
        let slot = open_user_db(spec.as_ptr());
        if slot.is_null() {
            return Err(failed("SECMOD_OpenUserDB"));
        }

        let removed = (|| {
            if need_user_init(slot) == PR_TRUE {
                return Err(not_installed());
            }
            if check_user_password(slot, pin.as_ptr()) != SEC_SUCCESS {
                return Err(failed("PK11_CheckUserPassword"));
            }

            let list = list_certs_in_slot(slot);
            let found = certificate_with_id(list, slot, low_level_key_id, free_item, wanted);
            let result = match found {
                Some(certificate) => {
                    if delete_cert_and_key(certificate, std::ptr::null_mut()) == SEC_SUCCESS {
                        Ok(())
                    } else {
                        Err(failed("PK11_DeleteTokenCertAndKey"))
                    }
                }
                None => Err(not_installed()),
            };
            if !list.is_null() {
                destroy_cert_list(list);
            }
            result
        })();

        close_user_db(slot);
        free_slot(slot);
        removed
    })();

    nss_shutdown();
    outcome
}

/// El almacén no tiene ningún certificado con ese `CKA_ID`: no hay nada que borrar.
fn not_installed() -> TokenError {
    TokenError::new(
        Situation::CertificateNotFound,
        "el Almacen de rFirma no tiene ese certificado",
    )
}

/// Sin `CKA_ID` no hay forma de distinguir este certificado de otro con el mismo nombre.
fn certificate_without_a_cka_id() -> TokenError {
    TokenError::new(
        Situation::CertificateNotFound,
        "el certificado no lleva CKA_ID con el que reencontrarlo",
    )
}

/// El certificado de la lista cuyo `CKA_ID` en `slot` coincide con `wanted`, si lo hay.
fn certificate_with_id(
    list: *mut CertList,
    slot: *mut c_void,
    low_level_key_id: LowLevelKeyId,
    free_item: FreeItem,
    wanted: &[u8],
) -> Option<*mut c_void> {
    if list.is_null() {
        return None;
    }

    // SAFETY: NSS devuelve una lista circular viva hasta que se destruye.
    unsafe {
        let head: *mut PrCList = &raw mut (*list).links;
        let mut link = (*head).next;
        while !link.is_null() && !std::ptr::eq(link, head) {
            let certificate = (*link.cast::<CertListNode>()).certificate;
            if !certificate.is_null() {
                let id = low_level_key_id(slot, certificate, std::ptr::null_mut());
                if !id.is_null() {
                    let bytes = std::slice::from_raw_parts((*id).data, (*id).len as usize);
                    let matches = bytes == wanted;
                    free_item(id, PR_TRUE);
                    if matches {
                        return Some(certificate);
                    }
                }
            }
            link = (*link).next;
        }
    }

    None
}
