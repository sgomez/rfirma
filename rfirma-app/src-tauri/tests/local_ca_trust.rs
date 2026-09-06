//! Integración de la CA local en almacén NSS real (ADR-0005, ADR-0014).

use std::path::Path;
use std::process::Command;

use rfirma_lib::app::trust::refresh_local_ca_trust;
use rfirma_lib::tls::{authority::COMMON_NAME, CaFiles, LocalCa, LocalCaStore};
use rfirma_lib::trust::{nss::is_trusted_ssl_ca, Moment, NssTrustStores, Situation, TrustStores};

/// Marca de confianza TLS en la salida de `certutil -L`.
const TRUSTED_FOR_TLS_ONLY: &str = "C,,";

/// Perfil NSS temporal desechable.
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

/// Contenido devuelto por `certutil -L`.
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

/// Filas de CA local marcadas como de confianza para TLS.
fn trusted_rows(profile: &Path) -> usize {
    certutil_listing(profile)
        .lines()
        .filter(|line| line.contains(COMMON_NAME) && line.contains(TRUSTED_FOR_TLS_ONLY))
        .count()
}

/// Almacén de CA local en un directorio temporal.
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

#[test]
fn installing_the_same_local_ca_twice_leaves_one_row() {
    let profile = a_disposable_profile();
    let ca = LocalCa::generate().expect("deberia fabricarse");

    install(profile.path(), &ca);
    install(profile.path(), &ca);

    assert_eq!(trusted_rows(profile.path()), 1);
}

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
