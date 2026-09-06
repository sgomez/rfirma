//! Registro del manejador predeterminado para un esquema en mimeapps.list (ADR-0015).

use super::{content_type_for, Channel};
use crate::desktop::error::{DesktopError, Situation};
use crate::paths::{xdg_config_home, HomeUnknown};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Nombre del fichero de asociaciones predeterminadas del usuario.
const MIMEAPPS_LIST: &str = "mimeapps.list";

/// Sección del fichero donde se declaran las aplicaciones predeterminadas.
const DEFAULT_APPLICATIONS: &str = "[Default Applications]";

/// Circunstancias externas que pueden condicionar la preferencia registrada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceOverride {
    /// Firefox mantiene su propia tabla interna de asociaciones en sus preferencias.
    FirefoxKeepsItsOwn,
}

/// Lista de circunstancias externas reconocidas.
const WHAT_CAN_OVERRIDE_THE_CHOICE: [ChoiceOverride; 1] = [ChoiceOverride::FirefoxKeepsItsOwn];

/// Resultado del registro de un manejador predeterminado junto con posibles advertencias.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceWritten {
    list: PathBuf,
    overridden_by: &'static [ChoiceOverride],
}

impl ChoiceWritten {
    /// Fichero mimeapps.list donde se registró la preferencia.
    pub fn list(&self) -> &Path {
        &self.list
    }

    /// Circunstancias externas que pueden condicionar la preferencia.
    pub fn overridden_by(&self) -> &'static [ChoiceOverride] {
        self.overridden_by
    }
}

/// Obtiene la ruta de mimeapps.list a partir de las variables de entorno actuales.
pub fn mimeapps_list_from_environment() -> Result<PathBuf, HomeUnknown> {
    mimeapps_list(&|name| std::env::var_os(name))
}

/// Obtiene la ruta de mimeapps.list resolviendo el entorno con una función provista.
pub fn mimeapps_list(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, HomeUnknown> {
    Ok(xdg_config_home(environment)?.join(MIMEAPPS_LIST))
}

/// Registra el manejador predeterminado para un esquema en el fichero mimeapps.list.
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

/// Consulta el manejador predeterminado configurado actualmente para un esquema.
pub fn current_default_for_scheme(channel: Channel, list: &Path, scheme: &str) -> Option<String> {
    if channel == Channel::Flatpak {
        return None;
    }
    let content = fs::read_to_string(list).ok()?;
    default_in(&content, &content_type_for(scheme))
}

/// Busca el valor predeterminado para un tipo de contenido en el texto del fichero.
fn default_in(content: &str, content_type: &str) -> Option<String> {
    let mut inside = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == DEFAULT_APPLICATIONS;
            continue;
        }
        if !inside || key_of(line) != Some(content_type) {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        return value
            .split(';')
            .map(str::trim)
            .find(|entry| !entry.is_empty())
            .map(str::to_owned);
    }
    None
}

/// Inserta o actualiza la clave del esquema dentro de [Default Applications].
fn with_explicit_default(content: &str, content_type: &str, handler: &str) -> String {
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

/// Extrae la clave de una línea clave=valor si no está comentada.
fn key_of(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(key, _)| key.trim())
}

/// Inserta una línea antes de las líneas vacías finales del bloque.
fn insert_before_the_blank_tail(lines: &mut Vec<String>, entry: &str) {
    let blanks = lines
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty())
        .count();
    lines.insert(lines.len() - blanks, entry.to_owned());
}

/// Escribe el contenido atómicamente mediante un fichero temporal.
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

    #[test]
    fn the_list_that_gets_written_is_the_one_in_the_home() {
        let list = mimeapps_list(&|name| match name {
            "XDG_CONFIG_HOME" => Some(OsString::from("/home/quien/.config")),
            _ => None,
        })
        .expect("deberia resolverse");

        assert_eq!(list, PathBuf::from("/home/quien/.config/mimeapps.list"));
    }

    #[test]
    fn choosing_a_handler_writes_an_explicit_default() {
        let updated = with_explicit_default("", "x-scheme-handler/afirma", "rfirma.desktop");

        assert_eq!(
            updated,
            "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n"
        );
    }

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

    #[test]
    fn the_written_default_is_what_gets_read_back() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");
        choose_handler_for_scheme(Channel::Native, &list, "afirma", "rfirma.desktop")
            .expect("deberia escribirse");

        let current = current_default_for_scheme(Channel::Native, &list, "afirma");

        assert_eq!(current.as_deref(), Some("rfirma.desktop"));
    }

    #[test]
    fn no_list_means_nobody_has_been_chosen() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let current = current_default_for_scheme(
            Channel::Native,
            &directory.path().join("mimeapps.list"),
            "afirma",
        );

        assert_eq!(current, None);
    }

    #[test]
    fn an_added_association_is_not_the_default() {
        let content = "[Added Associations]\nx-scheme-handler/afirma=autofirma.desktop;\n";

        assert_eq!(default_in(content, "x-scheme-handler/afirma"), None);
    }

    #[test]
    fn the_first_entry_of_the_list_is_the_one_that_answers() {
        let content =
            "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;autofirma.desktop;\n";

        assert_eq!(
            default_in(content, "x-scheme-handler/afirma"),
            Some("rfirma.desktop".to_owned())
        );
    }

    #[test]
    fn inside_the_sandbox_nothing_is_read_either() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let list = directory.path().join("mimeapps.list");
        std::fs::write(
            &list,
            "[Default Applications]\nx-scheme-handler/afirma=rfirma.desktop;\n",
        )
        .expect("deberia escribirse");

        assert_eq!(
            current_default_for_scheme(Channel::Flatpak, &list, "afirma"),
            None
        );
    }
}
