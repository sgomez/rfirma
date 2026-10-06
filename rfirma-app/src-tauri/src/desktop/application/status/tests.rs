use std::time::{Duration, SystemTime};

use super::*;
use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::handlers::{UrlHandler, FLATPAK_DESKTOP_FILE, OUR_DESKTOP_FILE};
use crate::desktop::domain::status::{
    ActionKind, Signal, SignalDetail, StoreBrand, StoreCertificates, Verdict,
};
use crate::desktop::domain::version_check::VersionCheck;
use crate::signing::application::tests::a_memory;

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

fn a_release(tag: &str) -> String {
    format!(r#"{{"tag_name":"{tag}","name":"rFirma {tag}"}}"#)
}

#[test]
fn version_is_up_to_date_when_announced_is_same() {
    let running = Version::parse("0.4.1").unwrap();
    let row = evaluate_version_signal(running, Some(running), Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn version_is_up_to_date_when_announced_is_older() {
    let running = Version::parse("0.4.1").unwrap();
    let older = Version::parse("0.3.9").unwrap();
    let row = evaluate_version_signal(running, Some(older), Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn version_is_up_to_date_when_no_version_announced() {
    let running = Version::parse("0.4.1").unwrap();
    let row = evaluate_version_signal(running, None, Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn version_requires_attention_when_newer_published_on_native_channel() {
    let running = Version::parse("0.4.1").unwrap();
    let newer = Version::parse("0.5.0").unwrap();
    let row = evaluate_version_signal(running, Some(newer), Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1 → 0.5.0");
    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Link,
            target: "releases".into(),
        })
    );
}

#[test]
fn version_requires_attention_when_newer_published_on_flatpak_channel() {
    let running = Version::parse("0.4.1").unwrap();
    let newer = Version::parse("0.5.0").unwrap();
    let row = evaluate_version_signal(running, Some(newer), Channel::Flatpak);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1 → 0.5.0");
    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Link,
            target: "repository".into(),
        })
    );
}

#[test]
fn checking_version_signal_has_no_value_nor_action() {
    let row = checking_version_signal();

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "");
    assert_eq!(row.verdict, Verdict::Checking);
    assert_eq!(row.action, None);
}

#[test]
fn each_channel_updates_from_its_own_destination() {
    assert_eq!(update_destination_for(Channel::Windows), "windows");
    assert_eq!(update_destination_for(Channel::Native), "releases");
    assert_eq!(update_destination_for(Channel::Flatpak), "repository");
}

#[test]
fn measure_version_signal_answers_with_what_the_feed_announces() {
    let home = tempfile::tempdir().expect("tempdir");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.1").unwrap();

    let row = measure_version_signal(
        running,
        &memory,
        &|| Some(a_release("v0.5.0")),
        Channel::Native,
        at(1_000_000),
    );

    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(row.value, "0.4.1 → 0.5.0");

    let cached = memory.last_version_check().expect("guardada en memoria");
    assert_eq!(cached.announced, "0.5.0");
}

#[test]
fn measure_version_signal_without_network_falls_back_to_the_remembered_answer() {
    let home = tempfile::tempdir().expect("tempdir");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.1").unwrap();

    let _ = memory.remember_version_check(VersionCheck {
        checked_at: 1_000_000,
        announced: "0.5.0".into(),
    });

    let row = measure_version_signal(running, &memory, &|| None, Channel::Native, at(1_000_100));

    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(row.value, "0.4.1 → 0.5.0");
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Link,
            target: "releases".into(),
        })
    );
}

#[test]
fn measure_version_signal_without_network_nor_memory_is_correct() {
    let home = tempfile::tempdir().expect("tempdir");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.1").unwrap();

    let row = measure_version_signal(running, &memory, &|| None, Channel::Native, at(1_000_000));

    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.action, None);
}

#[test]
fn no_certificates_found_requires_attention_and_offers_how_to_install() {
    let row = evaluate_user_certificates_signal(vec![]);

    assert_eq!(row.signal, Signal::UserCertificates);
    assert_eq!(row.value, "0");
    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Link,
            target: "certificateIssuance".into(),
        })
    );
    assert_eq!(row.detail, None);
}

