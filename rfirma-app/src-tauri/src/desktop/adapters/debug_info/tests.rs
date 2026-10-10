use std::path::Path;

use crate::site::domain::local_ca::{ChannelMark, LocalCa};

use super::*;

#[test]
fn a_discarded_module_says_why_without_being_probed() {
    let reasons = [
        (
            p11kit::DiscardReason::DisabledInRfirma,
            DiscardReason::DisabledInRfirma,
        ),
        (
            p11kit::DiscardReason::EnabledOnlyElsewhere,
            DiscardReason::EnabledOnlyElsewhere,
        ),
        (
            p11kit::DiscardReason::TrustPolicy,
            DiscardReason::TrustPolicy,
        ),
        (
            p11kit::DiscardReason::MissingModule,
            DiscardReason::MissingModule,
        ),
    ];

    for (found, reported) in reasons {
        assert_eq!(
            module_status(Some(Path::new("/nonexistent/libnothing.so")), Some(found)),
            ModuleStatus::Discarded(reported)
        );
    }
}

#[test]
fn a_module_without_a_library_does_not_load() {
    assert_eq!(module_status(None, None), ModuleStatus::DoesNotLoad);
}

#[test]
fn a_module_whose_library_does_not_exist_does_not_load() {
    assert_eq!(
        module_status(Some(Path::new("/nonexistent/libnothing.so")), None),
        ModuleStatus::DoesNotLoad
    );
}

#[test]
fn without_a_serving_local_ca_there_is_no_expiry() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    assert_eq!(
        local_ca_expiry(&LocalCaStore::of(&Paths::under(directory.path()))),
        None
    );
}

#[test]
fn the_expiry_of_the_serving_local_ca_is_the_day_its_certificate_ends() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = LocalCaStore::of(&Paths::under(directory.path()));
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");
    store.write(&ca).expect("deberia guardarse");

    let expiry = local_ca_expiry(&store).expect("la CA vigente tiene caducidad");

    let day = expiry.replace('-', "");
    let first_second = Asn1Time::from_str(&format!("{day}000000Z")).expect("fecha valida");
    let last_second = Asn1Time::from_str(&format!("{day}235959Z")).expect("fecha valida");
    let not_after = ca.certificate().not_after();
    assert!(&*first_second <= not_after && not_after <= &*last_second);
}

#[test]
fn a_firefox_profile_is_named_after_its_browser_and_packaging() {
    assert_eq!(
        browser_of(Path::new("/home/u/.mozilla/firefox/abc.default")),
        "Firefox"
    );
    assert_eq!(
        browser_of(Path::new(
            "/home/u/snap/firefox/common/.mozilla/firefox/abc.default"
        )),
        "Firefox snap"
    );
    assert_eq!(
        browser_of(Path::new(
            "/home/u/.var/app/org.mozilla.firefox/.mozilla/firefox/abc.default"
        )),
        "Firefox flatpak"
    );
}

#[test]
fn the_shared_nss_database_is_chrome_and_any_other_is_the_systems() {
    assert_eq!(browser_of(Path::new("/home/u/.pki/nssdb")), "Chrome");
    assert_eq!(
        browser_of(Path::new("/etc/pki/nssdb-other")),
        "NSS del sistema"
    );
}
