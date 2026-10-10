//! La frontera C de PKCS#11: valida punteros, delega en el estado del proceso y anota cada llamada.

use std::ffi::c_void;
use std::slice;
use std::sync::{Mutex, MutexGuard};

use cryptoki_sys::*;

use crate::calls;
use crate::card;
use crate::info;
use crate::module::{check_slot, Module, SLOT};
use crate::objects::Lookup;
use crate::signing::MECHANISMS;

static MODULE: Mutex<Option<Module>> = Mutex::new(None);

static FUNCTIONS: CK_FUNCTION_LIST = {
    // SAFETY: la lista es de punteros opcionales a función, y todo a cero es `None`.
    let mut list: CK_FUNCTION_LIST = unsafe { std::mem::zeroed() };
    list.version = CK_VERSION {
        major: 2,
        minor: 40,
    };
    list.C_Initialize = Some(initialize);
    list.C_Finalize = Some(finalize);
    list.C_GetInfo = Some(get_info);
    list.C_GetFunctionList = Some(C_GetFunctionList);
    list.C_GetSlotList = Some(get_slot_list);
    list.C_GetSlotInfo = Some(get_slot_info);
    list.C_GetTokenInfo = Some(get_token_info);
    list.C_GetMechanismList = Some(get_mechanism_list);
    list.C_GetMechanismInfo = Some(get_mechanism_info);
    list.C_OpenSession = Some(open_session);
    list.C_CloseSession = Some(close_session);
    list.C_CloseAllSessions = Some(close_all_sessions);
    list.C_GetSessionInfo = Some(get_session_info);
    list.C_Login = Some(login);
    list.C_Logout = Some(logout);
    list.C_GetAttributeValue = Some(get_attribute_value);
    list.C_FindObjectsInit = Some(find_objects_init);
    list.C_FindObjects = Some(find_objects);
    list.C_FindObjectsFinal = Some(find_objects_final);
    list.C_SignInit = Some(sign_init);
    list.C_Sign = Some(sign);
    list.C_SignUpdate = Some(sign_update);
    list.C_SignFinal = Some(sign_final);
    list
};

/// La única entrada exportada: `cryptoki` y p11-kit llegan al resto por esta lista.
///
/// # Safety
///
/// `list` debe ser nulo o apuntar a memoria escribible para un puntero.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn C_GetFunctionList(list: *mut *mut CK_FUNCTION_LIST) -> CK_RV {
    let rv = match list.as_mut() {
        None => CKR_ARGUMENTS_BAD,
        Some(list) => {
            *list = std::ptr::addr_of!(FUNCTIONS).cast_mut();
            CKR_OK
        }
    };
    record("C_GetFunctionList", "", rv)
}

fn state() -> MutexGuard<'static, Option<Module>> {
    MODULE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn record(function: &str, detail: &str, rv: CK_RV) -> CK_RV {
    card::record_call(card::card_dir(), &calls::line(function, detail, rv));
    rv
}

/// Corre `body` sobre el estado del proceso, si el módulo está inicializado, y anota la llamada.
fn entry(
    function: &str,
    detail: &str,
    body: impl FnOnce(&mut Module) -> Result<(), CK_RV>,
) -> CK_RV {
    run(function, detail, true, body)
}

/// Como `entry`, para lo que responde el lector aunque no tenga tarjeta.
fn reader_entry(
    function: &str,
    detail: &str,
    body: impl FnOnce(&mut Module) -> Result<(), CK_RV>,
) -> CK_RV {
    run(function, detail, false, body)
}

fn run(
    function: &str,
    detail: &str,
    needs_card: bool,
    body: impl FnOnce(&mut Module) -> Result<(), CK_RV>,
) -> CK_RV {
    let rv = match state().as_mut() {
        None => CKR_CRYPTOKI_NOT_INITIALIZED,
        Some(_) if needs_card && card::card_removed(card::card_dir()) => removed_card_rv(function),
        Some(module) => body(module).err().unwrap_or(CKR_OK),
    };
    record(function, detail, rv)
}

/// Lo que responde un lector sin tarjeta a una pregunta sobre ella, o sobre una sesión que tuvo.
fn removed_card_rv(function: &str) -> CK_RV {
    match function {
        "C_GetTokenInfo" | "C_GetMechanismList" | "C_GetMechanismInfo" | "C_OpenSession" => {
            CKR_TOKEN_NOT_PRESENT
        }
        _ => CKR_DEVICE_REMOVED,
    }
}

