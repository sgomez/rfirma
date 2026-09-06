//! Casos de uso de la aplicación y composición del entorno de ejecución.

pub mod certificates;
pub mod codec;
pub mod configuration;
pub mod cycle;
pub mod documents;
pub mod engines;
pub mod errand;
pub mod filtering;
pub mod frontier;
pub mod handlers;
pub mod in_hand;
pub mod invocation;
pub mod policies;
pub mod preview;
pub mod recents;
pub mod rubric;
pub mod signing;
pub mod site;
pub mod startup;
pub mod transport;
pub mod trust;
pub mod version;

#[cfg(test)]
pub(crate) mod fixtures;

use std::sync::Mutex;

use crate::destination::DestinationFolder;
use crate::memory::{Configuration, ListedCertificates, Memory};

/// Entorno de composición que agrupa almacenes, configuración y persistencia.
pub struct Environment {
    /// Almacenes de certificados configurados.
    pub stores: Vec<crate::pkcs11::Store>,
    /// Certificados del último listado.
    pub listed: ListedCertificates,
    /// Carpeta de documentos del usuario por omisión.
    pub documents_folder: std::path::PathBuf,
    /// Configuración en memoria viva compartida.
    pub configuration: Mutex<Configuration>,
    /// Acceso a la persistencia en disco (ADR-0010).
    pub memory: Memory,
    /// Almacén de la rúbrica (ADR-0012).
    pub rubric: crate::rubric::RubricStore,
    /// Directorio de certificados de software instalados.
    pub installed_certificates: std::path::PathBuf,
}

impl Environment {
    /// Devuelve una instantánea de la configuración viva.
    pub fn configuration(&self) -> Configuration {
        lock(&self.configuration).clone()
    }

    /// Devuelve todos los almacenes disponibles incluyendo certificados instalados.
    pub fn all_stores(&self) -> Vec<crate::pkcs11::Store> {
        let mut stores = self.stores.clone();
        if let Some(softoken) = crate::pkcs11::stores::softoken() {
            stores.extend(crate::pkcs11::stores::installed_stores(
                &softoken,
                &self.installed_certificates,
            ));
        }
        stores
    }
}

/// Resuelve la carpeta destino elegida o la carpeta de documentos por omisión.
pub fn chosen_folder(
    configuration: &Configuration,
    documents_folder: impl Into<std::path::PathBuf>,
) -> DestinationFolder {
    configuration
        .destination
        .clone()
        .unwrap_or_else(|| DestinationFolder::at(documents_folder))
}

/// Adquiere el cerrojo recuperando el valor si el mutex estaba envenenado.
pub fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;

    use super::{chosen_folder, Environment};
    use crate::app::fixtures::a_memory;
    use crate::destination::DestinationFolder;
    use crate::memory::{Configuration, ListedCertificates};

    #[test]
    fn the_environment_hands_out_a_copy_of_the_live_configuration() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let environment = Environment {
            stores: Vec::new(),
            listed: ListedCertificates::new(),
            documents_folder: home.path().to_path_buf(),
            configuration: Mutex::new(Configuration {
                remember_activity: false,
                ..Configuration::default()
            }),
            memory: a_memory(home.path()),
            rubric: crate::rubric::RubricStore::at(home.path().join("rubric.jpg")),
            installed_certificates: home.path().join("certificates"),
        };

        let copy = environment.configuration();

        assert!(!copy.remember_activity);
        assert!(
            environment.configuration().remember_activity == copy.remember_activity,
            "la copia dice lo mismo que la viva"
        );
    }

    #[test]
    fn the_remembered_folder_is_reused_and_nobody_is_asked_again() {
        let configuration = Configuration {
            destination: Some(DestinationFolder::at("/home/quien/Documentos/Firmados")),
            ..Configuration::default()
        };

        let folder = chosen_folder(&configuration, "/home/quien/Documentos");

        assert_eq!(
            folder.path(),
            Path::new("/home/quien/Documentos/Firmados"),
            "elegida una vez, se reutiliza"
        );
        assert_eq!(folder.name(), "Firmados");
    }

    #[test]
    fn without_a_remembered_folder_the_destination_is_the_documents_folder() {
        let folder = chosen_folder(&Configuration::default(), "/home/quien/Documentos");

        assert_eq!(folder.path(), Path::new("/home/quien/Documentos"));
    }
}
