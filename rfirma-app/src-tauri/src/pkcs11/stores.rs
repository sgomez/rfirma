//! **Dónde** se buscan los certificados: la colección de almacenes.
//!
//! Hasta el #97 esto era una constante, `/app/lib/pkcs11/opensc-pkcs11.so`, y
//! una ruta única. Esa ruta **solo existe dentro del flatpak**, así que con
//! `just dev` la carga fallaba siempre y la ventana se quedaba sin
//! certificados. Ahora el binario resuelve los almacenes que de verdad hay
//! debajo —el del sandbox cuando corre dentro, y los del anfitrión cuando corre
//! fuera— sin que nadie tenga que exportar nada a mano.
//!
//! Es una **colección** y no una ruta a propósito (ID-03): un almacén que no
//! cargue no puede dejar sin certificados a los demás.
//!
//! Desde el #99 hay dos clases de almacén y **una sola** implementación
//! (ID-01): el módulo PKCS#11 de una tarjeta, y el almacén NSS de Mozilla —el
//! perfil de Firefox y `~/.pki/nssdb`—, que entra por `libsoftokn3.so` como un
//! módulo PKCS#11 más. Lo único que los distingue es que el segundo necesita
//! que se le diga **qué perfil** abrir: eso son los init args de [`Store`].

use std::path::{Path, PathBuf};

/// Los módulos que se buscan cuando nadie dice otra cosa, en orden.
///
/// Se declaran por ruta absoluta y no se adivinan con `dlopen` a secas: cargar
/// «el primer `opensc-pkcs11.so` del `LD_LIBRARY_PATH`» es dejar que el entorno
/// decida con qué se firma.
pub const CANDIDATE_MODULES: &[&str] = &[
    // El que empaqueta el propio flatpak: los del anfitrión no cargan dentro
    // del sandbox (`docs/research/flatpak-canal-unico.md`).
    "/app/lib/pkcs11/opensc-pkcs11.so",
    // Los del anfitrión, que es lo que hay debajo de `just dev`. OpenSC cubre
    // el DNIe y las tarjetas corrientes; SoftHSM es el token de pruebas.
    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so",
    "/usr/lib/x86_64-linux-gnu/pkcs11/opensc-pkcs11.so",
    "/usr/lib64/opensc-pkcs11.so",
    "/usr/lib64/pkcs11/opensc-pkcs11.so",
    "/usr/lib/opensc-pkcs11.so",
    "/usr/lib/pkcs11/opensc-pkcs11.so",
    "/usr/lib/softhsm/libsofthsm2.so",
    "/usr/lib/x86_64-linux-gnu/softhsm/libsofthsm2.so",
];

/// Dónde puede estar el softoken de NSS, que es lo que abre un perfil de
/// Firefox.
///
/// No se empaqueta (ID-15): el runtime `org.gnome.Platform//50` ya trae el
/// primero de esta lista, así que dentro del sandbox y fuera se busca igual.
pub const CANDIDATE_SOFTOKENS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libsoftokn3.so",
    "/usr/lib/x86_64-linux-gnu/nss/libsoftokn3.so",
    "/usr/lib64/libsoftokn3.so",
    "/usr/lib64/nss/libsoftokn3.so",
    "/usr/lib/libsoftokn3.so",
    "/usr/lib/nss/libsoftokn3.so",
];

/// **Qué clase de almacén** es, para poder decirlo en la ventana.
///
/// Cruza la frontera como clase en inglés y **nunca como ruta** (ADR-0011): el
/// rótulo —«Firefox», «Chrome», «Tarjeta»— lo pone el catálogo de la ventana,
/// igual que hace con la `situation` de un fallo. Componer aquí el nombre ya
/// escrito se saltaría los catálogos y sacaría castellano en la versión en
/// inglés.
///
/// Hace falta porque el mismo certificado en el perfil de Firefox y en
/// `~/.pki/nssdb` es indistinguible sin él, y quien tiene tres iguales no puede
/// elegir a ciegas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoreClass {
    /// Un módulo PKCS#11 corriente: el DNIe, una tarjeta, SoftHSM.
    Card,
    /// Un perfil de Firefox, servido por `libsoftokn3.so`.
    Firefox,
    /// `~/.pki/nssdb`, que es donde guardan los suyos Chrome y las
    /// herramientas de NSS.
    Chrome,
    /// Otro almacén NSS: un perfil fuera de sitio, o el que se apunte a mano.
    /// No se disfraza de Firefox ni de Chrome, porque no se sabe de quién es.
    Nssdb,
}

