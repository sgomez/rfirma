//! **Elegir quién atiende un esquema**: el `default` explícito que se escribe
//! en el `mimeapps.list` de la persona (ID-238), y lo que aun así puede quedar
//! por encima de él (ID-241).
//!
//! El fichero que se toca es **el del `$HOME` y ninguno más**. Ni el del
//! sistema, ni el orden alfabético del `mimeinfo.cache`, ni el truco de
//! Firefox de AutoFirma, que suprime un consentimiento (ID-237).
//!
//! Lo que decide dónde entra la línea es [`with_explicit_default`], que es
//! puro y se prueba entero sin tocar el disco.

use super::{content_type_for, Channel};
use crate::desktop::error::{DesktopError, Situation};
use crate::paths::{xdg_config_home, HomeUnknown};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// El fichero donde el escritorio guarda las elecciones de la persona
/// (freedesktop, *mime-apps spec*). Vive en su directorio de configuración y
/// **en ningún otro sitio**: el del sistema no se toca (ID-237).
const MIMEAPPS_LIST: &str = "mimeapps.list";

/// El grupo donde vive un `default` **explícito**, y no otro. `[Added
/// Associations]` solo añade candidatos a la lista; sin una entrada aquí,
/// GNOME saca un selector en cada invocación y no ofrece recordar la elección
/// (ID-238).
const DEFAULT_APPLICATIONS: &str = "[Default Applications]";

/// Lo que puede dejar sin efecto el `default` recién escrito (ID-241).
///
/// No es un fallo ni una detección: es lo que rFirma **sabe de antemano** que
/// manda por encima del `mimeapps.list`, y que por eso la ventana tiene que
/// decir siempre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceOverride {
    /// Firefox guarda su propia elección en *Preferences → Applications* y la
    /// impone: una URL `afirma://` pinchada dentro de Firefox va a donde diga
    /// Firefox, no a donde diga el `mimeapps.list`.
    ///
    /// No se comprueba porque no se puede: vive en el `handlers.json` de un
    /// perfil que puede ser cualquiera, y leerlo sería adivinar. Se avisa
    /// siempre.
    FirefoxKeepsItsOwn,
}

/// Todo lo que puede quedar por encima de la elección. Es una constante, no
/// una medida del sistema.
const WHAT_CAN_OVERRIDE_THE_CHOICE: [ChoiceOverride; 1] = [ChoiceOverride::FirefoxKeepsItsOwn];

/// La elección ya escrita, y lo que aun así puede quedar por encima de ella.
///
/// Van juntas a propósito: quien escribe el `default` recibe en la misma mano
/// la advertencia que tiene que enseñar, y así no hay forma de escribirlo y
/// olvidarse de decirlo (ID-241).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceWritten {
    list: PathBuf,
    overridden_by: &'static [ChoiceOverride],
}

impl ChoiceWritten {
    /// El `mimeapps.list` en el que quedó apuntada la elección.
    pub fn list(&self) -> &Path {
        &self.list
    }

    /// Lo que puede mandar por encima de lo que se acaba de escribir.
    pub fn overridden_by(&self) -> &'static [ChoiceOverride] {
        self.overridden_by
    }
}

/// El `mimeapps.list` de la persona, resuelto contra el entorno del proceso.
pub fn mimeapps_list_from_environment() -> Result<PathBuf, HomeUnknown> {
    mimeapps_list(&|name| std::env::var_os(name))
}

/// El `mimeapps.list` de la persona, leyendo el entorno con `environment`.
///
/// Es pública con el entorno por delante por lo mismo que
/// [`crate::paths::Paths::resolve`]: cambiar el entorno del proceso es global
/// y las pruebas corren en hilos.
pub fn mimeapps_list(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, HomeUnknown> {
    Ok(xdg_config_home(environment)?.join(MIMEAPPS_LIST))
}

/// Deja apuntado que `handler` —un fichero `.desktop`— atiende `scheme`, con
/// un `default` **explícito** en `[Default Applications]` de `list` (ID-238).
///
/// Lo que ya hubiera en el fichero sobrevive entero: los demás grupos, las
/// demás asociaciones, los comentarios y el orden. Lo único que cambia es la
/// línea de este esquema, que se sustituye si estaba y se añade si no.
///
/// Dentro del sandbox no se intenta: ahí `set_as_default_for_type()` devuelve
/// `true` mintiendo y el `mimeapps.list` que se vería no es el del anfitrión
/// (ID-240).
pub fn choose_handler_for_scheme(
    channel: Channel,
    list: &Path,
    scheme: &str,
    handler: &str,
) -> Result<ChoiceWritten, DesktopError> {
    if channel == Channel::Flatpak {
        return Err(DesktopError::new(
            Situation::NotAvailableInsideTheSandbox,
            format!("{} no es el del anfitrión", list.display()),
        ));
    }
    let current = match fs::read_to_string(list) {
        Ok(current) => current,
        // Que no exista es lo normal la primera vez, y es un fichero vacío.
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(DesktopError::new(
                Situation::TheListIsNotReadable,
                format!("{}: {error}", list.display()),
            ))
        }
    };
    let updated = with_explicit_default(&current, &content_type_for(scheme), handler);
    write_atomically(list, updated.as_bytes()).map_err(|error| {
        DesktopError::new(
            Situation::TheListIsNotWritable,
            format!("{}: {error}", list.display()),
        )
    })?;
    Ok(ChoiceWritten {
        list: list.to_path_buf(),
        overridden_by: &WHAT_CAN_OVERRIDE_THE_CHOICE,
    })
}

