use super::*;

fn installed_everywhere(stores: &Doubled, profiles: &[PathBuf], cas: &[&LocalCa]) {
    for profile in profiles {
        for ca in cas {
            stores
                .install(profile, &der_of(ca), COMMON_NAME)
                .expect("deberia registrarse");
        }
    }
}

fn marks_inside(stores: &Doubled, profile: &Path) -> Vec<Option<ChannelMark>> {
    stores
        .inside(profile)
        .iter()
        .map(|(der, _)| ChannelMark::of_certificate(der))
        .collect()
}

#[test]
fn the_first_install_makes_a_local_ca_with_the_mark_of_its_channel() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);

    refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Flatpak,
    )
    .expect("deberia poder fabricarse y registrarse");

    assert_eq!(
        marks_inside(&stores, &profiles[0]),
        vec![Some(ChannelMark::Flatpak)]
    );
}

#[test]
fn installing_replaces_an_unmarked_local_ca_with_a_marked_one_and_withdraws_the_old_one() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&unmarked).expect("deberia guardarse");
    installed_everywhere(&stores, &profiles, &[&unmarked]);

    let mut outcome = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Flatpak,
    )
    .expect("deberia poder sustituirse");

    let serving = store.serving().unwrap().unwrap();
    assert_eq!(serving.mark(), Some(ChannelMark::Flatpak));
    for profile in &profiles {
        assert_eq!(
            stores.inside(profile),
            vec![(der_of(&serving), COMMON_NAME.to_owned())],
            "solo queda la nueva, que es la vigente"
        );
    }
    assert_eq!(outcome.work, Work::ReplaceTheUnmarkedOne);
    assert_eq!(outcome.trusted, 2);
    assert_eq!(
        outcome.notice.when_the_errand_ends(),
        Some(Notice::RestartTheBrowser)
    );
}

#[test]
fn replacing_an_unmarked_local_ca_also_withdraws_the_one_waiting_in_the_overlap() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let serving = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    let waiting = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&serving).expect("deberia guardarse");
    store.write_next(&waiting).expect("deberia guardarse");
    installed_everywhere(&stores, &profiles, &[&serving, &waiting]);

    refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Native,
    )
    .expect("deberia poder sustituirse");

    assert!(store.next().unwrap().is_none());
    assert_eq!(
        marks_inside(&stores, &profiles[0]),
        vec![Some(ChannelMark::Native)]
    );
}

#[test]
fn a_marked_local_ca_is_not_replaced() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let marked = LocalCa::generate(ChannelMark::Native).expect("deberia fabricarse");
    store.write_serving(&marked).expect("deberia guardarse");
    installed_everywhere(&stores, &profiles, &[&marked]);

    let outcome = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Native,
    )
    .expect("deberia registrarse");

    assert_eq!(outcome.work, Work::InstallTheOneWeHave);
    assert_eq!(der_of(&store.serving().unwrap().unwrap()), der_of(&marked));
    assert_eq!(
        stores.inside(&profiles[0]),
        vec![(der_of(&marked), COMMON_NAME.to_owned())]
    );
}

#[test]
fn windows_does_not_replace_an_unmarked_local_ca() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&unmarked).expect("deberia guardarse");
    installed_everywhere(&stores, &profiles, &[&unmarked]);

    let mut outcome = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Windows,
    )
    .expect("deberia registrarse");

    assert_eq!(
        der_of(&store.serving().unwrap().unwrap()),
        der_of(&unmarked)
    );
    assert_eq!(
        stores.inside(&profiles[0]),
        vec![(der_of(&unmarked), COMMON_NAME.to_owned())]
    );
    assert_eq!(outcome.notice.when_the_errand_ends(), None);
}

#[test]
fn an_unmarked_local_ca_is_not_touched_in_the_middle_of_an_errand() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&unmarked).expect("deberia guardarse");
    installed_everywhere(&stores, &profiles, &[&unmarked]);

    let outcome = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::MidErrand,
        ChannelMark::Flatpak,
    )
    .expect("no hacer nada no es un fallo");

    assert_eq!(outcome.work, Work::Nothing);
    assert_eq!(
        der_of(&store.serving().unwrap().unwrap()),
        der_of(&unmarked)
    );
    assert_eq!(
        stores.inside(&profiles[0]),
        vec![(der_of(&unmarked), COMMON_NAME.to_owned())]
    );
}

#[test]
fn a_profile_that_refuses_the_replacement_is_reported_and_the_others_get_the_marked_one() {
    let store = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
    store.write_serving(&unmarked).expect("deberia guardarse");
    installed_everywhere(&stores, &profiles, &[&unmarked]);
    let stores = stores.refusing(&profiles[1]);

    let outcome = refresh_local_ca_trust(
        &store,
        &profiles,
        &stores,
        Moment::Startup,
        ChannelMark::Flatpak,
    )
    .expect("un perfil que falla no es un fallo del material");

    assert_eq!(outcome.trusted, 1);
    assert_eq!(outcome.missed.len(), 1);
    assert_eq!(outcome.missed[0].0, profiles[1]);
    assert_eq!(
        marks_inside(&stores, &profiles[0]),
        vec![Some(ChannelMark::Flatpak)]
    );
}

#[test]
fn each_distribution_channel_marks_its_local_ca_with_its_own_value() {
    assert_eq!(mark_of(Channel::Native).value(), "native");
    assert_eq!(mark_of(Channel::Flatpak).value(), "flatpak");
    assert_eq!(mark_of(Channel::Windows).value(), "windows");
}
