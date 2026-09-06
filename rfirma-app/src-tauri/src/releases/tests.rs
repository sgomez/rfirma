use super::LATEST_RELEASE_ENDPOINT;

#[test]
fn it_asks_github_for_the_latest_release_and_nobody_else() {
    assert_eq!(
        LATEST_RELEASE_ENDPOINT, "https://api.github.com/repos/sgomez/rfirma/releases/latest",
        "se le pregunta a GitHub, no a rfirma.sgomez.me (ADR-0015)"
    );
}
