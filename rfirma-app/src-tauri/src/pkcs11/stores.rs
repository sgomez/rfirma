//! **Dónde** se buscan los certificados: la colección de almacenes.
//!
//! Hasta el #97 esto era una constante, `/app/lib/pkcs11/opensc-pkcs11.so`, y
//! una ruta única. Esa ruta **solo existe dentro del flatpak**, así que con
//! `just dev` la carga fallaba siempre y la ventana se quedaba sin
//! certificados. Ahora el binario resuelve los almacenes que de verdad hay
//! debajo —el del arenero cuando corre dentro, y los del anfitrión cuando corre
//! fuera— sin que nadie tenga que exportar nada a mano.
//!
//! Es una **colección** y no una ruta a propósito (ID-03): un almacén que no
//! cargue no puede dejar sin certificados a los demás. Hoy todos son módulos
//! PKCS#11; el almacén NSS de Mozilla entra por aquí, como un módulo más, en su
//! propio sub-issue.

use std::path::{Path, PathBuf};

/// Los módulos que se buscan cuando nadie dice otra cosa, en orden.
///
/// Se declaran por ruta absoluta y no se adivinan con `dlopen` a secas: cargar
/// «el primer `opensc-pkcs11.so` del `LD_LIBRARY_PATH`» es dejar que el entorno
/// decida con qué se firma.
pub const CANDIDATE_MODULES: &[&str] = &[
    // El que empaqueta el propio flatpak: los del anfitrión no cargan dentro
    // del arenero (`docs/research/flatpak-canal-unico.md`).
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

/// Los almacenes de esta máquina, resueltos al arrancar.
///
/// `RFIRMA_PKCS11_MODULE` sigue siendo la escotilla para apuntar a otro módulo
/// —de ella dependen las pruebas de grada B contra SoftHSM— y cuando está
/// puesta **manda ella sola**: quien la exporta quiere ese módulo y no el que
/// nosotros hubiéramos elegido.
pub fn from_environment() -> Vec<PathBuf> {
    match std::env::var_os(crate::PKCS11_MODULE_VARIABLE) {
        Some(module) => vec![PathBuf::from(module)],
        None => present_among(CANDIDATE_MODULES, |path| path.is_file()),
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
}