/// Un almacén de certificados: el módulo que lo sirve y, si hace falta, **qué**
/// tiene que abrir.
///
/// Para una tarjeta los init args son `None` y esto es exactamente lo que era
/// antes, una ruta. Para NSS son la cadena que le dice a softoken qué perfil
/// abrir y en qué modo, y sin ellos **no falla**: abre un almacén vacío que se
/// anuncia como `token initialized` y devuelve cero objetos, que es el fallo
/// silencioso que este módulo existe para evitar.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Store {
    module: PathBuf,
    init_args: Option<String>,
}

impl Store {
    /// Un módulo PKCS#11 corriente, sin nada que configurar.
    pub fn module(module: impl Into<PathBuf>) -> Self {
        Self {
            module: module.into(),
            init_args: None,
        }
    }

    /// Un almacén con init args ya escritos. Lo usa [`CertificateRef`] para
    /// reconstruir el almacén de una referencia recordada.
    ///
    /// [`CertificateRef`]: super::CertificateRef
    pub fn with_init_args(module: impl Into<PathBuf>, init_args: Option<String>) -> Self {
        Self {
            module: module.into(),
            init_args,
        }
    }

    /// El almacén NSS de un perfil concreto (ID-02).
    ///
    /// `flags=readOnly` **no es opcional**: rfirma no puede escribir en el
    /// perfil de Firefox de nadie. `sql:` tampoco: es el formato de `cert9.db`,
    /// que es el que llevan los perfiles desde hace años, y sin el prefijo
    /// softoken buscaría los `cert8.db` de Berkeley DB que ya no existen.
    pub fn nss(softoken: impl Into<PathBuf>, profile: &Path) -> Self {
        Self {
            module: softoken.into(),
            init_args: Some(format!(
                "configdir='sql:{}' certPrefix='' keyPrefix='' secmod='secmod.db' flags=readOnly",
                profile.display()
            )),
        }
    }

    /// Ruta del módulo PKCS#11 que lo sirve.
    pub fn path(&self) -> &Path {
        &self.module
    }

    /// Lo que hay que pasarle a `C_Initialize` para que abra **este** almacén,
    /// o `None` cuando el módulo no necesita que se le diga nada.
    pub fn init_args(&self) -> Option<&str> {
        self.init_args.as_deref()
    }

    /// De qué clase es, que es lo único de un almacén que puede cruzar a la
    /// ventana.
    ///
    /// Sin init args es una tarjeta: eso es lo que era un almacén antes del
    /// #99. Con ellos es NSS, y **de quién** es el perfil se decide por su
    /// `configdir`; un perfil que no esté en ninguna de las rutas conocidas de
    /// Firefox (legado o XDG, ID-199) ni de Chrome se queda en
    /// [`StoreClass::Nssdb`] en vez de que se le atribuya un dueño que no se
    /// conoce.
    pub fn class(&self) -> StoreClass {
        let Some(profile) = self.profile() else {
            return StoreClass::Card;
        };
        if profile.contains("/.mozilla/firefox/") || profile.contains("/mozilla/firefox/") {
            StoreClass::Firefox
        } else if profile.ends_with("/.pki/nssdb") || profile.ends_with("/pki/nssdb") {
            StoreClass::Chrome
        } else {
            StoreClass::Nssdb
        }
    }

    /// El `configdir` de los init args, sin el prefijo `sql:` y sin las
    /// comillas. **No sale de este módulo**: es una ruta del anfitrión, y solo
    /// se lee para clasificar.
    fn profile(&self) -> Option<&str> {
        let args = self.init_args.as_deref()?;
        let after = args.split_once("configdir='")?.1;
        let inside = after.split_once('\'')?.0;
        Some(inside.strip_prefix("sql:").unwrap_or(inside))
    }
}

impl From<&Path> for Store {
    fn from(module: &Path) -> Self {
        Self::module(module)
    }
}

impl From<PathBuf> for Store {
    fn from(module: PathBuf) -> Self {
        Self::module(module)
    }
}

