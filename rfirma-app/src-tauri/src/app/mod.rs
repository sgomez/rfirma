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
mod tests;