#[test]
fn one_certificate_found_is_correct_without_action() {
    let stores = vec![a_place(StoreBrand::Card, 1)];

    let row = evaluate_user_certificates_signal(stores.clone());

    assert_eq!(row.signal, Signal::UserCertificates);
    assert_eq!(row.value, "1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
    assert_eq!(row.detail, Some(SignalDetail::Certificates { stores }));
}

#[test]
fn the_value_adds_up_what_every_place_holds() {
    let stores = vec![
        a_place(StoreBrand::Firefox, 2),
        a_place(StoreBrand::Card, 1),
        a_place(StoreBrand::Installed, 1),
    ];

    let row = evaluate_user_certificates_signal(stores.clone());

    assert_eq!(row.value, "4");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
    assert_eq!(row.detail, Some(SignalDetail::Certificates { stores }));
}

#[test]
fn checking_local_ca_certificate_signal_has_no_value_nor_detail() {
    let row = checking_local_ca_certificate_signal();

    assert_eq!(row.signal, Signal::LocalCaCertificate);
    assert_eq!(row.value, "");
    assert_eq!(row.verdict, Verdict::Checking);
    assert_eq!(row.action, None);
    assert_eq!(row.detail, None);
    assert!(!row.restart_firefox_notice);
}

fn a_store(brand: StoreBrand, trusted: bool) -> StoreDetail {
    StoreDetail { brand, trusted }
}

fn a_place(brand: StoreBrand, certificates: usize) -> StoreCertificates {
    StoreCertificates {
        brand,
        certificates,
    }
}

#[test]
fn no_store_trusted_is_incorrect_and_offers_to_install() {
    let detail = vec![
        a_store(StoreBrand::Firefox, false),
        a_store(StoreBrand::Chrome, false),
        a_store(StoreBrand::Nssdb, false),
    ];

    let row = evaluate_local_ca_certificate_signal(detail.clone(), false);

    assert_eq!(row.signal, Signal::LocalCaCertificate);
    assert_eq!(row.value, "0/3");
    assert_eq!(row.verdict, Verdict::Incorrect);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Repair,
            target: INSTALL_LOCAL_CA_CERTIFICATE.into(),
        })
    );
    assert_eq!(row.detail, Some(SignalDetail::Trust { stores: detail }));
}

#[test]
fn some_stores_trusted_is_attention_and_offers_to_install() {
    let detail = vec![
        a_store(StoreBrand::Firefox, true),
        a_store(StoreBrand::Chrome, false),
        a_store(StoreBrand::Nssdb, false),
    ];

    let row = evaluate_local_ca_certificate_signal(detail, false);

    assert_eq!(row.value, "1/3");
    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Repair,
            target: INSTALL_LOCAL_CA_CERTIFICATE.into(),
        })
    );
}

#[test]
fn every_store_trusted_is_correct_without_action() {
    let detail = vec![
        a_store(StoreBrand::Firefox, true),
        a_store(StoreBrand::Chrome, true),
    ];

    let row = evaluate_local_ca_certificate_signal(detail, false);

    assert_eq!(row.value, "2/2");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn no_store_detected_at_all_is_incorrect() {
    let row = evaluate_local_ca_certificate_signal(Vec::new(), false);

    assert_eq!(row.value, "0/0");
    assert_eq!(row.verdict, Verdict::Incorrect);
}

#[test]
fn restart_firefox_notice_carries_through_when_asked() {
    let detail = vec![a_store(StoreBrand::Firefox, true)];

    let with_firefox_alive = evaluate_local_ca_certificate_signal(detail.clone(), true);
    let without_firefox_alive = evaluate_local_ca_certificate_signal(detail, false);

    assert!(with_firefox_alive.restart_firefox_notice);
    assert!(!without_firefox_alive.restart_firefox_notice);
}

#[test]
fn firefox_restart_notice_when_its_own_profile_starts_trusting_while_alive() {
    assert!(firefox_restart_notice(true, &[false], &[true]));
}

#[test]
fn no_firefox_restart_notice_when_firefox_was_not_running() {
    assert!(!firefox_restart_notice(false, &[false], &[true]));
}

#[test]
fn no_firefox_restart_notice_when_firefox_profile_was_already_trusted() {
    // Escenario del hallazgo: Firefox ya de confianza, solo otro almacén (p. ej. Chrome) recibe
    // la instalación mientras Firefox está abierto — no debe avisar de reiniciar Firefox.
    assert!(!firefox_restart_notice(true, &[true], &[true]));
}

#[test]
fn no_firefox_restart_notice_when_nothing_changed() {
    assert!(!firefox_restart_notice(true, &[false], &[false]));
}

#[test]
fn firefox_restart_notice_with_several_profiles_needs_only_one_to_flip() {
    assert!(firefox_restart_notice(true, &[true, false], &[true, true]));
}

fn a_url_handlers(
    available: bool,
    handlers: Vec<UrlHandler>,
    current: Option<&str>,
) -> UrlHandlers {
    UrlHandlers {
        available,
        handlers,
        current: current.map(str::to_owned),
        ours: OUR_DESKTOP_FILE.to_owned(),
    }
}

fn a_handler(id: &str, name: &str) -> UrlHandler {
    UrlHandler {
        id: id.to_owned(),
        name: name.to_owned(),
    }
}

fn site_signature_on_native(handlers: UrlHandlers) -> SignalRow {
    evaluate_site_signature_signal(handlers, Channel::Native)
}

#[test]
fn site_signature_is_not_applicable_when_the_sandbox_hides_the_registry() {
    let row =
        evaluate_site_signature_signal(a_url_handlers(false, Vec::new(), None), Channel::Flatpak);

    assert_eq!(row.signal, Signal::SiteSignature);
    assert_eq!(row.value, "");
    assert_eq!(row.verdict, Verdict::NotApplicable);
    assert_eq!(row.action, None);
    assert_eq!(row.candidates, None);
}

#[test]
fn site_signature_inside_the_sandbox_carries_the_flatpak_desktop_file_to_diagnose() {
    let row =
        evaluate_site_signature_signal(a_url_handlers(false, Vec::new(), None), Channel::Flatpak);

    assert_eq!(
        row.detail,
        Some(SignalDetail::HandlerDiagnosis {
            desktop_file: FLATPAK_DESKTOP_FILE.to_owned(),
        })
    );
}

#[test]
fn site_signature_unavailable_outside_the_sandbox_has_no_diagnosis() {
    let row = site_signature_on_native(a_url_handlers(false, Vec::new(), None));

    assert_eq!(row.verdict, Verdict::NotApplicable);
    assert_eq!(row.detail, None);
}

#[test]
fn site_signature_offers_to_use_rfirma_when_nothing_is_configured() {
    let row = site_signature_on_native(a_url_handlers(
        true,
        vec![a_handler("autofirma.desktop", "AutoFirma")],
        None,
    ));

    assert_eq!(row.signal, Signal::SiteSignature);
    assert_eq!(row.value, "");
    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Choice,
            target: OUR_DESKTOP_FILE.to_string(),
        })
    );
}

