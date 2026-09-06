use super::*;

#[test]
fn the_profile_is_opened_read_write_and_in_sql_format() {
    let spec = read_write_spec(Path::new("/home/quien/.mozilla/firefox/perfil"));

    assert!(spec.contains("configDir='sql:/home/quien/.mozilla/firefox/perfil'"));
    assert!(spec.contains("flags=readWrite"));
}

#[test]
fn opening_someone_elses_profile_does_not_rename_their_token() {
    assert!(!read_write_spec(Path::new("/tmp/perfil")).contains("tokenDescription"));
}

#[test]
fn the_bits_that_come_back_from_the_softoken_still_read_as_trusted() {
    assert!(is_trusted_ssl_ca(0x38));
    assert!(!is_trusted_ssl_ca(0x08));
    assert!(!is_trusted_ssl_ca(0));
}

#[test]
fn the_local_ca_is_trusted_for_tls_and_for_nothing_else() {
    let trust = CertTrust {
        ssl: TRUSTED_SSL_CA,
        ..CertTrust::default()
    };

    assert_eq!(trust.ssl, 0x18);
    assert!(is_trusted_ssl_ca(trust.ssl));
    assert_eq!(trust.email, 0);
    assert_eq!(trust.object_signing, 0);
}
