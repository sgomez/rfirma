//! **El canal de distribución** en el que corre este proceso, y **quién dice
//! el escritorio que atiende** `afirma://` (ID-237, ID-240, ID-242, #340).
//!
//! Las dos preguntas van juntas porque la segunda solo tiene sentido a la luz
//! de la primera. Fuera del sandbox —`.deb` y `.rpm`— GIO sabe leer el
//! `mimeinfo.cache` del sistema y contesta de verdad. Dentro del flatpak no
//! hay ninguna pregunta que valga la pena hacer (ID-240), medido:
//!
//! - GIO contesta `None`/lista vacía a todos los esquemas, porque el sandbox
//!   no ve el `mimeapps.list` del anfitrión.
//! - No existe ningún portal para manejadores predeterminados:
//!   `OpenURI.SchemeSupported` dice *si hay alguien*, nunca *quién*.
//! - `xdg-mime` no está en el runtime.
//! - `set_as_default_for_type()` **devuelve `true` mintiendo**.
//!
//! Por eso este módulo nunca intenta ninguna de esas cuatro cosas dentro del
//! sandbox: la respuesta correcta ahí es «no se puede saber», no un intento
//! que parece funcionar y no funciona. La frase fija que se enseña en ese caso
//! es cosa de Preferencias (#364); aquí solo se detecta y se lee.
//!
//! **Registro pasivo (ID-237):** lo único que se escribe aquí es el
//! `mimeapps.list` **de la persona**, y solo cuando ella elige manejador. No
//! hay ningún fichero de este repositorio que escriba en un `mimeapps.list`
//! del sistema, ni que toque el orden alfabético del `mimeinfo.cache`, ni que
//! copie el truco de Firefox de AutoFirma. El registro de la aplicación sigue
//! siendo pasivo: el `MimeType=x-scheme-handler/afirma;` declarado en los
//! lanzadores de `packaging/`, que el escritorio recoge por su cuenta.

//!
//! | Pieza | Qué es |
//! |---|---|
//! | este fichero | El canal, y quién dice el escritorio que atiende un esquema. |
//! | [`choice`] | Elegir manejador: el `default` explícito en el `mimeapps.list` de la persona (ID-238, ID-241). |
//! | [`error`] | Las situaciones de leer y escribir esa elección (ADR-0009). |

pub mod choice;
pub mod error;

use std::path::Path;

/// Dónde se apunta a sí mismo un flatpak: el fichero que el propio `bwrap`
/// deja dentro del sandbox. Mismo criterio que
/// [`crate::destination::the_original_folder_can_be_offered`], preguntado
/// aquí por una razón distinta.
const SANDBOX_MARKER: &str = "/.flatpak-info";

/// El canal de distribución en el que corre este proceso (ADR-0004).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// `.deb` o `.rpm`: sin sandbox, el escritorio contesta de verdad.
    Native,
    /// El flatpak: cualquier pregunta al escritorio sobre manejadores está
    /// cerrada o miente (ID-240).
    Flatpak,
}

impl Channel {
    /// El canal detectado por `/.flatpak-info`.
    pub fn detected() -> Self {
        Self::over(Path::new(SANDBOX_MARKER))
    }

    /// La misma pregunta sobre una marca cualquiera, que es lo que la hace
    /// comprobable sin un flatpak montado.
    fn over(marker: &Path) -> Self {
        if marker.exists() {
            Self::Flatpak
        } else {
            Self::Native
        }
    }
}

/// Lo que se puede saber sobre quién atiende un esquema de URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisteredHandlers {
    /// Fuera del sandbox: la lista que da el escritorio, tal cual. Ningún
    /// nombre de aplicación está cableado en este módulo — lo que haya aquí
    /// es lo que GIO haya encontrado, sea lo que sea.
    Known(Vec<RegisteredHandler>),
    /// Dentro del sandbox no hay pregunta posible (ID-240): no se ha llamado
    /// a GIO, ni al portal, ni a `xdg-mime`.
    NotAvailableInsideTheSandbox,
}