impl From<&PathBuf> for Store {
    fn from(module: &PathBuf) -> Self {
        Self::module(module.clone())
    }
}

impl From<&str> for Store {
    fn from(module: &str) -> Self {
        Self::module(module)
    }
}

impl From<&Store> for Store {
    fn from(store: &Store) -> Self {
        store.clone()
    }
}

/// Los almacenes de esta máquina, resueltos al arrancar.
///
/// `RFIRMA_PKCS11_MODULE` sigue siendo la escotilla para apuntar a otro módulo
/// —de ella dependen las pruebas de grada B contra SoftHSM— y cuando está
/// puesta **manda ella sola**: quien la exporta quiere ese módulo y no el que
/// nosotros hubiéramos elegido, ni el perfil de Firefox que tenga delante.
pub fn from_environment() -> Vec<Store> {
    if let Some(module) = std::env::var_os(crate::PKCS11_MODULE_VARIABLE) {
        return vec![Store::module(module)];
    }

    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut stores: Vec<Store> = present_among(CANDIDATE_MODULES, |path| path.is_file())
        .into_iter()
        .map(Store::module)
        .collect();

    // El softoken se busca una vez y sirve para todos los perfiles: lo que
    // cambia de un perfil a otro son los init args, no el módulo.
    if let (Some(home), Some(softoken)) = (
        home,
        present_among(CANDIDATE_SOFTOKENS, |path| path.is_file())
            .into_iter()
            .next(),
    ) {
        stores.extend(
            nss_profiles(&home)
                .into_iter()
                .map(|profile| Store::nss(&softoken, &profile)),
        );
    }

    stores
}

/// Los pares (directorio de configuración, directorio de datos) de Firefox
/// que se buscan, en orden.
///
/// Hasta Firefox 146 los dos eran el mismo directorio, `~/.mozilla/firefox`.
/// Firefox 147 se mudó a XDG: `profiles.ini` pasa a
/// `~/.config/mozilla/firefox` y los perfiles —donde vive `cert9.db`— a
/// `~/.local/share/mozilla/firefox`. Bajo XDG el `Path=` relativo de
/// `profiles.ini` resuelve contra el directorio de **datos**, así que los
/// dos entran **emparejados** (ID-199): declarar sólo el de configuración da
/// cero certificados otra vez y sin error.
fn firefox_layouts(home: &Path) -> [(PathBuf, PathBuf); 2] {
    [
        (home.join(".mozilla/firefox"), home.join(".mozilla/firefox")),
        (
            home.join(".config/mozilla/firefox"),
            home.join(".local/share/mozilla/firefox"),
        ),
    ]
}

/// Los perfiles NSS que hay bajo un `HOME`, en orden (ID-05).
///
/// Los de Firefox salen de `profiles.ini` y **no** de adivinar el nombre del
/// directorio: el sufijo aleatorio de `xxxxxxxx.default-release` no se puede
/// deducir, y quien tenga varios perfiles no cabe en «el primero que haya».
/// Se buscan en las dos disposiciones de [`firefox_layouts`], la antigua y la
/// XDG, porque las dos conviven según la versión instalada. Después van
/// `~/.pki/nssdb` y `~/.local/share/pki/nssdb`, que es donde guardan los
/// suyos Chrome y las herramientas de NSS: la familia Chromium entera, porque
/// ninguna de las dos rutas es «el almacén de Chrome» sino la base NSS
/// compartida del usuario (ID-199).
///
/// Un perfil **sin `cert9.db` se salta**: es un perfil sin base de datos de
/// certificados, y abrirlo en solo lectura no crearía ninguna.
pub fn nss_profiles(home: &Path) -> Vec<PathBuf> {
    let mut profiles: Vec<PathBuf> = Vec::new();
    for (config, data) in firefox_layouts(home) {
        profiles.extend(
            profiles_declared_in(&config.join("profiles.ini"))
                .into_iter()
                .map(|relative_or_absolute| resolve_under(&data, &relative_or_absolute)),
        );
    }
    profiles.push(home.join(".pki/nssdb"));
    profiles.push(home.join(".local/share/pki/nssdb"));

    let mut found: Vec<PathBuf> = Vec::new();
    for profile in profiles {
        if !profile.join("cert9.db").is_file() {
            continue;
        }
        let resolved = profile.canonicalize().unwrap_or_else(|_| profile.clone());
        if !found
            .iter()
            .any(|already| already.canonicalize().unwrap_or_else(|_| already.clone()) == resolved)
        {
            found.push(profile);
        }
    }

    found
}

