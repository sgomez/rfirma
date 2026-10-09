use super::ProtectedSecret;

#[test]
fn creates_from_bytes_and_reads_correctly() {
    let secret = ProtectedSecret::new(b"123456");
    assert_eq!(secret.as_bytes(), b"123456");
    assert_eq!(secret.as_str(), Ok("123456"));
    assert_eq!(secret.len(), 6);
    assert!(!secret.is_empty());
}

#[test]
fn creates_from_str() {
    let secret = ProtectedSecret::from_str("mi_clave_secreta");
    assert_eq!(secret.as_bytes(), b"mi_clave_secreta");
    assert_eq!(secret.expose_secret(), Ok("mi_clave_secreta"));
}

#[test]
fn empty_secret_is_handled() {
    let secret = ProtectedSecret::new(b"");
    assert_eq!(secret.len(), 0);
    assert!(secret.is_empty());
    assert!(!secret.is_locked());
}

#[test]
fn debug_representation_redacts_the_secret() {
    let secret = ProtectedSecret::from_str("clave_ultra_secreta");
    let debug_str = format!("{secret:?}");
    assert_eq!(debug_str, "ProtectedSecret([REDACTED])");
    assert!(!debug_str.contains("clave_ultra_secreta"));
}

#[test]
fn wipe_clears_memory_with_zeroes() {
    let mut secret = ProtectedSecret::new(b"supersecret");
    assert_eq!(secret.as_bytes(), b"supersecret");
    secret.wipe();
    assert!(secret.as_bytes().iter().all(|&b| b == 0));
}

#[test]
fn mlock_executes_without_panic() {
    let secret = ProtectedSecret::new(b"test_pin");
    let _ = secret.is_locked();
}

fn locked_kib() -> Option<usize> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("VmLck:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|n| n.parse().ok())
}

#[test]
fn dropping_the_secret_unlocks_its_memory() {
    let secret = ProtectedSecret::new(vec![7u8; 48 * 1024]);
    let Some(while_alive) = locked_kib().filter(|_| secret.is_locked()) else {
        return;
    };
    drop(secret);
    assert!(locked_kib().unwrap() + 40 <= while_alive);
}
