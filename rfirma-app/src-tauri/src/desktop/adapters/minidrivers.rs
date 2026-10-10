//! Los tipos de tarjeta que Windows tiene dados de alta como minidrivers, leídos del registro; no la tarjeta conectada.

/// Los nombres de los tipos de tarjeta registrados; vacío fuera de Windows.
#[cfg(not(windows))]
pub fn registered_minidrivers() -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
pub use registry::registered_minidrivers;

#[cfg(windows)]
mod registry {
    use std::ptr;

    use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
        KEY_WOW64_64KEY,
    };

    const SMART_CARDS_KEY: &str = r"SOFTWARE\Microsoft\Cryptography\Calais\SmartCards";
    const MAX_KEY_NAME: usize = 256;

    /// Los nombres de los tipos de tarjeta registrados, ordenados; vacío si no hay ninguno o no se leen.
    pub fn registered_minidrivers() -> Vec<String> {
        let path: Vec<u16> = SMART_CARDS_KEY.encode_utf16().chain(Some(0)).collect();
        let mut key: HKEY = ptr::null_mut();
        let opened = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                path.as_ptr(),
                0,
                KEY_READ | KEY_WOW64_64KEY,
                &mut key,
            )
        };
        if opened != ERROR_SUCCESS {
            return Vec::new();
        }
        let mut names = Vec::new();
        for index in 0.. {
            let mut buffer = [0u16; MAX_KEY_NAME];
            let mut length = MAX_KEY_NAME as u32;
            let status = unsafe {
                RegEnumKeyExW(
                    key,
                    index,
                    buffer.as_mut_ptr(),
                    &mut length,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if status == ERROR_NO_MORE_ITEMS || status != ERROR_SUCCESS {
                break;
            }
            names.push(String::from_utf16_lossy(&buffer[..length as usize]));
        }
        unsafe { RegCloseKey(key) };
        names.sort();
        names
    }
}