/// El contenido del `mimeapps.list` con el `default` de `content_type` puesto
/// a `handler`, y **todo lo demás intacto**.
///
/// Es la mitad pura de [`choose_handler_for_scheme`], y donde está la única
/// decisión que importa: la línea entra en `[Default Applications]` y en
/// ningún otro grupo. Si el grupo no estaba, se añade al final; si estaba y ya
/// tenía este esquema, se sustituye en su sitio en vez de duplicarlo —dos
/// líneas con la misma clave dejan al escritorio eligiendo por su cuenta—.
fn with_explicit_default(content: &str, content_type: &str, handler: &str) -> String {
    // El valor de `[Default Applications]` es una **lista** de *Desktop File
    // ID* en el sentido del formato de fichero clave, y la convención de todo
    // el escritorio es cerrarla con el separador. GLib descarta el trozo
    // vacío final al leer, así que las dos formas valen; se sale con la misma
    // que escriben las demás herramientas para que nadie dude al leer el
    // fichero.
    let entry = format!("{content_type}={handler};");
    let mut lines: Vec<String> = Vec::new();
    let mut inside_the_group = false;
    let mut written = false;

    for line in content.lines() {
        if line.trim_start().starts_with('[') {
            if inside_the_group && !written {
                insert_before_the_blank_tail(&mut lines, &entry);
                written = true;
            }
            inside_the_group = line.trim() == DEFAULT_APPLICATIONS;
            lines.push(line.to_owned());
            continue;
        }
        if inside_the_group && key_of(line) == Some(content_type) {
            if !written {
                lines.push(entry.clone());
                written = true;
            }
            continue;
        }
        lines.push(line.to_owned());
    }

    if inside_the_group && !written {
        insert_before_the_blank_tail(&mut lines, &entry);
        written = true;
    }
    if !written {
        if lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(DEFAULT_APPLICATIONS.to_owned());
        lines.push(entry);
    }
    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
}

/// La clave de una línea `clave=valor`, si la línea es una y no un comentario.
fn key_of(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(key, _)| key.trim())
}

/// Mete `entry` al final del grupo, que es antes de las líneas en blanco que
/// lo separan del siguiente: pegada al último ajuste, no flotando entre dos
/// grupos.
fn insert_before_the_blank_tail(lines: &mut Vec<String>, entry: &str) {
    let blanks = lines
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty())
        .count();
    lines.insert(lines.len() - blanks, entry.to_owned());
}

