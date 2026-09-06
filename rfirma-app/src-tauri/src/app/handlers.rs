//! Casos de uso para consultar y registrar manejadores de afirma:// en el escritorio (ADR-0015).

use crate::commands::views::{UrlHandlerView, UrlHandlersView};
use crate::commands::Failure;
use crate::desktop::choice::{choose_handler_for_scheme, current_default_for_scheme};
use crate::desktop::error::Situation;
use crate::desktop::{
    registered_handlers_for_scheme, Channel, RegisteredHandlers, OUR_DESKTOP_FILE,
};
use std::path::Path;

/// Esquema de URL gestionado por la aplicación.
pub const SCHEME: &str = "afirma";

/// Consulta el estado y manejadores disponibles para el esquema afirma://.
pub fn who_handles(channel: Channel, list: &Path) -> UrlHandlersView {
    match registered_handlers_for_scheme(channel, SCHEME) {
        RegisteredHandlers::NotAvailableInsideTheSandbox => UrlHandlersView {
            available: false,
            handlers: Vec::new(),
            current: None,
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
        RegisteredHandlers::Known(handlers) => UrlHandlersView {
            available: true,
            handlers: handlers
                .iter()
                .map(|handler| UrlHandlerView {
                    id: handler.id().to_owned(),
                    name: handler.name().to_owned(),
                })
                .collect(),
            current: current_default_for_scheme(channel, list, SCHEME),
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
    }
}

/// Registra un manejador como predeterminado para afirma:// en mimeapps.list.
pub fn chosen(channel: Channel, list: &Path, handler: &str) -> Result<(), Failure> {
    choose_handler_for_scheme(channel, list, SCHEME, handler)?;
    Ok(())
}

/// Clave del catálogo asociada a cada situación de error del escritorio (ADR-0009).
pub fn situation_name(situation: Situation) -> &'static str {
    match situation {
        Situation::NotAvailableInsideTheSandbox => "handlerNotAvailable",
        Situation::TheListIsNotReadable => "handlerListUnreadable",
        Situation::TheListIsNotWritable => "handlerListUnwritable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_the_sandbox_nothing_can_be_known() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let view = who_handles(Channel::Flatpak, &directory.path().join("mimeapps.list"));

        assert!(!view.available);
        assert!(view.handlers.is_empty());
        assert_eq!(view.current, None);
    }

    #[test]
    fn outside_the_sandbox_the_written_choice_is_the_one_shown() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");
        chosen(Channel::Native, &list, OUR_DESKTOP_FILE).expect("deberia escribirse");

        let view = who_handles(Channel::Native, &list);

        assert!(view.available);
        assert_eq!(view.current.as_deref(), Some(OUR_DESKTOP_FILE));
    }

    #[test]
    fn our_own_launcher_crosses_with_the_answer() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let view = who_handles(Channel::Native, &directory.path().join("mimeapps.list"));

        assert_eq!(view.ours, OUR_DESKTOP_FILE);
    }

    #[test]
    fn choosing_inside_the_sandbox_fails_with_its_own_situation() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let failure = chosen(
            Channel::Flatpak,
            &directory.path().join("mimeapps.list"),
            OUR_DESKTOP_FILE,
        )
        .expect_err("dentro del sandbox no se escribe");

        assert_eq!(failure.situation, "handlerNotAvailable");
        assert!(!failure.detail.is_empty());
    }

    #[test]
    fn every_desktop_situation_has_its_own_catalog_key() {
        let names = [
            situation_name(Situation::NotAvailableInsideTheSandbox),
            situation_name(Situation::TheListIsNotReadable),
            situation_name(Situation::TheListIsNotWritable),
        ];

        assert!(names.iter().all(|name| name.starts_with("handler")));
        assert_eq!(
            names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            names.len()
        );
    }
}
