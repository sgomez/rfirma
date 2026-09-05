//! **La CA local dentro de un almacén NSS de verdad** (ADR-0005, ID-227,
//! ID-228, TD-60). **Grada B** (ADR-0014, TD-02): carril rápido, segundos.
//!
//! # El oráculo es `certutil -L`, y solo `certutil -L`
//!
//! La confianza se comprueba **leyendo los bits**, nunca verificando una
//! cadena: el #326 midió que el veredicto de `vfychain` sale **invertido**
//! respecto a lo que hace Firefox de verdad, así que no vale ni de oráculo
//! binario (ID-227, TD-60). Aquí no aparece `vfychain` por ninguna parte, y no
//! debe aparecer.
//!
//! `certutil -L` imprime **una línea por certificado**, no por apodo: dos CA
//! locales con el mismo sujeto y el mismo apodo salen como dos filas
//! `rFirma CA local … C,,`. Eso es lo que hace que el solape del ID-224 sea
//! comprobable desde fuera y no una promesa.
//!
//! # Cómo se monta el perfil
//!
//! ```sh
//! sudo apt install -y libnss3 libnss3-tools
//! ```
//!
//! Cada prueba se hace **su propio perfil desechable** en un directorio
//! temporal, con `certutil -N --empty-password`, que es un Firefox recién
//! instalado. El perfil real de nadie se toca jamás: aquí no aparece
//! `~/.mozilla`.

use std::path::Path;
use std::process::Command;

use rfirma_lib::app::trust::refresh_local_ca_trust;
use rfirma_lib::tls::{authority::COMMON_NAME, CaFiles, LocalCa, LocalCaStore};
use rfirma_lib::trust::{nss::is_trusted_ssl_ca, Moment, NssTrustStores, Situation, TrustStores};

/// Lo que `certutil -L` imprime en la columna de la izquierda para una CA de
/// confianza para TLS y para nada más.
const TRUSTED_FOR_TLS_ONLY: &str = "C,,";

/// Un perfil NSS vacío y desechable, el de un Firefox recién instalado.
fn a_disposable_profile() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let created = Command::new("certutil")
        .args(["-N", "-d"])
        .arg(format!("sql:{}", directory.path().display()))
        .arg("--empty-password")
        .output()
        .expect(
            "falta certutil. Las pruebas de grada B de la confianza lo necesitan:\n  \
             sudo apt install -y libnss3 libnss3-tools",
        );
    assert!(
        created.status.success(),
        "no se ha podido crear el perfil NSS:\n{}",
        String::from_utf8_lossy(&created.stderr)
    );
    directory
}

