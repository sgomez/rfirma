//! Quién abre un esquema de URL en Windows: `Software\Classes` del usuario, que manda sobre el de la máquina.

use std::path::{Path, PathBuf};
use std::ptr;

use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegDeleteTreeW, RegGetValueW, RegSetKeyValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    REG_SZ, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
};

use crate::desktop::domain::error::{DesktopError, Situation};
use crate::desktop::domain::handlers::{UrlHandler, OUR_DESKTOP_FILE};
use crate::desktop::ports::HandlerRegistry;

const OUR_NAME: &str = "rFirma";

/// Una raíz del registro y la ruta de su `Classes`.
#[derive(Clone, Copy, Debug)]
pub struct Hive {
    root: HKEY,
    classes: &'static str,
}

impl Hive {
    /// La rama `Classes` bajo la raíz dada.
    pub const fn new(root: HKEY, classes: &'static str) -> Self {
        Self { root, classes }
    }
}

/// Las dos ramas de `Classes` que Windows mezcla, con el programa que es rFirma.
#[derive(Clone, Debug)]
pub struct Classes {
    user: Hive,
    machine: Hive,
    ours: PathBuf,
}

/// Un manejador escrito en el registro: el programa que lanza su orden `open`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Written {
    program: String,
}

impl Classes {
    /// Las ramas reales del usuario y de la máquina, con el ejecutable en marcha como rFirma.
    pub fn of_this_user() -> Self {
        Self::new(
            Hive::new(HKEY_CURRENT_USER, r"Software\Classes"),
            Hive::new(HKEY_LOCAL_MACHINE, r"Software\Classes"),
            std::env::current_exe().unwrap_or_default(),
        )
    }

    /// Las ramas y el programa dados.
    pub fn new(user: Hive, machine: Hive, ours: PathBuf) -> Self {
        Self {
            user,
            machine,
            ours,
        }
    }

    /// rFirma y los programas que el registro tiene escritos para el esquema.
    pub fn handlers_for(&self, scheme: &str) -> Vec<UrlHandler> {
        let mut handlers = vec![UrlHandler {
            id: OUR_DESKTOP_FILE.to_owned(),
            name: OUR_NAME.to_owned(),
        }];
        for written in [self.user, self.machine]
            .into_iter()
            .filter_map(|hive| written_in(hive, scheme))
        {
            let handler = self.handler_of(&written);
            if !handlers.iter().any(|already| already.id == handler.id) {
                handlers.push(handler);
            }
        }
        handlers
    }

    /// El manejador que usa Windows hoy: el del usuario o, si no hay, el de la máquina.
    pub fn current_handler_for(&self, scheme: &str) -> Option<String> {
        written_in(self.user, scheme)
            .or_else(|| written_in(self.machine, scheme))
            .map(|written| self.handler_of(&written).id)
    }

    /// Deja ese manejador como el que abre el esquema.
    pub fn choose_handler_for(&self, scheme: &str, handler: &str) -> Result<(), DesktopError> {
        if handler == OUR_DESKTOP_FILE {
            return self.write_ours(scheme);
        }
        let of_the_machine = written_in(self.machine, scheme)
            .is_some_and(|written| self.handler_of(&written).id == handler);
        let of_the_user = written_in(self.user, scheme)
            .is_some_and(|written| self.handler_of(&written).id == handler);
        if of_the_user {
            return Ok(());
        }
        if of_the_machine {
            return delete_tree(self.user, scheme);
        }
        Err(DesktopError::new(
            Situation::TheListIsNotWritable,
            format!("«{handler}» no está registrado para {scheme}://"),
        ))
    }

    /// Quita el registro del usuario si es de rFirma; el de otro programa no se toca.
    pub fn remove_ours_for(&self, scheme: &str) -> Result<(), DesktopError> {
        match written_in(self.user, scheme) {
            Some(written) if self.is_ours(&written) => delete_tree(self.user, scheme),
            _ => Ok(()),
        }
    }

