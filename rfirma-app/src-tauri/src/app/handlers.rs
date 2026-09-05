//! **Quién atiende los enlaces `afirma://`**, del escritorio a la ventana y de
//! vuelta (ID-238, ID-239, ID-240).
//!
//! Son dos casos de uso y una sola pregunta: [`who_handles`] contesta lo que se
//! puede saber —la lista del escritorio, el `default` explícito que hay escrito
//! y con qué lanzador se registra rFirma— y [`chosen`] escribe la elección en
//! el `mimeapps.list` de la persona.
//!
//! **Dentro del flatpak no hay nada que preguntar ni que escribir** (ID-240):
//! [`who_handles`] contesta `available: false` sin llamar a GIO, y con eso
//! Preferencias enseña la frase fija que remite a los ajustes del escritorio en
//! vez de un desplegable que no podría cumplir. Aquí no se decide qué frase es:
//! eso es de la ventana.
//!
//! La ventana **no nombra ningún fichero `.desktop` propio**: el de rFirma sale
//! de [`crate::desktop::OUR_DESKTOP_FILE`] y cruza en la misma respuesta, para
//! que el banner pueda saber si ya está elegida sin cablear nada.

use crate::commands::views::{UrlHandlerView, UrlHandlersView};
use crate::commands::Failure;
use crate::desktop::choice::{choose_handler_for_scheme, current_default_for_scheme};
use crate::desktop::error::Situation;
use crate::desktop::{
    registered_handlers_for_scheme, Channel, RegisteredHandlers, OUR_DESKTOP_FILE,
};
use std::path::Path;

/// El esquema del que trata todo este módulo. Uno y solo uno: rFirma no
/// registra ningún otro (ID-237).
pub const SCHEME: &str = "afirma";

/// **Caso de uso.** Qué se puede saber de quién atiende `afirma://`.
///
/// `list` es el `mimeapps.list` de la persona, que en el sandbox no se lee.
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

/// **Caso de uso.** Deja apuntado que `handler` atiende `afirma://`.
///
/// Lo que se escribe es un `default` explícito en el `mimeapps.list` de la
/// persona y nada más (ID-238). El fichero `.desktop` lo manda la ventana, y es
/// uno de los que salieron de [`who_handles`]: aquí no se cablea ninguno.
pub fn chosen(channel: Channel, list: &Path, handler: &str) -> Result<(), Failure> {
    choose_handler_for_scheme(channel, list, SCHEME, handler)?;
    Ok(())
}

/// El nombre en `camelCase` de una situación del escritorio, que es la clave
/// con la que el catálogo la traduce (ID-29, ADR-0009).
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

    /// **Grada A**: dentro del sandbox no hay ni lista ni elegido, y se dice
    /// así —`available: false`— en vez de contestar una lista vacía, que se
    /// leería como «no hay ningún manejador instalado» (ID-240).
    #[test]
    fn inside_the_sandbox_nothing_can_be_known() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let view = who_handles(Channel::Flatpak, &directory.path().join("mimeapps.list"));

        assert!(!view.available);
        assert!(view.handlers.is_empty());
        assert_eq!(view.current, None);
    }

    /// Fuera del sandbox se pregunta de verdad, y lo elegido es lo que hay
    /// escrito en el `mimeapps.list` de la persona.
    #[test]
    fn outside_the_sandbox_the_written_choice_is_the_one_shown() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");
        chosen(Channel::Native, &list, OUR_DESKTOP_FILE).expect("deberia escribirse");

        let view = who_handles(Channel::Native, &list);

        assert!(view.available);
        assert_eq!(view.current.as_deref(), Some(OUR_DESKTOP_FILE));
    }

    /// El lanzador propio cruza con la respuesta: la ventana no cablea ningún
    /// nombre de aplicación, ni el de rFirma ni el de nadie (ID-238).
    #[test]
    fn our_own_launcher_crosses_with_the_answer() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let view = who_handles(Channel::Native, &directory.path().join("mimeapps.list"));

        assert_eq!(view.ours, OUR_DESKTOP_FILE);
    }

    /// Dentro del sandbox elegir fracasa con su situación, y no en silencio
    /// (ID-240).
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

    /// Cada situación del escritorio tiene su clave del catálogo, y ninguna
    /// cae en «desconocido» (ADR-0009).
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