/// Las rutas que declara un `profiles.ini`, tal cual vienen.
///
/// Se leen las secciones `[ProfileN]` y solo su clave `Path`. Las secciones
/// `[Install…]` se ignoran a propósito: su `Default=` apunta a un perfil que ya
/// está declarado como `[ProfileN]`, y contarlo dos veces enseñaría cada
/// certificado por duplicado.
///
/// El formato no da para un analizador de INI de verdad: son secciones entre
/// corchetes y `clave=valor` sin comillas ni continuaciones.
fn profiles_declared_in(ini: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(ini) else {
        // No tener Firefox instalado no es un fallo: es no tener perfiles
        // (ID-03). Quien firma con tarjeta sigue firmando.
        return Vec::new();
    };

    let mut paths = Vec::new();
    let mut inside_a_profile = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside_a_profile = section.starts_with("Profile");
            continue;
        }
        if !inside_a_profile {
            continue;
        }
        if let Some(value) = line.strip_prefix("Path=") {
            let value = value.trim();
            if !value.is_empty() {
                paths.push(value.to_owned());
            }
        }
    }

    paths
}

/// Una ruta de `profiles.ini` resuelta contra el directorio de Firefox.
///
/// `IsRelative` no se lee: una ruta absoluta se reconoce por empezar con `/`, y
/// creer a la clave por encima de la ruta convertiría un `profiles.ini` mal
/// escrito en un directorio inexistente en vez de en el perfil que hay.
fn resolve_under(firefox: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        firefox.join(path)
    }
}