    fn write_ours(&self, scheme: &str) -> Result<(), DesktopError> {
        let program = self.ours.display().to_string();
        let key = format!(r"{}\{scheme}", self.user.classes);
        let command = format!(r"{key}\shell\open\command");
        set_string(self.user.root, &key, None, &format!("URL:{scheme}"))?;
        set_string(self.user.root, &key, Some("URL Protocol"), "")?;
        set_string(
            self.user.root,
            &format!(r"{key}\DefaultIcon"),
            None,
            &format!("\"{program}\",0"),
        )?;
        set_string(
            self.user.root,
            &command,
            None,
            &format!("\"{program}\" \"%1\""),
        )
    }

    fn is_ours(&self, written: &Written) -> bool {
        same_program(Path::new(&written.program), &self.ours)
    }

    fn handler_of(&self, written: &Written) -> UrlHandler {
        if self.is_ours(written) {
            return UrlHandler {
                id: OUR_DESKTOP_FILE.to_owned(),
                name: OUR_NAME.to_owned(),
            };
        }
        UrlHandler {
            id: written.program.clone(),
            name: Path::new(&written.program).file_stem().map_or_else(
                || written.program.clone(),
                |stem| stem.to_string_lossy().into_owned(),
            ),
        }
    }
}

impl HandlerRegistry for Classes {
    fn registered_for(&self, scheme: &str) -> Option<Vec<UrlHandler>> {
        Some(self.handlers_for(scheme))
    }

    fn current_default_for(&self, scheme: &str) -> Option<String> {
        self.current_handler_for(scheme)
    }

    fn choose_for(&self, scheme: &str, handler: &str) -> Result<(), DesktopError> {
        self.choose_handler_for(scheme, handler)
    }

    fn remove_for(&self, scheme: &str) -> Result<(), DesktopError> {
        self.remove_ours_for(scheme)
    }
}

/// El programa de una orden `open`: lo que va entre comillas o, si no las hay, hasta el primer espacio.
pub fn program_of(command: &str) -> Option<String> {
    let command = command.trim();
    let program = match command.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next()?,
        None => command.split(' ').next()?,
    };
    (!program.is_empty()).then(|| program.to_owned())
}

fn same_program(written: &Path, ours: &Path) -> bool {
    let canonical = |path: &Path| {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    !ours.as_os_str().is_empty() && canonical(written) == canonical(ours)
}

fn written_in(hive: Hive, scheme: &str) -> Option<Written> {
    let command = get_string(
        hive.root,
        &format!(r"{}\{scheme}\shell\open\command", hive.classes),
    )?;
    program_of(&command).map(|program| Written { program })
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn get_string(root: HKEY, key: &str) -> Option<String> {
    let key = wide(key);
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
    let mut size = 0u32;
    let asked = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            ptr::null(),
            flags,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut size,
        )
    };
    if asked != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
    let read = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            ptr::null(),
            flags,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if read != ERROR_SUCCESS {
        return None;
    }
    let text = String::from_utf16_lossy(&buffer);
    Some(text.trim_end_matches('\0').to_owned())
}

fn set_string(root: HKEY, key: &str, name: Option<&str>, value: &str) -> Result<(), DesktopError> {
    let wide_key = wide(key);
    let wide_name = name.map(wide);
    let data = wide(value);
    let name_pointer: PCWSTR = wide_name.as_ref().map_or(ptr::null(), |name| name.as_ptr());
    let written = unsafe {
        RegSetKeyValueW(
            root,
            wide_key.as_ptr(),
            name_pointer,
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        )
    };
    if written == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(DesktopError::new(
            Situation::TheListIsNotWritable,
            format!("el registro no deja escribir «{key}» (código {written})"),
        ))
    }
}

fn delete_tree(hive: Hive, scheme: &str) -> Result<(), DesktopError> {
    let key = wide(&format!(r"{}\{scheme}", hive.classes));
    let deleted = unsafe { RegDeleteTreeW(hive.root, key.as_ptr()) };
    if deleted == ERROR_SUCCESS || deleted == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(DesktopError::new(
            Situation::TheListIsNotWritable,
            format!("el registro no deja borrar {scheme}:// (código {deleted})"),
        ))
    }
}

#[cfg(test)]
mod tests;