#[test]
fn site_signature_is_correct_and_offers_no_action_when_rfirma_is_the_current_handler() {
    let row = site_signature_on_native(a_url_handlers(
        true,
        vec![
            a_handler(OUR_DESKTOP_FILE, "rFirma"),
            a_handler("autofirma.desktop", "AutoFirma"),
        ],
        Some(OUR_DESKTOP_FILE),
    ));

    assert_eq!(row.signal, Signal::SiteSignature);
    assert_eq!(row.value, "rFirma");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn site_signature_needs_attention_and_offers_to_use_rfirma_when_another_program_is_current() {
    let row = site_signature_on_native(a_url_handlers(
        true,
        vec![
            a_handler(OUR_DESKTOP_FILE, "rFirma"),
            a_handler("autofirma.desktop", "AutoFirma"),
        ],
        Some("autofirma.desktop"),
    ));

    assert_eq!(row.signal, Signal::SiteSignature);
    assert_eq!(row.value, "AutoFirma");
    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Choice,
            target: OUR_DESKTOP_FILE.to_string(),
        })
    );
}

#[test]
fn site_signature_falls_back_to_the_id_when_the_current_handler_is_unlisted() {
    let row = site_signature_on_native(a_url_handlers(true, Vec::new(), Some("unknown.desktop")));

    assert_eq!(row.value, "unknown.desktop");
    assert_eq!(row.verdict, Verdict::Attention);
}

#[test]
fn site_signature_has_no_candidates_with_a_single_registered_handler() {
    let row = site_signature_on_native(a_url_handlers(
        true,
        vec![a_handler(OUR_DESKTOP_FILE, "rFirma")],
        Some(OUR_DESKTOP_FILE),
    ));

    assert_eq!(row.candidates, None);
}

#[test]
fn site_signature_lists_every_registered_handler_as_a_candidate_marking_the_current_one() {
    let row = site_signature_on_native(a_url_handlers(
        true,
        vec![
            a_handler(OUR_DESKTOP_FILE, "rFirma"),
            a_handler("autofirma.desktop", "AutoFirma"),
        ],
        Some("autofirma.desktop"),
    ));

    assert_eq!(
        row.candidates,
        Some(vec![
            SiteSignatureCandidate {
                id: OUR_DESKTOP_FILE.to_string(),
                name: "rFirma".to_string(),
                selected: false,
            },
            SiteSignatureCandidate {
                id: "autofirma.desktop".to_string(),
                name: "AutoFirma".to_string(),
                selected: true,
            },
        ])
    );
}
