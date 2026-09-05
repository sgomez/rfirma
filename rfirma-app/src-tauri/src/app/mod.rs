//! **Los casos de uso**: lo que la aplicación sabe hacer, sin Tauri delante.
//!
//! Hasta el #135 esto vivía dentro de [`crate::commands`] como veintitrés
//! funciones privadas entre los cuerpos de las órdenes, y por eso **ninguna de
//! las pruebas invocaba una orden**: una orden toma `State<'_, T>` y eso no se
//! construye fuera de un runtime de Tauri, así que todas entraban por la puerta
//! de atrás. Aquí las funciones son **públicas y esa es la interfaz por la que
//! se prueba** (ID-77, TD-20).
//!
//! # Cada caso de uso pide lo que usa
//!
//! Ninguna firma menciona `State` ni recibe el [`Environment`] entero (ID-78).
//! `Environment` sigue existiendo —es lo que Tauri gestiona y lo que `lib.rs`
//! construye—, pero es la **raíz de composición**: una orden lo desempaqueta y
//! pasa las referencias que ese caso de uso pide. Leer la firma de un caso de
//! uso dice qué toca y qué no.
//!
//! # La dirección es hacia el dominio
//!
//! [`crate::commands`] depende de este módulo; este módulo depende de los de
//! dominio e infraestructura —[`crate::pkcs11`], [`crate::signing`],
//! [`crate::memory`], [`crate::destination`], [`crate::isolate`]— y **ninguno
//! de esos depende de este ni de las órdenes** (ID-81). La única mirada hacia
//! arriba son los tipos serde que la ventana y los casos de uso comparten
//! ([`crate::commands::views`] y [`crate::commands::orders`]): el ID-80 fija
//! dónde viven y el ID-89 prohíbe cambiar lo que cruza, así que duplicarlos
//! aquí habría sido inventarse una segunda copia de la misma cosa.

pub mod certificates;
pub mod configuration;
pub mod cycle;
pub mod documents;
pub mod filtering;
pub mod frontier;
pub mod in_hand;
pub mod invocation;
pub mod preview;
pub mod recents;
pub mod rubric;
pub mod signing;
pub mod site;
pub mod version;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod guards;

use std::sync::Mutex;

use crate::destination::DestinationFolder;
use crate::memory::{Configuration, ListedCertificates, Memory};

/// Los almacenes de certificados, la carpeta de destino y lo que se recuerda.
///
/// Es la **raíz de composición**: lo que `lib.rs` arma al arrancar y lo que
/// Tauri gestiona. Ningún caso de uso lo recibe entero (ID-78); las órdenes lo
/// desempaquetan.
pub struct Environment {
    /// Dónde se buscan los certificados, en orden.
    ///
    /// Es una **colección** y no una ruta única (ID-03): un almacén que no
    /// cargue no puede dejar sin certificados a los demás. Los resuelve
    /// [`crate::pkcs11::stores::from_environment`] al arrancar.
    pub stores: Vec<crate::pkcs11::Store>,
    /// Los certificados del último listado, por su asa. Ver
    /// [`ListedCertificates`]: es donde se queda todo lo que la ventana no
    /// puede tener.
    pub listed: ListedCertificates,
    /// La carpeta de documentos del usuario, para cuando no haya destino
    /// elegido.
    pub documents_folder: std::path::PathBuf,
    /// Lo que se recuerda entre sesiones, ya leído.
    ///
    /// Es la copia viva: las órdenes de firma la consultan sin tocar el disco,
    /// y la orden que guarda ajustes la actualiza a la vez que la guarda. Tener
    /// solo el fichero obligaría a releerlo en cada firma; tener solo la copia
    /// perdería lo elegido al cerrar la ventana.
    pub configuration: Mutex<Configuration>,
    /// Los dos ficheros donde se recuerda. Ver [`crate::memory::Memory`].
    pub memory: Memory,
    /// El almacén de la rúbrica: se copia, no se referencia (ID-33).
    pub rubric: crate::rubric::RubricStore,
    /// Donde viven los almacenes NSS de los `.p12` instalados (ID-192). Lo
    /// resuelve [`crate::paths::Paths::installed_certificates_dir`] al
    /// arrancar.
    pub installed_certificates: std::path::PathBuf,
}

impl Environment {
    /// Una copia de la configuración viva, para pasársela a un caso de uso.
    ///
    /// Desempaquetar es lo único que una orden hace antes de llamar (ID-79), y
    /// esto es desempaquetar: el cerrojo no cruza a [`crate::app`] porque un
    /// caso de uso que lo tomara podría quedárselo mientras hace entrada y
    /// salida.
    ///
    /// Es una **instantánea**, y a propósito: una orden la toma al entrar y esa
    /// copia le vale para todo el recorrido. Antes el cerrojo se cogía en cada
    /// punto de uso, así que unas Preferencias guardadas a mitad de una
    /// postfirma —o con el diálogo de abrir abierto— podían cambiarle el rumbo
    /// por la mitad. Un recorrido que empieza con unos ajustes termina con esos
    /// mismos.
    pub fn configuration(&self) -> Configuration {
        lock(&self.configuration).clone()
    }

    /// Los almacenes de **ahora mismo**: los que se resolvieron al arrancar más
    /// los `.p12` instalados, que se leen del disco en cada llamada.
    ///
    /// Los instalados no pueden vivir en [`Self::stores`] porque cambian con la
    /// sesión abierta: instalar uno y quitarlo son dos gestos de Preferencias, y
    /// una lista fijada al arrancar dejaría el recién instalado sin listar hasta
    /// el siguiente arranque (ID-192).
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

/// La carpeta donde cae lo firmado: la que el usuario eligió una vez, o la de
/// documentos.
///
/// Vive aquí y no en [`crate::destination`] porque desenvolver la configuración
/// es una decisión, y las decisiones las toma quien ya tiene la configuración
/// delante (ID-83). El módulo del destino solo sabe de carpetas.
///
/// `documents_folder` se pasa desde fuera —lo resuelve
/// [`crate::paths::documents_folder`]— porque saber dónde tiene sus documentos
/// un usuario es conocimiento de sistema operativo, y el ID-35 dice que ese
/// conocimiento vive en un solo fichero.
pub fn chosen_folder(
    configuration: &Configuration,
    documents_folder: impl Into<std::path::PathBuf>,
) -> DestinationFolder {
    configuration
        .destination
        .clone()
        .unwrap_or_else(|| DestinationFolder::at(documents_folder))
}

/// El cerrojo, sin envenenarse.
///
/// Un `Mutex` envenenado es un hilo que se cayó teniéndolo cogido; lo que hay
/// dentro sigue siendo la configuración que se leyó del disco, y tumbar la
/// firma por eso no arregla nada.
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

    /// Lo que una orden desempaqueta es una **copia**, no el cerrojo: un caso de
    /// uso que se quedara el `MutexGuard` lo tendría cogido mientras hace
    /// entrada y salida (ID-78).
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
