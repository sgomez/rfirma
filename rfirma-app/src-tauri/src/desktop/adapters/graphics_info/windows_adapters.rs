//! Los adaptadores de pantalla que Windows tiene dados de alta en la clase `Display` del registro.

use std::ptr;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
    KEY_WOW64_64KEY, RRF_RT_REG_SZ,
};

use super::DisplayAdapter;

const DISPLAY_CLASS_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
const MAX_KEY_NAME: usize = 256;

/// Los adaptadores con `DriverDesc`, en el orden de sus subclaves; vacío si la clase no se lee.
pub fn display_adapters() -> Vec<DisplayAdapter> {
    let path = wide(DISPLAY_CLASS_KEY);
    let mut class: HKEY = ptr::null_mut();
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            0,
            KEY_READ | KEY_WOW64_64KEY,
            &mut class,
        )
    };
    if opened != ERROR_SUCCESS {
        return Vec::new();
    }
    let adapters = instance_keys(class)
        .iter()
        .filter_map(|instance| adapter_at(class, instance))
        .collect();
    unsafe { RegCloseKey(class) };
    adapters
}

fn instance_keys(class: HKEY) -> Vec<String> {
    let mut names = Vec::new();
    for index in 0.. {
        let mut buffer = [0u16; MAX_KEY_NAME];
        let mut length = MAX_KEY_NAME as u32;
        let status = unsafe {
            RegEnumKeyExW(
                class,
                index,
                buffer.as_mut_ptr(),
                &mut length,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS {
            break;
        }
        names.push(String::from_utf16_lossy(&buffer[..length as usize]));
    }
    names.retain(|name| name.bytes().all(|byte| byte.is_ascii_digit()));
    names.sort();
    names
}

fn adapter_at(class: HKEY, instance: &str) -> Option<DisplayAdapter> {
    let instance = wide(instance);
    Some(DisplayAdapter {
        description: string_value(class, &instance, "DriverDesc")?,
        driver_version: string_value(class, &instance, "DriverVersion"),
        hardware_id: string_value(class, &instance, "MatchingDeviceId"),
        provider: string_value(class, &instance, "ProviderName"),
    })
}

fn string_value(class: HKEY, instance: &[u16], name: &str) -> Option<String> {
    let name = wide(name);
    let mut size: u32 = 0;
    let measured = unsafe {
        RegGetValueW(
            class,
            instance.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut size,
        )
    };
    if measured != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
    let read = unsafe {
        RegGetValueW(
            class,
            instance.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if read != ERROR_SUCCESS {
        return None;
    }
    let length = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    Some(
        String::from_utf16_lossy(&buffer[..length])
            .trim()
            .to_owned(),
    )
    .filter(|text| !text.is_empty())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
