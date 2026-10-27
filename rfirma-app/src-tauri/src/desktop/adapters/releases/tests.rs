use super::{endpoint_for, LATEST_RELEASE_ENDPOINT, MACOS_FEED_ENDPOINT, WINDOWS_FEED_ENDPOINT};
use crate::desktop::domain::channel::Channel;

#[test]
fn each_channel_asks_its_own_feed() {
    assert_eq!(endpoint_for(Channel::Windows), WINDOWS_FEED_ENDPOINT);
    assert_eq!(endpoint_for(Channel::MacOs), MACOS_FEED_ENDPOINT);
    assert_eq!(endpoint_for(Channel::Native), LATEST_RELEASE_ENDPOINT);
    assert_eq!(endpoint_for(Channel::Flatpak), LATEST_RELEASE_ENDPOINT);
    assert_eq!(
        WINDOWS_FEED_ENDPOINT,
        "https://rfirma.com/windows/latest.json"
    );
    assert_eq!(MACOS_FEED_ENDPOINT, "https://rfirma.com/macos/latest.json");
}

#[test]
fn it_asks_github_for_the_latest_release_and_nobody_else() {
    assert_eq!(
        LATEST_RELEASE_ENDPOINT, "https://api.github.com/repos/sgomez/rfirma/releases/latest",
        "se le pregunta a GitHub, no a rfirma.com (ADR-0015)"
    );
}

const WINDOWS_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packaging/windows/tauri.windows.json"
));
const MACOS_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packaging/macos/tauri.macos.json"
));

fn updater_of(config: &str) -> serde_json::Value {
    let config: serde_json::Value = serde_json::from_str(config).expect("es JSON");
    config["plugins"]["updater"].clone()
}

#[test]
fn the_installer_of_each_platform_asks_the_same_feed_as_the_version_check() {
    assert_eq!(
        updater_of(WINDOWS_CONFIG)["endpoints"],
        serde_json::json!([WINDOWS_FEED_ENDPOINT])
    );
    assert_eq!(
        updater_of(MACOS_CONFIG)["endpoints"],
        serde_json::json!([MACOS_FEED_ENDPOINT])
    );
}

#[test]
fn windows_and_macos_trust_the_same_update_key() {
    assert_eq!(
        updater_of(MACOS_CONFIG)["pubkey"],
        updater_of(WINDOWS_CONFIG)["pubkey"]
    );
}
