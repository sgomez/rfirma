//! Integración de la CA local en almacén NSS real (ADR-0005, ADR-0014).

use std::path::Path;
use std::process::Command;

use openssl::asn1::Asn1Time;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::x509::extension::BasicConstraints;
use openssl::x509::{X509Name, X509};
use rfirma_lib::identity::adapters::pkcs11::RealNssHost;
use rfirma_lib::site::adapters::nss::NssTrustStores;
use rfirma_lib::site::adapters::tls::{CaFiles, LocalCaStore};
use rfirma_lib::site::application::trust::refresh_local_ca_trust;
use rfirma_lib::site::domain::local_ca::{random_serial, COMMON_NAME};
use rfirma_lib::site::domain::local_ca::{ChannelMark, LocalCa};
use rfirma_lib::site::domain::trust::is_trusted_ssl_ca;
use rfirma_lib::site::domain::trust::{Moment, Situation};
use rfirma_lib::site::ports::TrustStores;

/// Marca de confianza TLS en la salida de `certutil -L`.
const TRUSTED_FOR_TLS_ONLY: &str = "C,,";

fn stores() -> NssTrustStores<RealNssHost> {
    NssTrustStores::new(RealNssHost)
}

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
    stores()
        .install(profile, &der_of(ca), COMMON_NAME)
        .expect("la CA local deberia entrar en el perfil");
}

fn withdraw(profile: &Path, ca: &LocalCa) {
    stores()
        .withdraw(profile, &der_of(ca))
        .expect("la retirada no deberia fallar");
}

#[test]
fn the_local_ca_ends_up_trusted_and_certutil_reads_the_bits() {
    let profile = a_disposable_profile();
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");

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
    let current = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la vigente");
    let next = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la siguiente");

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
    let current = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la vigente");
    let next = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la siguiente");

    install(profile.path(), &next);
    install(profile.path(), &current);

    assert_eq!(
        trusted_rows(profile.path()),
        2,
        "listado:\n{}",
        certutil_listing(profile.path())
    );
    let bits = stores()
        .trust_of(profile.path(), &der_of(&current))
        .expect("deberian leerse los bits");
    assert!(bits.is_some_and(is_trusted_ssl_ca), "bits: {bits:?}");
}

#[test]
fn installing_the_same_local_ca_twice_leaves_one_row() {
    let profile = a_disposable_profile();
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");

    install(profile.path(), &ca);
    install(profile.path(), &ca);

    assert_eq!(trusted_rows(profile.path()), 1);
}

#[test]
fn the_bits_come_back_and_a_ca_that_is_not_there_is_not_a_failure() {
    let profile = a_disposable_profile();
    let installed = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");
    let stranger = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");

    install(profile.path(), &installed);

    assert!(stores()
        .trust_of(profile.path(), &der_of(&installed))
        .expect("deberian leerse los bits")
        .is_some_and(is_trusted_ssl_ca));
    assert_eq!(
        stores()
            .trust_of(profile.path(), &der_of(&stranger))
            .expect("no estar no es un fallo"),
        None
    );
}

#[test]
fn withdrawing_both_local_cas_after_an_overlap_leaves_the_store_trusting_neither() {
    let profile = a_disposable_profile();
    let current = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la vigente");
    let next = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la siguiente");

    install(profile.path(), &current);
    install(profile.path(), &next);
    withdraw(profile.path(), &current);
    withdraw(profile.path(), &next);

    assert_eq!(
        trusted_rows(profile.path()),
        0,
        "listado:\n{}",
        certutil_listing(profile.path())
    );
    assert_eq!(
        stores()
            .trust_of(profile.path(), &der_of(&current))
            .expect("deberian leerse los bits"),
        None
    );
    assert_eq!(
        stores()
            .trust_of(profile.path(), &der_of(&next))
            .expect("deberian leerse los bits"),
        None
    );
}

#[test]
fn withdrawing_a_local_ca_that_is_not_there_is_not_a_failure() {
    let profile = a_disposable_profile();
    let stranger = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");

    withdraw(profile.path(), &stranger);
}

#[test]
fn withdrawing_one_local_ca_leaves_the_other_certificates_trusted() {
    let profile = a_disposable_profile();
    let current = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la vigente");
    let next = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la siguiente");

    install(profile.path(), &current);
    install(profile.path(), &next);
    withdraw(profile.path(), &current);

    assert_eq!(
        trusted_rows(profile.path()),
        1,
        "listado:\n{}",
        certutil_listing(profile.path())
    );
    assert!(stores()
        .trust_of(profile.path(), &der_of(&next))
        .expect("deberian leerse los bits")
        .is_some_and(is_trusted_ssl_ca));
    assert_eq!(
        stores()
            .trust_of(profile.path(), &der_of(&current))
            .expect("deberian leerse los bits"),
        None
    );
}

#[test]
fn a_directory_that_is_not_a_profile_says_the_store_is_unreachable() {
    let nowhere = tempfile::tempdir().expect("deberia haber directorio temporal");
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");

    let error = stores()
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

    let mut first = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores(),
        Moment::Startup,
        ChannelMark::Native,
    )
    .expect("deberia poder instalarse");
    let mut second = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores(),
        Moment::Startup,
        ChannelMark::Native,
    )
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
        &stores(),
        Moment::MidErrand,
        ChannelMark::Native,
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
    let current = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la vigente");
    let next = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la siguiente");

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