/// Quién dice el escritorio que atiende `scheme` (sin el `x-scheme-handler/`
/// por delante; lo añade esta función).
///
/// Dentro del sandbox no se ejecuta ninguna llamada: la rama de
/// [`Channel::Flatpak`] no toca GIO en absoluto, que es la única forma de que
/// un `set_as_default_for_type()` mintiendo no acabe filtrándose como
/// respuesta (ID-240).
pub fn registered_handlers_for_scheme(channel: Channel, scheme: &str) -> RegisteredHandlers {
    match channel {
        Channel::Flatpak => RegisteredHandlers::NotAvailableInsideTheSandbox,
        Channel::Native => {
            let handlers = gio::AppInfo::all_for_type(&content_type_for(scheme))
                .iter()
                .filter_map(|info| {
                    // Sin `.desktop` no hay nada que escribir en el
                    // `[Default Applications]`, así que un manejador sin
                    // identificador no es elegible y no se ofrece.
                    let id = gio::prelude::AppInfoExt::id(info)?;
                    Some(RegisteredHandler::new(
                        gio::prelude::AppInfoExt::name(info).to_string(),
                        id.to_string(),
                    ))
                })
                .collect();
            RegisteredHandlers::Known(handlers)
        }
    }
}

/// Un manejador registrado, tal y como lo da el escritorio: lo que la persona
/// lee y lo que se escribe por ella.
///
/// Los dos no son el mismo dato y hacen falta los dos: el desplegable enseña
/// el nombre —«AutoFirma», «Firefox», lo que sea— y el `mimeapps.list` solo
/// entiende de ficheros `.desktop`. Ninguno de los dos está cableado aquí
/// (ID-238).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredHandler {
    name: String,
    id: String,
}

impl RegisteredHandler {
    /// Un manejador con su nombre visible y su fichero `.desktop`.
    pub fn new(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
        }
    }

    /// El nombre que enseña el desplegable, tal y como lo dio el escritorio.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// El fichero `.desktop` con el que se escribe el `default`.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// El tipo de contenido de un esquema de URL, que es como lo nombra el
/// `mimeapps.list`.
fn content_type_for(scheme: &str) -> String {
    format!("x-scheme-handler/{scheme}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Con la marca puesta, el canal es el flatpak — el mismo criterio que
    /// [`crate::destination::the_original_folder_can_be_offered`] comprueba
    /// para su propia pregunta.
    #[test]
    fn the_marker_present_means_the_flatpak_channel() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let marker = directory.path().join(".flatpak-info");
        fs::write(&marker, b"[Application]\n").expect("deberia escribirse");

        assert_eq!(Channel::over(&marker), Channel::Flatpak);
    }

    /// Sin la marca, el canal es el nativo: `.deb` y `.rpm`.
    #[test]
    fn no_marker_means_the_native_channel() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        assert_eq!(
            Channel::over(&directory.path().join(".flatpak-info")),
            Channel::Native
        );
    }

    /// La pregunta de verdad se hace sobre `/.flatpak-info`, no sobre otra
    /// cosa.
    #[test]
    fn the_real_question_is_asked_over_the_well_known_marker() {
        assert_eq!(SANDBOX_MARKER, "/.flatpak-info");
    }

    /// Dentro del sandbox no hay lista que leer: ni vacía, ni con nombres.
    /// Es una situación distinta de «no hay ningún manejador», que sí sería
    /// una lista vacía fuera del sandbox.
    #[test]
    fn inside_the_sandbox_there_is_no_answer_at_all() {
        let handlers = registered_handlers_for_scheme(Channel::Flatpak, "afirma");

        assert_eq!(handlers, RegisteredHandlers::NotAvailableInsideTheSandbox);
    }

    /// Fuera del sandbox la pregunta se hace de verdad, a GIO. El registro
    /// real del escritorio de cada canal **se ensaya, no se prueba aquí**
    /// (TD-65): lo que esta prueba fija es que la rama nativa produce una
    /// lista, cualquiera que sea su contenido en esta máquina.
    #[test]
    fn outside_the_sandbox_the_answer_comes_from_the_desktop() {
        let handlers = registered_handlers_for_scheme(Channel::Native, "afirma");

        assert!(matches!(handlers, RegisteredHandlers::Known(_)));
    }

    /// El manejador que se ofrece lleva las dos cosas: lo que se lee y lo que
    /// se escribe.
    #[test]
    fn a_handler_carries_both_the_name_and_the_desktop_file() {
        let handler = RegisteredHandler::new("AutoFirma", "autofirma.desktop");

        assert_eq!(handler.name(), "AutoFirma");
        assert_eq!(handler.id(), "autofirma.desktop");
    }

    /// Todo manejador que el escritorio nombre se puede escribir como
    /// `default`: uno sin fichero `.desktop` no se ofrece, porque elegirlo no
    /// haría nada.
    #[test]
    fn every_offered_handler_can_be_written_as_a_default() {
        let RegisteredHandlers::Known(handlers) =
            registered_handlers_for_scheme(Channel::Native, "http")
        else {
            panic!("fuera del sandbox tiene que haber lista");
        };

        assert!(handlers.iter().all(|handler| !handler.id().is_empty()));
    }
}