fn out<'a, T>(pointer: *mut T) -> Result<&'a mut T, CK_RV> {
    // SAFETY: quien llama a PKCS#11 da punteros nulos o válidos para escribir un `T`.
    unsafe { pointer.as_mut() }.ok_or(CKR_ARGUMENTS_BAD)
}

/// Escribe `values` en una lista de C con su longitud, o solo la longitud si la lista es nula.
fn write_list<T: Copy>(values: &[T], list: *mut T, count: *mut CK_ULONG) -> Result<(), CK_RV> {
    let count = out(count)?;
    let capacity = *count as usize;
    *count = values.len() as CK_ULONG;
    if list.is_null() {
        return Ok(());
    }
    if capacity < values.len() {
        return Err(CKR_BUFFER_TOO_SMALL);
    }
    // SAFETY: la lista tiene `capacity` elementos, y caben todos.
    unsafe { slice::from_raw_parts_mut(list, values.len()) }.copy_from_slice(values);
    Ok(())
}

unsafe extern "C" fn initialize(_args: *mut c_void) -> CK_RV {
    if card::hangs_on_initialize(card::card_dir()) {
        loop {
            std::thread::park();
        }
    }
    let mut state = state();
    let rv = if state.is_some() {
        CKR_CRYPTOKI_ALREADY_INITIALIZED
    } else {
        match Module::load(card::card_dir().to_path_buf()) {
            Ok(module) => {
                *state = Some(module);
                CKR_OK
            }
            Err(_) => CKR_GENERAL_ERROR,
        }
    };
    drop(state);
    record("C_Initialize", "", rv)
}

unsafe extern "C" fn finalize(reserved: *mut c_void) -> CK_RV {
    let rv = if !reserved.is_null() {
        CKR_ARGUMENTS_BAD
    } else if state().take().is_none() {
        CKR_CRYPTOKI_NOT_INITIALIZED
    } else {
        CKR_OK
    };
    record("C_Finalize", "", rv)
}

unsafe extern "C" fn get_info(target: *mut CK_INFO) -> CK_RV {
    reader_entry("C_GetInfo", "", |_| {
        *out(target)? = info::library();
        Ok(())
    })
}

unsafe extern "C" fn get_slot_list(
    token_present: CK_BBOOL,
    list: *mut CK_SLOT_ID,
    count: *mut CK_ULONG,
) -> CK_RV {
    reader_entry("C_GetSlotList", "", |_| {
        let slots: &[CK_SLOT_ID] = if token_present != CK_FALSE && card_removed() {
            &[]
        } else {
            &[SLOT]
        };
        write_list(slots, list, count)
    })
}

fn card_removed() -> bool {
    card::card_removed(card::card_dir())
}

unsafe extern "C" fn get_slot_info(slot: CK_SLOT_ID, target: *mut CK_SLOT_INFO) -> CK_RV {
    reader_entry("C_GetSlotInfo", "", |_| {
        check_slot(slot)?;
        *out(target)? = info::slot(!card_removed());
        Ok(())
    })
}

unsafe extern "C" fn get_token_info(slot: CK_SLOT_ID, target: *mut CK_TOKEN_INFO) -> CK_RV {
    entry("C_GetTokenInfo", "", |module| {
        check_slot(slot)?;
        *out(target)? = info::token(module.pin_signals(), module.session_count());
        Ok(())
    })
}

unsafe extern "C" fn get_mechanism_list(
    slot: CK_SLOT_ID,
    list: *mut CK_MECHANISM_TYPE,
    count: *mut CK_ULONG,
) -> CK_RV {
    entry("C_GetMechanismList", "", |_| {
        check_slot(slot)?;
        write_list(&MECHANISMS, list, count)
    })
}

unsafe extern "C" fn get_mechanism_info(
    slot: CK_SLOT_ID,
    mechanism: CK_MECHANISM_TYPE,
    target: *mut CK_MECHANISM_INFO,
) -> CK_RV {
    entry("C_GetMechanismInfo", "", |_| {
        check_slot(slot)?;
        if !MECHANISMS.contains(&mechanism) {
            return Err(CKR_MECHANISM_INVALID);
        }
        *out(target)? = info::mechanism();
        Ok(())
    })
}

