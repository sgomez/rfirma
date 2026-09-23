//! Las peticiones de `save` y `load`.

use super::super::codes::Parameter;
use super::super::data_source::DataSource;
use super::super::refusal::Refusal;
use super::super::url::AfirmaUrl;
use super::document::data_of;
use super::properties::{comma_list, optional};
use super::SiteOperation;

const FORBIDDEN_IN_A_FILENAME: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

const FORBIDDEN_IN_AN_EXTENSION: [char; 11] =
    ['\\', '/', ':', '*', '?', '"', '<', '>', '|', ';', ' '];

/// La petición de `save`: guardar un fichero en el equipo (`UrlParametersToSave`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    data: Vec<u8>,
    title: Option<String>,
    filename: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
}

impl SaveRequest {
    /// El fichero que la sede pide guardar, en bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Título del diálogo de guardado, si la sede lo declaró.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Nombre de fichero que propone la sede.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Extensiones admitidas por el filtro del diálogo.
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Descripción del filtro de extensiones, si la sede la declaró.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// La petición de `load`: cargar uno o varios ficheros del equipo (`UrlParametersToLoad`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadRequest {
    title: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
    starting_folder: Option<String>,
    multiple: bool,
}

impl LoadRequest {
    /// Título del selector, si la sede lo declaró.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Extensiones admitidas por el filtro del selector.
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Descripción del filtro de extensiones, si la sede la declaró.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Carpeta inicial sugerida por la sede, nunca la única fuente de lectura.
    pub fn starting_folder(&self) -> Option<&str> {
        self.starting_folder.as_deref()
    }

    /// Si la sede pide varios ficheros (`multiload=true`) o uno solo.
    pub fn multiple(&self) -> bool {
        self.multiple
    }
}

/// La petición de guardado: solo `dat` es obligatorio (`ProtocolInvocationLauncherSave`, 1.9.2).
pub(super) fn save_request(
    url: &AfirmaUrl,
    data: &dyn DataSource,
) -> Result<SiteOperation, Refusal> {
    let filename = optional(url, "filename");
    if let Some(filename) = &filename {
        check_filename(filename)?;
    }
    if let Some(extensions) = optional(url, "exts") {
        check_extensions(&extensions)?;
    }

    Ok(SiteOperation::Save(SaveRequest {
        data: data_of(url, data)?,
        title: optional(url, "title"),
        filename,
        extensions: comma_list(url, "exts"),
        description: optional(url, "desc"),
    }))
}

/// La petición de carga: nada es obligatorio (`ProtocolInvocationLauncherLoad`, 1.9.2).
pub(super) fn load_request(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    Ok(SiteOperation::Load(LoadRequest {
        title: optional(url, "title"),
        extensions: comma_list(url, "exts"),
        description: optional(url, "desc"),
        starting_folder: optional(url, "filePath"),
        multiple: url
            .parameter("multiload")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
    }))
}

/// Un nombre de fichero sin los caracteres que el original prohíbe, o el `SAF_03` que lo nombra.
pub(super) fn check_filename(candidate: &str) -> Result<(), Refusal> {
    check_free_of(candidate, &FORBIDDEN_IN_A_FILENAME, Parameter::Filename)
}

/// Las extensiones sin los caracteres que el original prohíbe, el `;` y el espacio incluidos.
fn check_extensions(candidate: &str) -> Result<(), Refusal> {
    check_free_of(candidate, &FORBIDDEN_IN_AN_EXTENSION, Parameter::Extensions)
}

fn check_free_of(candidate: &str, forbidden: &[char], blame: Parameter) -> Result<(), Refusal> {
    match candidate.chars().find(|it| forbidden.contains(it)) {
        Some(character) => Err(Refusal::about(
            blame,
            format!("el parametro '{blame}' trae un caracter que no se admite: {character}"),
        )),
        None => Ok(()),
    }
}
