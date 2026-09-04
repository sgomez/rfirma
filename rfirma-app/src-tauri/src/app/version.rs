//! **Si hay una versión nueva publicada**: el caso de uso que pregunta, y las
//! tres decisiones que el cliente HTTP no toma (ID-177, ID-178, ID-180,
//! ID-182).
//!
//! Las tres son: **qué versión anuncia** el cuerpo que devuelve el puerto,
//! **si esa versión es más nueva** que la que se está ejecutando, y **cada
//! cuánto se vuelve a preguntar**. El puerto —[`ReleaseFeed`]— solo trae una
//! cadena o nada, así que se dobla con un cierre y ninguna prueba de aquí abre
//! un socket (TD-39).
//!
//! # Es un aviso, no una actualización
//!
//! Lo que sale de aquí es un número para pintar una franja (ID-181). No hay
//! artefacto que descargar, ni firma que verificar, ni nada que instalar
//! (ID-177): los tres paquetes de la v0.4 los actualiza quien los instaló.
//!
//! # Sin red, silencio
//!
//! Que el puerto devuelva `None` no es un fallo que enseñar: es que no se ha
//! podido preguntar. No se avisa de nada y **no se toca la caché**, así que en
//! el siguiente arranque se vuelve a intentar.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::memory::{Memory, VersionCheck};

/// **El puerto de red**: trae el cuerpo de la última publicación, o nada.
///
/// Un cierre y no un rasgo, como el entorno que lee [`crate::paths`]: lo que
/// se necesita de fuera es una función sin argumentos, y un rasgo con un solo
/// método sería la misma función con una ceremonia alrededor. En producción lo
/// cumple [`crate::releases::latest_release`]; en las pruebas, un cierre que
/// devuelve una cadena escrita a mano.
pub type ReleaseFeed<'a> = &'a dyn Fn() -> Option<String>;

