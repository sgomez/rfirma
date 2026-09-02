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
pub mod documents;
pub mod signing;

#[cfg(test)]
pub(crate) mod fixtures;

use std::sync::Mutex;

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
}

impl Environment {
    /// Una copia de la configuración viva, para pasársela a un caso de uso.
    ///
    /// Desempaquetar es lo único que una orden hace antes de llamar (ID-79), y
    /// esto es desempaquetar: el cerrojo no cruza a [`crate::app`] porque un
    /// caso de uso que lo tomara podría quedárselo mientras hace entrada y
    /// salida.
    pub fn configuration(&self) -> Configuration {
        lock(&self.configuration).clone()
    }
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
    use std::sync::Mutex;

    use super::Environment;
    use crate::app::fixtures::a_memory;
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
        };

        let copy = environment.configuration();

        assert!(!copy.remember_activity);
        assert!(
            environment.configuration().remember_activity == copy.remember_activity,
            "la copia dice lo mismo que la viva"
        );
    }
}
