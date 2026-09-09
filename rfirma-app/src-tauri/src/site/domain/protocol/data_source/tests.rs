use super::*;

#[test]
fn an_http_value_and_an_https_value_are_downloads() {
    assert_eq!(
        download_url("http://sede.example/4711.pdf"),
        Some("http://sede.example/4711.pdf")
    );
    assert_eq!(
        download_url("https://sede.example/4711.pdf"),
        Some("https://sede.example/4711.pdf")
    );
}

#[test]
fn the_value_is_trimmed_before_looking_at_its_scheme() {
    assert_eq!(
        download_url("  https://sede.example/4711.pdf  "),
        Some("https://sede.example/4711.pdf")
    );
}

#[test]
fn any_other_scheme_is_data_and_not_a_download() {
    assert_eq!(download_url("ftp://sede.example/4711.pdf"), None);
    assert_eq!(download_url("data:application/pdf;base64,JVBERi0="), None);
    assert_eq!(download_url("file:/etc/passwd"), None);
    assert_eq!(download_url("JVBERi0xLjcK"), None);
}