unsafe extern "C" fn open_session(
    slot: CK_SLOT_ID,
    flags: CK_FLAGS,
    _application: *mut c_void,
    _notify: CK_NOTIFY,
    session: *mut CK_SESSION_HANDLE,
) -> CK_RV {
    entry("C_OpenSession", "", |module| {
        let target = out(session)?;
        *target = module.open_session(slot, flags)?;
        Ok(())
    })
}

unsafe extern "C" fn close_session(session: CK_SESSION_HANDLE) -> CK_RV {
    entry("C_CloseSession", "", |module| module.close_session(session))
}

unsafe extern "C" fn close_all_sessions(slot: CK_SLOT_ID) -> CK_RV {
    entry("C_CloseAllSessions", "", |module| {
        module.close_all_sessions(slot)
    })
}

unsafe extern "C" fn get_session_info(
    session: CK_SESSION_HANDLE,
    target: *mut CK_SESSION_INFO,
) -> CK_RV {
    entry("C_GetSessionInfo", "", |module| {
        module.check_session(session)?;
        *out(target)? = CK_SESSION_INFO {
            slotID: SLOT,
            state: if module.is_logged_in() {
                CKS_RO_USER_FUNCTIONS
            } else {
                CKS_RO_PUBLIC_SESSION
            },
            flags: CKF_SERIAL_SESSION,
            ulDeviceError: 0,
        };
        Ok(())
    })
}

unsafe extern "C" fn login(
    session: CK_SESSION_HANDLE,
    user: CK_USER_TYPE,
    pin: *mut CK_UTF8CHAR,
    pin_len: CK_ULONG,
) -> CK_RV {
    let pin = (!pin.is_null()).then(|| slice::from_raw_parts(pin, pin_len as usize));
    let detail = format!("{} {}", calls::user_name(user), calls::pin_text(pin));
    entry("C_Login", &detail, |module| {
        module.login(session, user, pin)
    })
}

unsafe extern "C" fn logout(session: CK_SESSION_HANDLE) -> CK_RV {
    entry("C_Logout", "", |module| module.logout(session))
}

unsafe extern "C" fn get_attribute_value(
    session: CK_SESSION_HANDLE,
    object: CK_OBJECT_HANDLE,
    template: *mut CK_ATTRIBUTE,
    count: CK_ULONG,
) -> CK_RV {
    entry("C_GetAttributeValue", "", |module| {
        if template.is_null() {
            return Err(CKR_ARGUMENTS_BAD);
        }
        let template = slice::from_raw_parts_mut(template, count as usize);
        let mut verdict = Ok(());
        for attribute in template {
            let lookup = module.attribute(session, object, attribute.type_)?;
            if let Err(rv) = fill(attribute, lookup) {
                verdict = Err(rv);
            }
        }
        verdict
    })
}

/// Rellena un atributo pedido; un fallo en uno no impide rellenar los demás.
fn fill(attribute: &mut CK_ATTRIBUTE, lookup: Lookup<'_>) -> Result<(), CK_RV> {
    let bytes = match lookup {
        Lookup::Missing => return unavailable(attribute, CKR_ATTRIBUTE_TYPE_INVALID),
        Lookup::Sensitive => return unavailable(attribute, CKR_ATTRIBUTE_SENSITIVE),
        Lookup::Bytes(bytes) => bytes,
    };
    if attribute.pValue.is_null() {
        attribute.ulValueLen = bytes.len() as CK_ULONG;
        return Ok(());
    }
    if (attribute.ulValueLen as usize) < bytes.len() {
        return unavailable(attribute, CKR_BUFFER_TOO_SMALL);
    }
    // SAFETY: quien llama reserva `ulValueLen` bytes en `pValue`, y caben.
    unsafe { slice::from_raw_parts_mut(attribute.pValue.cast::<u8>(), bytes.len()) }
        .copy_from_slice(bytes);
    attribute.ulValueLen = bytes.len() as CK_ULONG;
    Ok(())
}

fn unavailable(attribute: &mut CK_ATTRIBUTE, rv: CK_RV) -> Result<(), CK_RV> {
    attribute.ulValueLen = CK_UNAVAILABLE_INFORMATION;
    Err(rv)
}

unsafe extern "C" fn find_objects_init(
    session: CK_SESSION_HANDLE,
    template: *mut CK_ATTRIBUTE,
    count: CK_ULONG,
) -> CK_RV {
    entry("C_FindObjectsInit", "", |module| {
        let wanted = if count == 0 {
            Vec::new()
        } else if template.is_null() {
            return Err(CKR_ARGUMENTS_BAD);
        } else {
            slice::from_raw_parts(template, count as usize)
                .iter()
                .map(|attribute| (attribute.type_, value_of(attribute)))
                .collect()
        };
        module.find_init(session, &wanted)
    })
}

