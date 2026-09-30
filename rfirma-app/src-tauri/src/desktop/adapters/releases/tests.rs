use super::{endpoint_for, LATEST_RELEASE_ENDPOINT, WINDOWS_FEED_ENDPOINT};
use crate::desktop::domain::channel::Channel;

#[test]
fn each_channel_asks_its_own_feed() {
    assert_eq!(endpoint_for(Channel::Windows), WINDOWS_FEED_ENDPOINT);
    assert_eq!(endpoint_for(Channel::Native), LATEST_RELEASE_ENDPOINT);
    assert_eq!(endpoint_for(Channel::Flatpak), LATEST_RELEASE_ENDPOINT);
    assert_eq!(
        WINDOWS_FEED_ENDPOINT,
        "https://rfirma.sgomez.me/windows/latest.json"
    );
}

#[test]
fn it_asks_github_for_the_latest_release_and_nobody_else() {
    assert_eq!(
        LATEST_RELEASE_ENDPOINT, "https://api.github.com/repos/sgomez/rfirma/releases/latest",
        "se le pregunta a GitHub, no a rfirma.sgomez.me (ADR-0015)"
    );
}
