//! El registro de manejadores del escritorio detrás del puerto `HandlerRegistry`: canal, GIO y `mimeapps.list`.

use std::path::PathBuf;

use crate::desktop::adapters::channel::{
    registered_handlers_for_scheme, Channel, RegisteredHandlers,
};
use crate::desktop::adapters::choice::{choose_handler_for_scheme, current_default_for_scheme};
use crate::desktop::domain::error::DesktopError;
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
}