/// Los candidatos que existen, sin repetir el mismo fichero dos veces.
///
/// La deduplicación no es cosmética: la mayoría de distribuciones instalan
/// `opensc-pkcs11.so` en un sitio y lo enlazan desde otro, y listar el mismo
/// módulo dos veces enseñaría **cada certificado por duplicado** en el panel.
/// Se compara por la ruta ya resuelta, que es lo que distingue dos ficheros de
/// dos nombres del mismo.
pub fn present_among(candidates: &[&str], present: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut stores: Vec<PathBuf> = Vec::new();

    for candidate in candidates {
        let path = Path::new(candidate);
        if !present(path) {
            continue;
        }
        // Un candidato que no se puede resolver se queda con su ruta tal cual:
        // que `canonicalize` falle no es motivo para descartar un módulo que el
        // predicado ya dio por presente.
        let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let already = stores
            .iter()
            .any(|store| store.canonicalize().unwrap_or_else(|_| store.clone()) == resolved);
        if !already {
            stores.push(path.to_path_buf());
        }
    }

    stores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_candidates_that_are_there() {
        let stores = present_among(&["/hay/uno.so", "/no/hay.so", "/hay/otro.so"], |path| {
            path.starts_with("/hay")
        });

        assert_eq!(
            stores,
            vec![PathBuf::from("/hay/uno.so"), PathBuf::from("/hay/otro.so")]
        );
    }

    #[test]
    fn has_no_stores_when_no_candidate_is_installed() {
        assert!(present_among(CANDIDATE_MODULES, |_| false).is_empty());
    }

    /// Dos nombres del mismo fichero son **un** almacén: listarlo dos veces
    /// enseñaría cada certificado por duplicado.
    #[test]
    fn lists_the_same_module_once_even_under_two_names() {
        let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
        let module = directory.path().join("modulo.so");
        std::fs::write(&module, b"").expect("deberia poder escribirse el modulo");
        let link = directory.path().join("enlace.so");
        std::os::unix::fs::symlink(&module, &link).expect("deberia poder enlazarse");

        let candidates = [
            module.to_str().expect("ruta valida"),
            link.to_str().expect("ruta valida"),
        ];
        let stores = present_among(&candidates, |path| path.is_file());

        assert_eq!(stores, vec![module]);
    }

    /// Un módulo de tarjeta no lleva init args: es lo que era antes del #99.
    #[test]
    fn a_plain_module_has_nothing_to_configure() {
        assert_eq!(Store::module("/usr/lib/x.so").init_args(), None);
    }

    /// Sin init args es una tarjeta, y esa es la clase que cruza a la ventana.
    #[test]
    fn a_plain_module_is_a_card() {
        assert_eq!(Store::module("/usr/lib/x.so").class(), StoreClass::Card);
    }

    /// Un perfil de `~/.mozilla/firefox` es de Firefox, y el de `~/.pki/nssdb`
    /// es de Chrome: son los dos que hay que poder distinguir en la lista.
    #[test]
    fn an_nss_store_is_classified_by_whose_profile_it_opens() {
        let firefox = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/casa/ada/.mozilla/firefox/aaaaaaaa.default-release"),
        );
        let chrome = Store::nss("/usr/lib/libsoftokn3.so", Path::new("/casa/ada/.pki/nssdb"));

        assert_eq!(firefox.class(), StoreClass::Firefox);
        assert_eq!(chrome.class(), StoreClass::Chrome);
    }

    /// Las mismas dos clases, ahora bajo las rutas XDG que estrenaron Firefox
    /// 147 y Chrome M146: la clasificación no se puede quedar atada a las
    /// rutas viejas mientras `nss_profiles` ya lee las nuevas.
    #[test]
    fn an_nss_store_is_classified_the_same_under_the_xdg_paths() {
        let firefox = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/casa/ada/.local/share/mozilla/firefox/cccccccc.default-release"),
        );
        let chrome = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/casa/ada/.local/share/pki/nssdb"),
        );

        assert_eq!(firefox.class(), StoreClass::Firefox);
        assert_eq!(chrome.class(), StoreClass::Chrome);
    }

    /// Un perfil que no esté en ninguno de los dos sitios se queda en «NSS» a
    /// secas: atribuirle un dueño sería inventárselo.
    #[test]
    fn an_nss_store_somewhere_else_claims_no_owner() {
        let store = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/tmp/perfil-de-pruebas"),
        );

        assert_eq!(store.class(), StoreClass::Nssdb);
    }

    /// Las dos mitades que no son negociables (ID-02): el formato `sql:` y el
    /// solo lectura.
    #[test]
    fn an_nss_store_opens_the_profile_read_only_and_in_sql_format() {
        let store = Store::nss("/usr/lib/libsoftokn3.so", Path::new("/casa/perfil"));
        let args = store.init_args().expect("un almacen NSS lleva init args");

        assert!(args.contains("configdir='sql:/casa/perfil'"), "{args}");
        assert!(args.contains("flags=readOnly"), "{args}");
    }

    fn a_home_with(profiles: &[(&str, bool)], ini: Option<&str>) -> tempfile::TempDir {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
        let firefox = home.path().join(".mozilla/firefox");
        std::fs::create_dir_all(&firefox).expect("deberia poder crearse .mozilla/firefox");
        for (name, with_database) in profiles {
            let directory = home.path().join(name);
            std::fs::create_dir_all(&directory).expect("deberia poder crearse el perfil");
            if *with_database {
                std::fs::write(directory.join("cert9.db"), b"").expect("deberia poder escribirse");
            }
        }
        if let Some(ini) = ini {
            std::fs::write(firefox.join("profiles.ini"), ini).expect("deberia poder escribirse");
        }
        home
    }

    /// El nombre del directorio de un perfil lleva un sufijo aleatorio: hay que
    /// leerlo del `profiles.ini`, y hay que leerlos **todos**.
    #[test]
    fn reads_every_firefox_profile_declared_in_profiles_ini() {
        let home = a_home_with(
            &[
                (".mozilla/firefox/aaaaaaaa.default-release", true),
                (".mozilla/firefox/bbbbbbbb.trabajo", true),
            ],
            Some(
                "[Install4F96D1932A9F858E]\n\
                 Default=aaaaaaaa.default-release\n\
                 \n\
                 [Profile0]\n\
                 Name=default-release\n\
                 IsRelative=1\n\
                 Path=aaaaaaaa.default-release\n\
                 \n\
                 [Profile1]\n\
                 Name=trabajo\n\
                 IsRelative=1\n\
                 Path=bbbbbbbb.trabajo\n",
            ),
        );

        assert_eq!(
            nss_profiles(home.path()),
            vec![
                home.path()
                    .join(".mozilla/firefox/aaaaaaaa.default-release"),
                home.path().join(".mozilla/firefox/bbbbbbbb.trabajo"),
            ]
        );
    }

    /// Un perfil declarado que no tiene base de datos de certificados no es un
    /// fallo: es un perfil sin certificados, y se salta.
    #[test]
    fn skips_a_declared_profile_without_a_certificate_database() {
        let home = a_home_with(
            &[
                (".mozilla/firefox/aaaaaaaa.vacio", false),
                (".mozilla/firefox/bbbbbbbb.lleno", true),
            ],
            Some("[Profile0]\nPath=aaaaaaaa.vacio\n\n[Profile1]\nPath=bbbbbbbb.lleno\n"),
        );

        assert_eq!(
            nss_profiles(home.path()),
            vec![home.path().join(".mozilla/firefox/bbbbbbbb.lleno")]
        );
    }

    /// El almacén de Chrome y de las herramientas de NSS entra gratis.
    #[test]
    fn reads_the_shared_nssdb_too() {
        let home = a_home_with(&[(".pki/nssdb", true)], None);

        assert_eq!(
            nss_profiles(home.path()),
            vec![home.path().join(".pki/nssdb")]
        );
    }

    /// Sin Firefox instalado no hay perfiles, y eso **no** es un fallo: quien
    /// firma con tarjeta sigue firmando.
    #[test]
    fn has_no_profiles_when_firefox_is_not_installed() {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");

        assert!(nss_profiles(home.path()).is_empty());
    }

    /// Firefox 147 bajo XDG: `profiles.ini` en `~/.config/mozilla/firefox`,
    /// perfil y `cert9.db` en `~/.local/share/mozilla/firefox`. Si el código
    /// resolviera el `Path=` relativo contra el directorio de configuración
    /// en vez del de datos —o sólo mirara uno de los dos—, este perfil no
    /// aparecería: es el fallo silencioso que el ID-199 evita.
    #[test]
    fn reads_a_firefox_profile_from_the_paired_xdg_config_and_data_dirs() {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
        let config = home.path().join(".config/mozilla/firefox");
        let data = home.path().join(".local/share/mozilla/firefox");
        let profile = data.join("cccccccc.default-release");
        std::fs::create_dir_all(&config).expect("deberia poder crearse .config/mozilla/firefox");
        std::fs::create_dir_all(&profile).expect("deberia poder crearse el perfil");
        std::fs::write(profile.join("cert9.db"), b"").expect("deberia poder escribirse");
        std::fs::write(
            config.join("profiles.ini"),
            "[Profile0]\nPath=cccccccc.default-release\n",
        )
        .expect("deberia poder escribirse");

        assert_eq!(nss_profiles(home.path()), vec![profile]);
    }

    /// La base NSS compartida de la familia Chromium bajo su ruta XDG
    /// (`~/.local/share/pki/nssdb`) entra igual que la antigua
    /// `~/.pki/nssdb`, y las dos pueden convivir.
    #[test]
    fn reads_the_xdg_shared_nssdb_too() {
        let home = a_home_with(&[(".local/share/pki/nssdb", true)], None);

        assert_eq!(
            nss_profiles(home.path()),
            vec![home.path().join(".local/share/pki/nssdb")]
        );
    }

    /// Un `profiles.ini` puede declarar una ruta absoluta; el resto son
    /// relativas a `~/.mozilla/firefox`.
    #[test]
    fn resolves_an_absolute_profile_path_as_it_comes() {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
        let firefox = home.path().join(".mozilla/firefox");
        let elsewhere = home.path().join("otro-sitio");
        std::fs::create_dir_all(&firefox).expect("deberia poder crearse .mozilla/firefox");
        std::fs::create_dir_all(&elsewhere).expect("deberia poder crearse el otro sitio");
        std::fs::write(elsewhere.join("cert9.db"), b"").expect("deberia poder escribirse");
        std::fs::write(
            firefox.join("profiles.ini"),
            format!("[Profile0]\nIsRelative=0\nPath={}\n", elsewhere.display()),
        )
        .expect("deberia poder escribirse");

        assert_eq!(nss_profiles(home.path()), vec![elsewhere]);
    }
}