/// Autoridad autofirmada con el sujeto dado y sin las restricciones de nombre de la CA local.
fn a_stranger_named(entries: &[(Nid, &str)]) -> Vec<u8> {
    let key = PKey::from_ec_key(
        EcKey::generate(
            &EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).expect("deberia haber curva"),
        )
        .expect("deberia generarse la clave"),
    )
    .expect("deberia envolverse la clave");
    let mut name = X509Name::builder().expect("deberia construirse el nombre");
    for (nid, value) in entries {
        name.append_entry_by_nid(*nid, value)
            .expect("deberia añadirse la entrada");
    }
    let name = name.build();
    let mut builder = X509::builder().expect("deberia construirse el certificado");
    builder.set_version(2).expect("versión");
    builder
        .set_serial_number(&random_serial().expect("serie"))
        .expect("serie");
    builder.set_subject_name(&name).expect("sujeto");
    builder.set_issuer_name(&name).expect("emisor");
    builder.set_pubkey(&key).expect("clave");
    builder
        .set_not_before(&Asn1Time::days_from_now(0).expect("inicio"))
        .expect("inicio");
    builder
        .set_not_after(&Asn1Time::days_from_now(30).expect("fin"))
        .expect("fin");
    builder
        .append_extension(BasicConstraints::new().critical().ca().build().expect("CA"))
        .expect("CA");
    builder
        .sign(&key, MessageDigest::sha256())
        .expect("deberia firmarse");
    builder.build().to_der().expect("deberia salir en DER")
}

fn sorted(mut ders: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    ders.sort();
    ders
}

/// Los ficheros del perfil con su contenido, para ver si algo los ha tocado.
fn contents_of(profile: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(profile)
        .expect("deberia leerse el perfil")
        .map(|entry| entry.expect("deberia leerse la entrada").path())
        .map(|path| {
            (
                path.display().to_string(),
                std::fs::read(&path).expect("deberia leerse el fichero"),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn every_certificate_with_the_local_ca_subject_is_listed_and_no_other() {
    let profile = a_disposable_profile();
    let current = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la vigente");
    let next = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la siguiente");
    let namesake = a_stranger_named(&[(Nid::COMMONNAME, COMMON_NAME)]);
    let other_name = a_stranger_named(&[(Nid::COMMONNAME, "Otra CA")]);
    let longer_subject = a_stranger_named(&[
        (Nid::COMMONNAME, COMMON_NAME),
        (Nid::ORGANIZATIONNAME, "Ajena"),
    ]);

    install(profile.path(), &current);
    install(profile.path(), &next);
    stores()
        .install(profile.path(), &namesake, COMMON_NAME)
        .expect("deberia entrar la homónima");
    stores()
        .install(profile.path(), &other_name, "Otra CA")
        .expect("deberia entrar la ajena");
    stores()
        .install(profile.path(), &longer_subject, "Ajena")
        .expect("deberia entrar la de sujeto más largo");

    let listed = stores()
        .local_cas(profile.path())
        .expect("deberia poder listarse");

    assert_eq!(
        sorted(listed),
        sorted(vec![der_of(&current), der_of(&next), namesake]),
        "listado:\n{}",
        certutil_listing(profile.path())
    );
}

#[test]
fn a_profile_without_any_local_ca_lists_nothing() {
    let profile = a_disposable_profile();
    stores()
        .install(
            profile.path(),
            &a_stranger_named(&[(Nid::COMMONNAME, "Otra CA")]),
            "Otra CA",
        )
        .expect("deberia entrar la ajena");

    let listed = stores()
        .local_cas(profile.path())
        .expect("un perfil sin CA local no es un fallo");

    assert!(listed.is_empty());
}

#[test]
fn listing_the_local_cas_leaves_the_profile_untouched() {
    let profile = a_disposable_profile();
    install(
        profile.path(),
        &LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse"),
    );
    let before = contents_of(profile.path());

    stores()
        .local_cas(profile.path())
        .expect("deberia poder listarse");

    assert_eq!(contents_of(profile.path()), before);
}

#[test]
fn each_local_ca_listed_from_a_real_profile_keeps_the_mark_of_its_channel() {
    let profile = a_disposable_profile();
    install(
        profile.path(),
        &LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse la del deb"),
    );
    install(
        profile.path(),
        &LocalCa::generate(ChannelMark::Flatpak).expect("deberia fabricarse la del flatpak"),
    );
    stores()
        .install(
            profile.path(),
            &a_stranger_named(&[(Nid::COMMONNAME, COMMON_NAME)]),
            COMMON_NAME,
        )
        .expect("deberia entrar la homónima sin marca");

    let marks: Vec<Option<ChannelMark>> = stores()
        .local_cas(profile.path())
        .expect("deberia poder listarse")
        .iter()
        .map(|der| ChannelMark::of_certificate(der))
        .collect();

    assert_eq!(marks.len(), 3, "{marks:?}");
    for mark in [Some(ChannelMark::Native), Some(ChannelMark::Flatpak), None] {
        assert!(marks.contains(&mark), "falta {mark:?} en {marks:?}");
    }
}