/// El listado del almacén, tal y como lo imprime `certutil -L`.
///
/// **Este es el oráculo del TD-60**: los bits, leídos por la herramienta de
/// NSS, sin verificar ninguna cadena.
fn certutil_listing(profile: &Path) -> String {
    let listed = Command::new("certutil")
        .args(["-L", "-d"])
        .arg(format!("sql:{}", profile.display()))
        .output()
        .expect("deberia poder ejecutarse certutil");
    assert!(
        listed.status.success(),
        "certutil -L ha fallado:\n{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    String::from_utf8_lossy(&listed.stdout).into_owned()
}

/// Cuántas filas de la CA local hay marcadas como CA de confianza para TLS.
fn trusted_rows(profile: &Path) -> usize {
    certutil_listing(profile)
        .lines()
        .filter(|line| line.contains(COMMON_NAME) && line.contains(TRUSTED_FOR_TLS_ONLY))
        .count()
}

/// Un almacén de CA local con sus dos ranuras dentro de un directorio de datos
/// desechable.
fn a_store_in(data: &Path) -> LocalCaStore {
    LocalCaStore::new(
        CaFiles::new(data.join("ca-local.pem"), data.join("ca-local.key")),
        CaFiles::new(
            data.join("ca-local-next.pem"),
            data.join("ca-local-next.key"),
        ),
    )
}

fn der_of(ca: &LocalCa) -> Vec<u8> {
    ca.certificate()
        .to_der()
        .expect("el certificado deberia salir en DER")
}

fn install(profile: &Path, ca: &LocalCa) {
    NssTrustStores
        .install(profile, &der_of(ca), COMMON_NAME)
        .expect("la CA local deberia entrar en el perfil");
}

/// **El criterio de aceptación entero, leído por `certutil -L`**: la CA local
/// queda dentro y marcada como CA de confianza para TLS, y **solo** para TLS.
#[test]
fn the_local_ca_ends_up_trusted_and_certutil_reads_the_bits() {
    let profile = a_disposable_profile();
    let ca = LocalCa::generate().expect("deberia fabricarse");

    install(profile.path(), &ca);

    let listing = certutil_listing(profile.path());
    assert!(
        listing.contains(COMMON_NAME),
        "la CA local no está en el perfil:\n{listing}"
    );
    assert_eq!(trusted_rows(profile.path()), 1, "listado:\n{listing}");
}

/// **El solape** (ID-224, TD-60): dos certificados de confianza con el mismo
/// sujeto conviven, y `certutil -L` los enseña como dos filas.
#[test]
fn two_local_ca_with_the_same_subject_live_together() {
    let profile = a_disposable_profile();
    let current = LocalCa::generate().expect("deberia fabricarse la vigente");
    let next = LocalCa::generate().expect("deberia fabricarse la siguiente");

    install(profile.path(), &current);
    install(profile.path(), &next);

    assert_eq!(
        trusted_rows(profile.path()),
        2,
        "listado:\n{}",
        certutil_listing(profile.path())
    );
}

/// **En cualquier orden** (ID-224): meter primero la que servirá y después la
/// que sirve tiene que dar exactamente lo mismo.
#[test]
fn the_overlap_holds_whichever_order_they_arrive_in() {
    let profile = a_disposable_profile();
    let current = LocalCa::generate().expect("deberia fabricarse la vigente");
    let next = LocalCa::generate().expect("deberia fabricarse la siguiente");

    install(profile.path(), &next);
    install(profile.path(), &current);

    assert_eq!(
        trusted_rows(profile.path()),
        2,
        "listado:\n{}",
        certutil_listing(profile.path())
    );
    let bits = NssTrustStores
        .trust_of(profile.path(), &der_of(&current))
        .expect("deberian leerse los bits");
    assert!(bits.is_some_and(is_trusted_ssl_ca), "bits: {bits:?}");
}

/// Registrar la misma CA dos veces no duplica nada: es lo que permite llamar al
/// caso de uso en cada arranque sin ensuciar el perfil de la persona.
#[test]
fn installing_the_same_local_ca_twice_leaves_one_row() {
    let profile = a_disposable_profile();
    let ca = LocalCa::generate().expect("deberia fabricarse");

    install(profile.path(), &ca);
    install(profile.path(), &ca);

    assert_eq!(trusted_rows(profile.path()), 1);
}

/// Los bits se leen de vuelta, y una CA que no está no da `Some(0)` ni un
/// fallo: da `None`.
#[test]
fn the_bits_come_back_and_a_ca_that_is_not_there_is_not_a_failure() {
    let profile = a_disposable_profile();
    let installed = LocalCa::generate().expect("deberia fabricarse");
    let stranger = LocalCa::generate().expect("deberia fabricarse");

    install(profile.path(), &installed);

    assert!(NssTrustStores
        .trust_of(profile.path(), &der_of(&installed))
        .expect("deberian leerse los bits")
        .is_some_and(is_trusted_ssl_ca));
    assert_eq!(
        NssTrustStores
            .trust_of(profile.path(), &der_of(&stranger))
            .expect("no estar no es un fallo"),
        None
    );
}

/// Un directorio que no es un perfil NSS se dice, y se dice como lo que es: el
/// almacén no se ha podido abrir. Es la situación con la que sale el flatpak al
/// que le falte el permiso del ID-228.
#[test]
fn a_directory_that_is_not_a_profile_says_the_store_is_unreachable() {
    let nowhere = tempfile::tempdir().expect("deberia haber directorio temporal");
    let ca = LocalCa::generate().expect("deberia fabricarse");

    let error = NssTrustStores
        .install(&nowhere.path().join("no-existe"), &der_of(&ca), COMMON_NAME)
        .expect_err("un perfil que no se puede abrir no es un éxito");

    assert_eq!(error.situation(), Situation::StoreUnreachable);
    assert!(!error.detail().is_empty());
}

/// **El caso de uso entero contra un perfil de verdad**: el primer arranque
/// deja la CA local fabricada, guardada y de confianza, y `certutil -L` lo
/// confirma. El segundo no vuelve a escribir ni vuelve a avisar.
#[test]
fn the_first_boot_leaves_the_local_ca_trusted_in_a_real_profile() {
    let data = tempfile::tempdir().expect("deberia haber directorio temporal");
    let profile = a_disposable_profile();
    let store = a_store_in(data.path());
    let profiles = [profile.path().to_path_buf()];

    let mut first = refresh_local_ca_trust(&store, &profiles, &NssTrustStores, Moment::Startup)
        .expect("deberia poder instalarse");
    let mut second = refresh_local_ca_trust(&store, &profiles, &NssTrustStores, Moment::Startup)
        .expect("deberia poder repetirse");

    assert_eq!(first.trusted, 1);
    assert!(first.missed.is_empty());
    assert!(first.notice.when_the_errand_ends().is_some());
    assert_eq!(second.trusted, 1);
    assert!(
        second.notice.when_the_errand_ends().is_none(),
        "el aviso no se repite en cada arranque"
    );
    assert_eq!(trusted_rows(profile.path()), 1);
}

/// **ID-224.** Con un trámite en marcha no se toca el perfil de nadie, ni
/// siquiera cuando no hay CA local ninguna.
#[test]
fn nothing_is_written_in_a_real_profile_in_the_middle_of_an_errand() {
    let data = tempfile::tempdir().expect("deberia haber directorio temporal");
    let profile = a_disposable_profile();
    let store = a_store_in(data.path());

    let outcome = refresh_local_ca_trust(
        &store,
        &[profile.path().to_path_buf()],
        &NssTrustStores,
        Moment::MidErrand,
    )
    .expect("no hacer nada no es un fallo");

    assert!(!outcome.looked(), "no se ha abierto ningún perfil");
    assert!(
        !outcome.nowhere(),
        "sin mirar no se puede afirmar que la CA no esté en ninguna parte"
    );
    assert_eq!(trusted_rows(profile.path()), 0);
    assert!(store.read().expect("deberia leerse").is_none());
}

/// **El solape contra un perfil de verdad** (ID-224): la vigente sigue siendo
/// la que sirve —la que firmará el certificado del servidor local— mientras la
/// siguiente espera en su propia ranura, y `certutil -L` enseña las dos filas.
///
/// El relevo, cuando la vigente caduca, se comprueba en las pruebas unitarias
/// del caso de uso: fabricar una CA ya caducada es un andamio `#[cfg(test)]` y
/// no cruza a una prueba de integración.
#[test]
fn during_the_overlap_the_serving_ca_keeps_serving_and_both_are_trusted() {
    let data = tempfile::tempdir().expect("deberia haber directorio temporal");
    let profile = a_disposable_profile();
    let store = a_store_in(data.path());
    let current = LocalCa::generate().expect("deberia fabricarse la vigente");
    let next = LocalCa::generate().expect("deberia fabricarse la siguiente");

    store.write(&current).expect("deberia guardarse la vigente");
    install(profile.path(), &current);
    store
        .write_next(&next)
        .expect("deberia guardarse la siguiente");
    install(profile.path(), &next);

    assert_eq!(
        der_of(
            &store
                .read()
                .expect("deberia leerse")
                .expect("la vigente sigue guardada")
        ),
        der_of(&current),
        "durante el solape la que firma sigue siendo la vigente"
    );
    assert_eq!(
        trusted_rows(profile.path()),
        2,
        "listado:\n{}",
        certutil_listing(profile.path())
    );

    // Y el relevo deja servir a la que llevaba meses instalada, sin tocar el
    // perfil: `promote_next` no escribe en ningún almacén.
    let promoted = store
        .promote_next()
        .expect("deberia poder relevarse")
        .expect("habia una siguiente esperando");

    assert_eq!(der_of(&promoted), der_of(&next));
    assert_eq!(
        trusted_rows(profile.path()),
        2,
        "el relevo no instala nada: las dos ya estaban"
    );
}
