//! Los diálogos del portal que abre la sede: el de guardado, con las pistas de `signandsave`, y el selector de carga.

use std::path::{Path, PathBuf};

use crate::site::domain::signing::SiteSignature;

use super::PendingSignature;

/// Pistas de guardado de `signandsave`, calculadas antes de firmar y usadas tras la postfirma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavingHints {
    /// Nombre de fichero propuesto (`signandsave` no declara `title`; siempre hay uno).
    pub filename: String,
    /// Extensiones admitidas por el filtro del diálogo de guardado.
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones, si la sede la declaró.
    pub description: Option<String>,
    /// Carpeta inicial sugerida por la sede, nunca la fuente de la escritura.
    pub starting_folder: Option<String>,
}

impl SavingHints {
    /// El paso de guardado tras la postfirma, contestando con el mismo par que `sign`.
    pub fn into_consent(self, signed: &SiteSignature) -> SavingConsent {
        SavingConsent {
            data: signed.signature.clone(),
            title: None,
            filename: Some(self.filename),
            extensions: self.extensions,
            description: self.description,
            starting_folder: self.starting_folder,
            signer_der: Some(signed.signer_der.clone()),
        }
    }
}

/// Datos para el diálogo de guardado del portal: el nombre cruza, la ruta nunca (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavingConsent {
    /// El fichero que la sede pide guardar, en bytes.
    pub data: Vec<u8>,
    /// Título del diálogo declarado por la sede.
    pub title: Option<String>,
    /// Nombre de fichero propuesto por la sede.
    pub filename: Option<String>,
    /// Extensiones admitidas por el filtro del diálogo.
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones declarada por la sede.
    pub description: Option<String>,
    /// Carpeta inicial sugerida por la sede, si la declaró (`signandsave`; `save` no la tiene).
    pub starting_folder: Option<String>,
    /// El DER del firmante con el que contestar si esto viene de `signandsave`, `None` en `save`.
    pub signer_der: Option<Vec<u8>>,
}

impl SavingConsent {
    /// La carpeta en la que se abre el diálogo: la que declaró la sede o, si no, `home`.
    pub fn dialog_folder(&self, home: Option<&Path>) -> Option<PathBuf> {
        declared_or_home(self.starting_folder.as_deref(), home)
    }
}

impl LoadingConsent {
    /// La carpeta en la que se abre el selector: la que declaró la sede o, si no, `home`.
    pub fn dialog_folder(&self, home: Option<&Path>) -> Option<PathBuf> {
        declared_or_home(self.starting_folder.as_deref(), home)
    }
}

fn declared_or_home(declared: Option<&str>, home: Option<&Path>) -> Option<PathBuf> {
    declared
        .map(PathBuf::from)
        .or_else(|| home.map(Path::to_path_buf))
}

/// Datos para el selector de carga del portal: el nombre cruza, la ruta nunca (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadingConsent {
    /// Título del selector declarado por la sede.
    pub title: Option<String>,
    /// Nombre que la sede propone al selector (`filenameActualName`), si lo declaró.
    pub filename: Option<String>,
    /// Extensiones admitidas por el filtro del selector.
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones declarada por la sede.
    pub description: Option<String>,
    /// Carpeta inicial sugerida por la sede, nunca la fuente de la lectura.
    pub starting_folder: Option<String>,
    /// Si la sede pide varios ficheros (`multiload=true`) o uno solo.
    pub multiple: bool,
    /// Si este selector viene de una firma sin `dat`, la petición que continúa con el documento
    /// elegido; `None` cuando es un `load` corriente que contesta a la sede.
    pub to_sign: Option<Box<PendingSignature>>,
}
