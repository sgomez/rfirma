use std::time::{Duration, SystemTime};

use super::*;
use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::status::{ActionKind, Signal, Verdict};
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
    let row = evaluate_version_signal(running, Some(running), false, Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn version_is_up_to_date_when_announced_is_older() {
    let running = Version::parse("0.4.1").unwrap();
    let older = Version::parse("0.3.9").unwrap();
    let row = evaluate_version_signal(running, Some(older), false, Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn version_is_up_to_date_when_no_version_announced() {
    let running = Version::parse("0.4.1").unwrap();
    let row = evaluate_version_signal(running, None, false, Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Correct);
    assert_eq!(row.action, None);
}

#[test]
fn version_requires_attention_when_newer_published_on_native_channel() {
    let running = Version::parse("0.4.1").unwrap();
    let newer = Version::parse("0.5.0").unwrap();
    let row = evaluate_version_signal(running, Some(newer), false, Channel::Native);

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
    let row = evaluate_version_signal(running, Some(newer), false, Channel::Flatpak);

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
fn version_is_checking_when_explicitly_set() {
    let running = Version::parse("0.4.1").unwrap();
    let row = evaluate_version_signal(running, None, true, Channel::Native);

    assert_eq!(row.signal, Signal::Version);
    assert_eq!(row.value, "0.4.1");
    assert_eq!(row.verdict, Verdict::Checking);
    assert_eq!(row.action, None);
}

#[test]
fn check_version_signal_with_expired_cache_starts_in_checking() {
    let home = tempfile::tempdir().expect("tempdir");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.1").unwrap();

    let row = check_version_signal(
        running,
        &memory,
        &|| Some(a_release("v0.5.0")),
        Channel::Native,
        false,
        at(1_000_000),
    );

    assert_eq!(row.verdict, Verdict::Checking);
    assert_eq!(row.value, "0.4.1");
}

#[test]
fn check_version_signal_with_fresh_cache_returns_cached_verdict() {
    let home = tempfile::tempdir().expect("tempdir");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.1").unwrap();

    let _ = memory.remember_version_check(VersionCheck {
        checked_at: 1_000_000,
        announced: "0.5.0".into(),
    });

    let row = check_version_signal(
        running,
        &memory,
        &|| None,
        Channel::Native,
        false,
        at(1_000_100),
    );

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
fn recheck_version_signal_queries_feed_and_updates_memory() {
    let home = tempfile::tempdir().expect("tempdir");
    let memory = a_memory(home.path());
    let running = Version::parse("0.4.1").unwrap();

    let row = check_version_signal(
        running,
        &memory,
        &|| Some(a_release("v0.5.0")),
        Channel::Flatpak,
        true,
        at(1_000_000),
    );

    assert_eq!(row.verdict, Verdict::Attention);
    assert_eq!(row.value, "0.4.1 → 0.5.0");
    assert_eq!(
        row.action,
        Some(StatusAction {
            kind: ActionKind::Link,
            target: "repository".into(),
        })
    );

    let cached = memory.last_version_check().expect("guardada en memoria");
    assert_eq!(cached.announced, "0.5.0");
}
