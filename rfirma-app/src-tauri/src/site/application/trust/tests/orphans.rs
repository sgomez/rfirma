use super::*;
use crate::site::domain::local_ca::generate_key;
use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::x509::{X509Name, X509};

fn a_namesake_without_the_constraints() -> Vec<u8> {
    let key = generate_key().expect("deberia generarse la clave");
    let mut name = X509Name::builder().expect("nombre");
    name.append_entry_by_nid(Nid::COMMONNAME, COMMON_NAME)
        .expect("CN");
    let name = name.build();
    let mut builder = X509::builder().expect("certificado");
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
        .sign(&key, MessageDigest::sha256())
        .expect("deberia firmarse");
    builder.build().to_der().expect("deberia salir en DER")
}

fn a_ca(mark: ChannelMark) -> LocalCa {
    LocalCa::generate(mark).expect("deberia fabricarse")
}

fn installed(stores: &Doubled, profiles: &[PathBuf], ders: &[Vec<u8>]) {
    for profile in profiles {
        for der in ders {
            stores
                .install(profile, der, COMMON_NAME)
                .expect("el doble deja instalar en la preparacion");
        }
    }
}

fn ders_inside(stores: &Doubled, profile: &Path) -> Vec<Vec<u8>> {
    let mut ders: Vec<Vec<u8>> = stores
        .inside(profile)
        .into_iter()
        .map(|(der, _)| der)
        .collect();
    ders.sort();
    ders
}

fn sorted(mut ders: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    ders.sort();
    ders
}

fn install(
    store: &InMemoryCaSlots,
    profiles: &[PathBuf],
    stores: &Doubled,
    moment: Moment,
    mark: ChannelMark,
) -> TrustOutcome {
    refresh_local_ca_trust(store, profiles, stores, moment, mark)
        .expect("un perfil que falla no es un fallo del material")
}

#[test]
fn installing_withdraws_the_old_local_cas_of_its_channel_and_keeps_the_current_and_the_next() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let (current, next, old) = (
        a_ca(ChannelMark::Flatpak),
        a_ca(ChannelMark::Flatpak),
        a_ca(ChannelMark::Flatpak),
    );
    store.write_serving(&current).expect("deberia guardarse");
    store.write_next(&next).expect("deberia guardarse");
    installed(&stores, &profiles, &[der_of(&old), der_of(&next)]);

    install(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Flatpak,
    );

    for profile in &profiles {
        assert_eq!(
            ders_inside(&stores, profile),
            sorted(vec![der_of(&current), der_of(&next)]),
            "la antigua del canal se retira; la vigente y la siguiente se quedan"
        );
    }
}

#[test]
fn installing_keeps_the_local_cas_marked_by_the_other_channel() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = a_ca(ChannelMark::Flatpak);
    let the_debs = a_ca(ChannelMark::Native);
    store.write_serving(&current).expect("deberia guardarse");
    installed(&stores, &profiles, &[der_of(&the_debs)]);

    install(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Flatpak,
    );

    assert_eq!(
        ders_inside(&stores, &profiles[0]),
        sorted(vec![der_of(&current), der_of(&the_debs)])
    );
}

#[test]
fn installing_withdraws_the_unmarked_local_cas_from_before_the_mark() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = a_ca(ChannelMark::Native);
    let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&current).expect("deberia guardarse");
    installed(&stores, &profiles, &[der_of(&current), der_of(&unmarked)]);

    let mut outcome = install(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Native,
    );

    assert_eq!(ders_inside(&stores, &profiles[0]), vec![der_of(&current)]);
    assert_eq!(ders_inside(&stores, &profiles[1]), vec![der_of(&current)]);
    assert_eq!(
        outcome.notice.when_the_errand_ends(),
        None,
        "retirar una huérfana no obliga a reiniciar el navegador"
    );
}

#[test]
fn installing_does_not_touch_a_namesake_without_the_name_constraints_of_rfirma() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = a_ca(ChannelMark::Native);
    let namesake = a_namesake_without_the_constraints();
    store.write_serving(&current).expect("deberia guardarse");
    installed(&stores, &profiles, &[namesake.clone()]);

    install(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Native,
    );

    assert_eq!(
        ders_inside(&stores, &profiles[0]),
        sorted(vec![der_of(&current), namesake])
    );
}

#[test]
fn a_profile_that_refuses_to_withdraw_is_reported_and_keeps_the_local_ca_like_the_others() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = a_ca(ChannelMark::Native);
    let old = a_ca(ChannelMark::Native);
    store.write_serving(&current).expect("deberia guardarse");
    installed(&stores, &profiles, &[der_of(&old)]);
    let stores = stores.refusing_to_withdraw(&profiles[1]);

    let outcome = install(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Native,
    );

    assert_eq!(ders_inside(&stores, &profiles[0]), vec![der_of(&current)]);
    assert!(
        ders_inside(&stores, &profiles[1]).contains(&der_of(&current)),
        "el perfil que no deja retirar se queda con la CA vigente"
    );
    assert_eq!(outcome.missed.len(), 1);
    assert_eq!(outcome.missed[0].0, profiles[1]);
    assert_eq!(outcome.trusted, 2);
}

#[test]
fn nothing_is_withdrawn_in_the_middle_of_an_errand() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = a_ca(ChannelMark::Native);
    let old = a_ca(ChannelMark::Native);
    store.write_serving(&current).expect("deberia guardarse");
    installed(&stores, &profiles, &[der_of(&current), der_of(&old)]);

    install(
        &store,
        &profiles,
        &stores,
        Moment::MidErrand,
        ChannelMark::Native,
    );

    assert_eq!(
        ders_inside(&stores, &profiles[0]),
        sorted(vec![der_of(&current), der_of(&old)])
    );
}

#[test]
fn windows_withdraws_no_old_local_ca() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = a_ca(ChannelMark::Windows);
    let old = a_ca(ChannelMark::Windows);
    let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&current).expect("deberia guardarse");
    installed(&stores, &profiles, &[der_of(&old), der_of(&unmarked)]);

    install(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Windows,
    );

    assert_eq!(
        ders_inside(&stores, &profiles[0]),
        sorted(vec![der_of(&current), der_of(&old), der_of(&unmarked)])
    );
}
