//! Andamio de grada A de `documents`: el disco de mentira que los casos de uso llevan detrás.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::documents::domain::destination::FolderFact;
use crate::documents::ports::DocumentFiles;

/// El instante de modificación que este disco contesta de todo lo que tiene.
pub const MODIFIED: u64 = 1_700_000_000;

/// Un disco de mentira: carpetas, ficheros y los que se niegan a abrirse.
#[derive(Default)]
pub struct InMemoryFiles {
    folders: Mutex<Vec<PathBuf>>,
    files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    unreadable: Mutex<Vec<PathBuf>>,
    links: Mutex<BTreeMap<PathBuf, PathBuf>>,
}

impl InMemoryFiles {
    /// Un disco vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Con esa carpeta dentro.
    pub fn with_folder(self, folder: impl Into<PathBuf>) -> Self {
        crate::lock(&self.folders).push(folder.into());
        self
    }

    /// Con ese fichero dentro.
    pub fn with_file(self, path: impl Into<PathBuf>, bytes: &[u8]) -> Self {
        crate::lock(&self.files).insert(path.into(), bytes.to_vec());
        self
    }

    /// Con ese atajo que lleva a la ruta de verdad.
    pub fn with_link(self, alias: impl Into<PathBuf>, real: impl Into<PathBuf>) -> Self {
        crate::lock(&self.links).insert(alias.into(), real.into());
        self
    }

    /// Deja ese fichero ahí, como si alguien lo hubiera puesto por fuera.
    pub fn put(&self, path: impl Into<PathBuf>, bytes: &[u8]) {
        crate::lock(&self.files).insert(path.into(), bytes.to_vec());
    }

    /// Quita ese fichero de ahí, como si alguien lo hubiera borrado por fuera.
    pub fn take_away(&self, path: &Path) {
        crate::lock(&self.files).remove(path);
    }

    /// Con ese fichero puesto ahí pero negado a abrirse.
    pub fn with_unreadable(self, path: impl Into<PathBuf>) -> Self {
        crate::lock(&self.unreadable).push(path.into());
        self
    }

    /// Lo que quedó escrito en esa ruta.
    pub fn written(&self, path: &Path) -> Option<Vec<u8>> {
        crate::lock(&self.files).get(path).cloned()
    }

    /// Cuántos ficheros hay en esa carpeta.
    pub fn count_in(&self, folder: &Path) -> usize {
        self.files_within(folder).len()
    }

    fn is_there(&self, path: &Path) -> bool {
        let path = self.followed(path);
        crate::lock(&self.files).contains_key(&path) || crate::lock(&self.folders).contains(&path)
    }

    fn followed(&self, path: &Path) -> PathBuf {
        crate::lock(&self.links)
            .get(path)
            .cloned()
            .unwrap_or_else(|| path.to_path_buf())
    }
}

impl DocumentFiles for InMemoryFiles {
    fn folder_fact(&self, path: &Path) -> FolderFact {
        let path = self.followed(path);
        if crate::lock(&self.folders).contains(&path) {
            return FolderFact::Folder;
        }
        if crate::lock(&self.files).contains_key(&path) {
            return FolderFact::NotAFolder;
        }
        FolderFact::Missing
    }

    fn exists(&self, path: &Path) -> bool {
        self.is_there(path)
    }

    fn readable(&self, path: &Path) -> Result<(), String> {
        let path = self.followed(path);
        if crate::lock(&self.unreadable).contains(&path) {
            return Err("permiso denegado".to_owned());
        }
        if crate::lock(&self.files).contains_key(&path) {
            return Ok(());
        }
        Err("no such file or directory".to_owned())
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.readable(path)?;
        Ok(crate::lock(&self.files)
            .get(&self.followed(path))
            .cloned()
            .unwrap_or_default())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let Some(folder) = path.parent() else {
            return Err("sin carpeta donde escribir".to_owned());
        };
        if !crate::lock(&self.folders)
            .iter()
            .any(|known| known == folder)
        {
            return Err("no such file or directory".to_owned());
        }
        crate::lock(&self.files).insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn modified_seconds(&self, path: &Path) -> Option<u64> {
        self.is_there(path).then_some(MODIFIED)
    }

    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        self.is_there(path).then(|| self.followed(path))
    }

    fn files_within(&self, folder: &Path) -> Vec<PathBuf> {
        crate::lock(&self.files)
            .keys()
            .filter(|path| path.parent() == Some(folder))
            .cloned()
            .collect()
    }
}