/// Temporal, `sync_all` y `rename`, como [`crate::memory::store`]: mientras el
/// `rename` no ocurre, el `mimeapps.list` que hay en disco sigue siendo el
/// anterior, entero. Este fichero no es nuestro y puede llevar años de
/// elecciones dentro.
///
/// A diferencia de las memorias de rFirma, aquí **no** se restringe al dueño:
/// un `mimeapps.list` es configuración corriente del escritorio y dejarlo
/// `0600` sería cambiarle a la persona algo que no ha pedido.
fn write_atomically(path: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".rfirma.tmp");
    let temporary = PathBuf::from(temporary);
    let written = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(content)?;
        file.sync_all()
    })();
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: el `mimeapps.list` de la persona sale de su directorio de
    /// configuración, y de ningún sitio del sistema (ID-237).
    #[test]
    fn the_list_that_gets_written_is_the_one_in_the_home() {
        let list = mimeapps_list(&|name| match name {
            "XDG_CONFIG_HOME" => Some(OsString::from("/home/quien/.config")),
            _ => None,
        })
        .expect("deberia resolverse");

        assert_eq!(list, PathBuf::from("/home/quien/.config/mimeapps.list"));
    }

    /// Elegir manejador escribe un `default` **explícito**, y en el grupo que
    /// manda: sin esta entrada GNOME saca un selector en cada invocación
    /// (ID-238).
    #[test]
    fn choosing_a_handler_writes_an_explicit_default() {
        let updated = with_explicit_default("", "x-scheme-handler/afirma", "rfirma.desktop");

        assert_eq!(
            updated,
            "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
        );
    }

    /// El `default` entra en `[Default Applications]` y en ningún otro grupo:
    /// una asociación añadida no elige nada.
    #[test]
    fn the_default_never_lands_in_another_group() {
        let updated = with_explicit_default(
            "[Added Associations]\nx-scheme-handler/afirma=autofirma.desktop;\n",
            "x-scheme-handler/afirma",
            "rfirma.desktop",
        );

        assert_eq!(
            updated,
            "[Added Associations]\nx-scheme-handler/afirma=autofirma.desktop;\n\
             \n[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
        );
    }

    /// Con el grupo ya puesto, la línea se añade dentro, detrás de lo que
    /// hubiera.
    #[test]
    fn an_existing_group_takes_the_line_inside_it() {
        let updated = with_explicit_default(
            "[Default Applications]\napplication/pdf=evince.desktop\n",
            "x-scheme-handler/afirma",
            "rfirma.desktop",
        );

        assert_eq!(
            updated,
            "[Default Applications]\napplication/pdf=evince.desktop\n\
             x-scheme-handler/afirma=rfirma.desktop;\n"
        );
    }

    /// Cambiar de manejador sustituye la línea en su sitio. Duplicarla dejaría
    /// al escritorio eligiendo por su cuenta cuál de las dos vale.
    #[test]
    fn an_existing_default_for_the_scheme_is_replaced_not_duplicated() {
        let updated = with_explicit_default(
            "[Default Applications]\n\
             x-scheme-handler/afirma=autofirma.desktop\n\
             application/pdf=evince.desktop\n",
            "x-scheme-handler/afirma",
            "rfirma.desktop",
        );

        assert_eq!(
            updated,
            "[Default Applications]\n\
             x-scheme-handler/afirma=rfirma.desktop;\n\
             application/pdf=evince.desktop\n"
        );
        assert_eq!(updated.matches("x-scheme-handler/afirma").count(), 1);
    }

    /// El fichero no es nuestro: los demás grupos, los comentarios y el orden
    /// salen tal y como entraron.
    #[test]
    fn everything_else_in_the_list_survives_untouched() {
        let before = "# lo escribio otra cosa\n\
                      [Added Associations]\n\
                      application/pdf=evince.desktop;okular.desktop;\n\
                      \n\
                      [Default Applications]\n\
                      application/pdf=evince.desktop\n\
                      \n\
                      [Removed Associations]\n\
                      text/plain=gedit.desktop;\n";

        let updated = with_explicit_default(before, "x-scheme-handler/afirma", "rfirma.desktop");

        assert!(updated.contains("# lo escribio otra cosa\n[Added Associations]\n"));
        assert!(updated.contains("[Removed Associations]\ntext/plain=gedit.desktop;\n"));
        assert!(updated.contains(
            "[Default Applications]\n\
             application/pdf=evince.desktop\n\
             x-scheme-handler/afirma=rfirma.desktop;\n\n[Removed Associations]"
        ));
    }

    /// Una clave comentada no es la clave: se queda como estaba y la de verdad
    /// se añade aparte.
    #[test]
    fn a_commented_out_line_is_not_the_entry() {
        let updated = with_explicit_default(
            "[Default Applications]\n#x-scheme-handler/afirma=autofirma.desktop\n",
            "x-scheme-handler/afirma",
            "rfirma.desktop",
        );

        assert!(updated.contains("#x-scheme-handler/afirma=autofirma.desktop\n"));
        assert!(updated.contains("\nx-scheme-handler/afirma=rfirma.desktop;\n"));
    }

    /// **Grada B**: sobre un fichero de verdad, y sin que haya que crearlo
    /// antes.
    #[test]
    fn the_choice_lands_on_disk_even_when_there_was_no_list() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("nuevo").join("mimeapps.list");

        let written = choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
            .expect("deberia escribirse");

        assert_eq!(written.list(), list);
        assert_eq!(
            fs::read_to_string(&list).expect("deberia leerse"),
            "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
        );
    }

    /// La escritura es de temporal y `rename`: cuando termina no queda ningún
    /// resto al lado del fichero.
    #[test]
    fn writing_the_choice_leaves_no_leftovers_behind() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");

        choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
            .expect("deberia escribirse");

        let left: Vec<_> = fs::read_dir(directory.path())
            .expect("deberia leerse el directorio")
            .map(|entry| entry.expect("deberia haber entrada").file_name())
            .collect();
        assert_eq!(left, vec![OsString::from("mimeapps.list")]);
    }

    /// Dentro del sandbox no se escribe nada, ni siquiera un fichero que
    /// pareciera el bueno: ahí el `mimeapps.list` visible no es el del
    /// anfitrión y `set_as_default_for_type()` devuelve `true` mintiendo
    /// (ID-240).
    #[test]
    fn inside_the_sandbox_no_default_is_written() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");

        let refused =
            choose_handler_for_scheme(Channel::Flatpak, &list, "afirma", "rfirma.desktop")
                .expect_err("no deberia escribirse dentro del sandbox");

        assert_eq!(refused.situation(), Situation::NotAvailableInsideTheSandbox);
        assert!(!list.exists());
    }

    /// Escrita la elección, quien la escribió recibe en la misma mano lo que
    /// puede quedar por encima: Firefox guarda la suya aparte y la impone
    /// (ID-241).
    #[test]
    fn the_written_choice_carries_what_firefox_can_override() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");

        let written = choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
            .expect("deberia escribirse");

        assert_eq!(
            written.overridden_by(),
            [ChoiceOverride::FirefoxKeepsItsOwn]
        );
    }
}