fn value_of(attribute: &CK_ATTRIBUTE) -> Vec<u8> {
    if attribute.pValue.is_null() {
        return Vec::new();
    }
    // SAFETY: un atributo de plantilla lleva `ulValueLen` bytes en `pValue`.
    unsafe { slice::from_raw_parts(attribute.pValue.cast::<u8>(), attribute.ulValueLen as usize) }
        .to_vec()
}

unsafe extern "C" fn find_objects(
    session: CK_SESSION_HANDLE,
    found: *mut CK_OBJECT_HANDLE,
    max: CK_ULONG,
    count: *mut CK_ULONG,
) -> CK_RV {
    entry("C_FindObjects", "", |module| {
        let count = out(count)?;
        if found.is_null() {
            return Err(CKR_ARGUMENTS_BAD);
        }
        let handles = module.find(session, max as usize)?;
        slice::from_raw_parts_mut(found, handles.len()).copy_from_slice(&handles);
        *count = handles.len() as CK_ULONG;
        Ok(())
    })
}

unsafe extern "C" fn find_objects_final(session: CK_SESSION_HANDLE) -> CK_RV {
    entry("C_FindObjectsFinal", "", |module| {
        module.find_final(session)
    })
}

unsafe extern "C" fn sign_init(
    session: CK_SESSION_HANDLE,
    mechanism: *mut CK_MECHANISM,
    key: CK_OBJECT_HANDLE,
) -> CK_RV {
    let mechanism = mechanism.as_ref().map(|chosen| chosen.mechanism);
    let detail = format!(
        "{:#x} key={key}",
        mechanism.unwrap_or(CK_UNAVAILABLE_INFORMATION)
    );
    entry("C_SignInit", &detail, |module| {
        module.sign_init(session, mechanism.ok_or(CKR_ARGUMENTS_BAD)?, key)
    })
}

unsafe extern "C" fn sign(
    session: CK_SESSION_HANDLE,
    data: *mut CK_BYTE,
    data_len: CK_ULONG,
    signature: *mut CK_BYTE,
    signature_len: *mut CK_ULONG,
) -> CK_RV {
    entry("C_Sign", "", |module| {
        let capacity = out(signature_len)?;
        if signature.is_null() {
            *capacity = module.signature_len(session)? as CK_ULONG;
            return Ok(());
        }
        if data.is_null() {
            module.sign_done(session);
            return Err(CKR_ARGUMENTS_BAD);
        }
        let produced = module.sign(session, slice::from_raw_parts(data, data_len as usize))?;
        deliver(module, session, &produced, signature, capacity)
    })
}

unsafe extern "C" fn sign_update(
    session: CK_SESSION_HANDLE,
    part: *mut CK_BYTE,
    part_len: CK_ULONG,
) -> CK_RV {
    entry("C_SignUpdate", "", |module| {
        if part.is_null() {
            module.sign_done(session);
            return Err(CKR_ARGUMENTS_BAD);
        }
        module.sign_update(session, slice::from_raw_parts(part, part_len as usize))
    })
}

unsafe extern "C" fn sign_final(
    session: CK_SESSION_HANDLE,
    signature: *mut CK_BYTE,
    signature_len: *mut CK_ULONG,
) -> CK_RV {
    entry("C_SignFinal", "", |module| {
        let capacity = out(signature_len)?;
        if signature.is_null() {
            *capacity = module.signature_len(session)? as CK_ULONG;
            return Ok(());
        }
        let produced = module.sign_final(session)?;
        deliver(module, session, &produced, signature, capacity)
    })
}

/// Entrega la firma si cabe, y cierra la operación; si no cabe, la deja abierta para repetir con más sitio.
unsafe fn deliver(
    module: &mut Module,
    session: CK_SESSION_HANDLE,
    produced: &[u8],
    signature: *mut CK_BYTE,
    capacity: &mut CK_ULONG,
) -> Result<(), CK_RV> {
    if (*capacity as usize) < produced.len() {
        *capacity = produced.len() as CK_ULONG;
        return Err(CKR_BUFFER_TOO_SMALL);
    }
    slice::from_raw_parts_mut(signature, produced.len()).copy_from_slice(produced);
    *capacity = produced.len() as CK_ULONG;
    module.sign_done(session);
    Ok(())
}