/// Cada cuánto se vuelve a preguntar (ID-180).
pub const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Una versión de rFirma, para poder compararlas.
///
/// Solo `mayor.menor.parche`: una etiqueta con cualquier otra cosa detrás
/// —`0.4.0-rc.1`— **no se interpreta**. Las `-rc.N` no producen paquetes
/// nativos (ID-150) y `/releases/latest` de GitHub ya excluye las
/// *prereleases*, así que una que llegara hasta aquí sería un anuncio que
/// nadie puede instalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// La versión que se está ejecutando.
    ///
    /// Sale del `Cargo.toml`, que está en el candado del ID-150 contra
    /// `tauri.conf.json`: la fuente es una sola y una puerta del CI comprueba
    /// que los demás sitios no divergen.
    pub fn running() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION"))
            .expect("la version del paquete es mayor.menor.parche")
    }

    /// Una versión leída de una etiqueta, con o sin la `v` delante.
    pub fn parse(text: &str) -> Option<Self> {
        let mut numbers = text.trim().trim_start_matches('v').split('.');
        let mut next = || numbers.next()?.parse::<u64>().ok();
        let (major, minor, patch) = (next()?, next()?, next()?);
        if numbers.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// La versión publicada, si es más nueva que la que se está ejecutando.
///
/// `None` cubre las tres situaciones en las que no hay nada que enseñar: no la
/// hay, no se pudo preguntar, o lo que contestó GitHub no se entiende. Ninguna
/// de las tres es un error para el usuario, así que ninguna sube como tal.
///
/// `now` entra como argumento —y no lo lee esta función— porque es lo que hace
/// comprobable la caché de 24 h sin dormir en una prueba.
pub fn new_version(
    running: Version,
    memory: &Memory,
    feed: ReleaseFeed<'_>,
    now: SystemTime,
) -> Option<Version> {
    let announced = match fresh_answer(memory, now) {
        Some(cached) => cached,
        None => ask_and_remember(memory, feed, now)?,
    };

    (announced > running).then_some(announced)
}

/// Lo contestado la última vez, si aún vale.
///
/// Un apunte que no se entiende —una versión escrita a mano en el
/// `state.json`— se trata como si no estuviera: se vuelve a preguntar.
fn fresh_answer(memory: &Memory, now: SystemTime) -> Option<Version> {
    let check = memory
        .state()
        .ok()?
        .into_value()
        .version_check
        .filter(|check| {
            seconds_since_epoch(now).saturating_sub(check.checked_at) < CACHE_TTL.as_secs()
        })?;

    Version::parse(&check.announced)
}

/// Pregunta al puerto y anota la respuesta.
///
/// Solo se anota lo que se ha entendido: si GitHub contesta algo que no lleva
/// una etiqueta legible, la caché se queda como estaba y se reintenta en el
/// siguiente arranque en vez de callar durante 24 h por una respuesta rara.
fn ask_and_remember(memory: &Memory, feed: ReleaseFeed<'_>, now: SystemTime) -> Option<Version> {
    let announced = announced_version(&feed()?)?;

    // Que no se pueda escribir el apunte no cancela el aviso: lo único que se
    // pierde es la caché, y eso son conexiones de más, no un fallo.
    let _ = memory.remember_version_check(VersionCheck {
        checked_at: seconds_since_epoch(now),
        announced: announced.to_string(),
    });

    Some(announced)
}

/// La versión que anuncia el cuerpo que trajo el puerto.
///
/// El campo es `tag_name`, y esta es **la única línea del programa que lo
/// sabe**: el cliente HTTP no interpreta nada (ID-182).
fn announced_version(body: &str) -> Option<Version> {
    let release: serde_json::Value = serde_json::from_str(body).ok()?;
    Version::parse(release.get("tag_name")?.as_str()?)
}

/// El reloj, en segundos desde el epoch.
///
/// Un reloj por detrás del epoch no es un caso que atender: cuenta como «hace
/// muchísimo», que es exactamente lo que hace que se vuelva a preguntar.
fn seconds_since_epoch(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::{new_version, Version, CACHE_TTL};
    use crate::app::fixtures::a_memory;
    use crate::memory::VersionCheck;

    /// **Grada A**: un directorio temporal para el `state.json`, y la red
    /// doblada por el puerto (TD-39). Ninguna prueba de aquí abre un socket ni
    /// habla con GitHub.
    fn a_release(tag: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","name":"rFirma {tag}"}}"#)
    }

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    /// Primera conducta: **hay versión nueva**.
    #[test]
    fn a_newer_published_version_is_announced() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());

        let newer = new_version(
            Version::parse("0.3.1").expect("es una version"),
            &memory,
            &|| Some(a_release("v0.4.0")),
            at(1_756_000_000),
        );

        assert_eq!(
            newer.map(|version| version.to_string()),
            Some("0.4.0".into())
        );
    }

    /// Segunda conducta: **no la hay**. La misma versión no es una versión
    /// nueva, y una más vieja tampoco.
    #[test]
    fn the_same_version_or_an_older_one_is_not_announced() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let running = Version::parse("0.4.0").expect("es una version");

        assert_eq!(
            new_version(running, &memory, &|| Some(a_release("v0.4.0")), at(1_000)),
            None,
            "la que se esta ejecutando no es una version nueva"
        );
        assert_eq!(
            new_version(running, &memory, &|| Some(a_release("v0.3.9")), at(2_000)),
            None,
            "una publicacion mas vieja tampoco"
        );
    }

    /// Tercera conducta: **no hay red**, y entonces silencio total. Ni aviso,
    /// ni fallo, ni apunte en la caché que impida volver a intentarlo.
    #[test]
    fn without_network_there_is_silence_and_the_cache_is_left_untouched() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());

        let nothing = new_version(
            Version::parse("0.1.0").expect("es una version"),
            &memory,
            &|| None,
            at(1_756_000_000),
        );

        assert_eq!(nothing, None);
        assert_eq!(
            memory
                .state()
                .expect("deberia leerse")
                .into_value()
                .version_check,
            None,
            "sin respuesta no se anota nada: el siguiente arranque vuelve a preguntar"
        );
    }

    /// Cuarta conducta: **dentro de las 24 h no se pregunta**. El puerto ni se
    /// llama, y la respuesta sale de lo anotado.
    #[test]
    fn within_twenty_four_hours_the_port_is_not_asked_again() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        memory
            .remember_version_check(VersionCheck {
                checked_at: 1_756_000_000,
                announced: "0.4.0".to_string(),
            })
            .expect("deberia anotarse");

        let announced = new_version(
            Version::parse("0.3.0").expect("es una version"),
            &memory,
            &|| panic!("no se puede salir a la red antes de que caduque la cache"),
            at(1_756_000_000 + CACHE_TTL.as_secs() - 1),
        );

        assert_eq!(
            announced.map(|version| version.to_string()),
            Some("0.4.0".into())
        );
    }

    /// Pasadas las 24 h sí se vuelve a preguntar, y lo nuevo sustituye a lo
    /// anotado.
    #[test]
    fn after_twenty_four_hours_the_port_is_asked_again() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        memory
            .remember_version_check(VersionCheck {
                checked_at: 1_756_000_000,
                announced: "0.4.0".to_string(),
            })
            .expect("deberia anotarse");
        let later = at(1_756_000_000 + CACHE_TTL.as_secs());

        let announced = new_version(
            Version::parse("0.3.0").expect("es una version"),
            &memory,
            &|| Some(a_release("v0.5.0")),
            later,
        );

        assert_eq!(
            announced.map(|version| version.to_string()),
            Some("0.5.0".into())
        );
        assert_eq!(
            memory
                .state()
                .expect("deberia leerse")
                .into_value()
                .version_check,
            Some(VersionCheck {
                checked_at: 1_756_000_000 + CACHE_TTL.as_secs(),
                announced: "0.5.0".to_string(),
            }),
            "lo preguntado se anota para las proximas 24 h"
        );
    }

    /// Una *prerelease* que llegue hasta aquí no se anuncia: no produce
    /// paquetes nativos, así que no hay nada que instalar (ID-150).
    #[test]
    fn a_release_candidate_tag_is_not_a_version_to_announce() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());

        let announced = new_version(
            Version::parse("0.3.0").expect("es una version"),
            &memory,
            &|| Some(a_release("v0.4.0-rc.1")),
            at(1_756_000_000),
        );

        assert_eq!(announced, None);
    }

    /// Una respuesta que no se entiende es silencio, y **no** se anota: se
    /// vuelve a preguntar en el siguiente arranque.
    #[test]
    fn an_answer_that_is_not_a_release_is_silence_and_is_not_remembered() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());

        let announced = new_version(
            Version::parse("0.3.0").expect("es una version"),
            &memory,
            &|| Some("<html>502 Bad Gateway</html>".to_string()),
            at(1_756_000_000),
        );

        assert_eq!(announced, None);
        assert_eq!(
            memory
                .state()
                .expect("deberia leerse")
                .into_value()
                .version_check,
            None
        );
    }

    /// Las versiones se comparan por número y no como texto: `0.10.0` es más
    /// nueva que `0.9.9`, que leídas como cadenas salen al revés.
    #[test]
    fn versions_are_compared_as_numbers_and_not_as_text() {
        let older = Version::parse("0.9.9").expect("es una version");
        let newer = Version::parse("0.10.0").expect("es una version");

        assert!(newer > older);
        assert_eq!(Version::parse("v1.2.3"), Version::parse("1.2.3"));
        assert_eq!(Version::parse("1.2"), None);
        assert_eq!(Version::parse("1.2.3.4"), None);
    }

    /// La versión que se está ejecutando se lee del paquete, que es donde el
    /// candado del ID-150 la mantiene igual a `tauri.conf.json`.
    #[test]
    fn the_running_version_comes_from_the_package() {
        assert_eq!(
            Version::running().to_string(),
            env!("CARGO_PKG_VERSION"),
            "la version del paquete es la que se compara"
        );
    }
}
