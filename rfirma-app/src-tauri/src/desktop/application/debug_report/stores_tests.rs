use std::path::PathBuf;

use super::tests::{a_flatpak_report, ana};
use super::*;

fn a_profile(browser: &str, directory: &str, state: NssProfileState) -> NssProfile {
    NssProfile {
        browser: browser.to_owned(),
        directory: PathBuf::from(directory),
        state,
    }
}

fn report_with_nss(profiles: Vec<NssProfile>, rfirma_store_installed: bool) -> String {
    let mut report = a_flatpak_report();
    report.pcsc = None;
    report.nss_stores = Some(NssStores {
        profiles,
        rfirma_store_installed,
    });
    let text = debug_report_text(&report, &ana());
    text.split_once("Almacenes NSS\n")
        .map(|(_, section)| section.to_owned())
        .unwrap_or_default()
}

#[test]
fn a_profile_that_trusts_the_local_channel_says_until_when() {
    let section = report_with_nss(
        vec![a_profile(
            "Firefox",
            "/home/ana/.mozilla/firefox/abcd1234.default",
            NssProfileState::TrustsLocalChannel {
                until: Some("2027-03-14".to_owned()),
            },
        )],
        false,
    );

    assert_eq!(
        section,
        "  Firefox: confía en el canal local · hasta 2027-03-14\n    \
           perfil: ~/.mozilla/firefox/<perfil>\n  \
         Almacén de rFirma: no instalado"
    );
}

#[test]
fn a_profile_that_does_not_trust_the_local_channel_says_so() {
    let section = report_with_nss(
        vec![a_profile(
            "Firefox snap",
            "/home/ana/snap/firefox/common/.mozilla/firefox/xyz.default",
            NssProfileState::DoesNotTrustLocalChannel,
        )],
        true,
    );

    assert_eq!(
        section,
        "  Firefox snap: no confía en el canal local\n    \
           perfil: ~/snap/firefox/common/.mozilla/firefox/<perfil>\n  \
         Almacén de rFirma: instalado"
    );
}

#[test]
fn a_profile_without_cert9_db_is_ignored() {
    let section = report_with_nss(
        vec![a_profile(
            "Firefox",
            "/home/ana/.mozilla/firefox/old.default",
            NssProfileState::IgnoredWithoutCertificateDatabase,
        )],
        false,
    );

    assert_eq!(
        section,
        "  Firefox: ignorado · sin cert9.db\n    \
           perfil: ~/.mozilla/firefox/<perfil>\n  \
         Almacén de rFirma: no instalado"
    );
}

fn report_with_windows(trust: NssProfileState, minidrivers: Vec<&str>) -> String {
    let mut report = a_flatpak_report();
    report.pcsc = None;
    report.windows_stores = Some(WindowsStores {
        local_channel: trust,
        minidrivers: minidrivers.into_iter().map(str::to_owned).collect(),
    });
    debug_report_text(&report, &ana())
}

#[test]
fn windows_replaces_the_nss_stores_with_the_windows_store_and_minidrivers() {
    let text = report_with_windows(
        NssProfileState::TrustsLocalChannel {
            until: Some("2027-03-14".to_owned()),
        },
        vec!["Gemalto IDPrime", "ACS ACOS5"],
    );

    assert!(
        text.ends_with(
            "Almacén de Windows\n  \
               Raíz del usuario: confía en el canal local · hasta 2027-03-14\n\
             \n\
             Minidrivers\n  \
               Gemalto IDPrime: registrado\n  \
               ACS ACOS5: registrado"
        ),
        "{text}"
    );
    assert!(!text.contains("Almacenes NSS"), "{text}");
}

#[test]
fn windows_without_trust_in_the_local_channel_says_so() {
    let text = report_with_windows(NssProfileState::DoesNotTrustLocalChannel, vec!["ACS ACOS5"]);

    assert!(
        text.contains("Almacén de Windows\n  Raíz del usuario: no confía en el canal local\n"),
        "{text}"
    );
}

#[test]
fn windows_without_registered_minidrivers_says_none() {
    let text = report_with_windows(NssProfileState::DoesNotTrustLocalChannel, Vec::new());

    assert!(
        text.ends_with("Minidrivers\n  tarjetas: ninguno registrado"),
        "{text}"
    );
}
