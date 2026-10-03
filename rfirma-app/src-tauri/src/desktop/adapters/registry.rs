//! `DesktopRegistry`, el registro de manejadores detrás del puerto `HandlerRegistry`: canal, GIO y `mimeapps.list` en Linux, el registro en Windows y Launch Services, pendiente, en macOS.

use std::path::PathBuf;

#[cfg(windows)]
pub mod windows_classes;

use crate::desktop::adapters::channel::{
    registered_handlers_for_scheme, Channel, RegisteredHandlers,
};
use crate::desktop::adapters::choice::{
    choose_handler_for_scheme, current_default_for_scheme, remove_handler_for_scheme,
};
use crate::desktop::domain::error::DesktopError;
#[cfg(not(windows))]
use crate::desktop::domain::error::Situation;
use crate::desktop::domain::handlers::UrlHandler;
use crate::desktop::ports::HandlerRegistry;

/// El escritorio de esta máquina: su canal y su `mimeapps.list`.
pub struct DesktopRegistry {
    channel: Channel,
    list: PathBuf,
}

impl DesktopRegistry {
    /// El registro sobre el canal detectado y la lista del `$HOME`.
    pub fn of(channel: Channel, list: PathBuf) -> Self {
        Self { channel, list }
    }
}

impl HandlerRegistry for DesktopRegistry {
    fn registered_for(&self, scheme: &str) -> Option<Vec<UrlHandler>> {
        match registered_handlers_for_scheme(self.channel, scheme) {
            RegisteredHandlers::NotAvailableInsideTheSandbox => None,
            RegisteredHandlers::Known(handlers) => Some(
                handlers
                    .iter()
                    .map(|handler| UrlHandler {
                        id: handler.id().to_owned(),
                        name: handler.name().to_owned(),
                    })
                    .collect(),
            ),
        }
    }

    fn current_default_for(&self, scheme: &str) -> Option<String> {
        current_default_for_scheme(self.channel, &self.list, scheme)
    }

    fn choose_for(&self, scheme: &str, handler: &str) -> Result<(), DesktopError> {
        choose_handler_for_scheme(self.channel, &self.list, scheme, handler)?;
        Ok(())
    }

    fn remove_for(&self, scheme: &str) -> Result<(), DesktopError> {
        remove_handler_for_scheme(self.channel, &self.list, scheme)
    }
}

/// Quién abre `afirma://` en este escritorio, para leerlo; sin `$HOME`, con una lista vacía.
#[cfg(target_os = "linux")]
pub fn this_desktop() -> Box<dyn HandlerRegistry> {
    let list =
        crate::desktop::adapters::choice::mimeapps_list_from_environment().unwrap_or_default();
    Box::new(DesktopRegistry::of(Channel::detected(), list))
}

/// Quién abre `afirma://` en este escritorio, para escribirlo, o por qué no se puede.
#[cfg(target_os = "linux")]
pub fn this_desktop_to_write() -> Result<Box<dyn HandlerRegistry>, DesktopError> {
    let list = crate::desktop::adapters::choice::mimeapps_list_from_environment()
        .map_err(|error| DesktopError::new(Situation::TheListIsNotWritable, error.to_string()))?;
    Ok(Box::new(DesktopRegistry::of(Channel::detected(), list)))
}

/// Quién abre `afirma://` en este escritorio, para leerlo: el registro del usuario y el de la máquina.
#[cfg(windows)]
pub fn this_desktop() -> Box<dyn HandlerRegistry> {
    Box::new(windows_classes::Classes::of_this_user())
}

/// Quién abre `afirma://` en este escritorio, para escribirlo: la rama del usuario.
#[cfg(windows)]
pub fn this_desktop_to_write() -> Result<Box<dyn HandlerRegistry>, DesktopError> {
    Ok(this_desktop())
}

/// Quién abre `afirma://` en este escritorio, para leerlo: Launch Services, aún pendiente.
#[cfg(target_os = "macos")]
pub fn this_desktop() -> Box<dyn HandlerRegistry> {
    Box::new(PendingMacosLaunchServices)
}

/// Quién abre `afirma://` en este escritorio, para escribirlo: Launch Services, aún pendiente.
#[cfg(target_os = "macos")]
pub fn this_desktop_to_write() -> Result<Box<dyn HandlerRegistry>, DesktopError> {
    Ok(this_desktop())
}

/// El registro de manejadores sobre Launch Services de macOS, que aún no existe: ni se consulta ni se escribe.
#[cfg(target_os = "macos")]
pub struct PendingMacosLaunchServices;

#[cfg(target_os = "macos")]
impl HandlerRegistry for PendingMacosLaunchServices {
    fn registered_for(&self, _scheme: &str) -> Option<Vec<UrlHandler>> {
        None
    }

    fn current_default_for(&self, _scheme: &str) -> Option<String> {
        None
    }

    fn choose_for(&self, _scheme: &str, _handler: &str) -> Result<(), DesktopError> {
        Err(pending_macos_launch_services())
    }

    fn remove_for(&self, _scheme: &str) -> Result<(), DesktopError> {
        Err(pending_macos_launch_services())
    }
}

#[cfg(target_os = "macos")]
fn pending_macos_launch_services() -> DesktopError {
    DesktopError::new(
        Situation::TheListIsNotWritable,
        "el registro de manejadores de macOS aún no está disponible",
    )
}
