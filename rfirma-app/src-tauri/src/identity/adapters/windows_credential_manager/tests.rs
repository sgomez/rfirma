use windows_sys::Win32::Security::Credentials::CredDeleteW;

use super::*;

struct ScratchCredential(WindowsCredentialManager);

impl ScratchCredential {
    fn new(tag: &str) -> Self {
        Self(WindowsCredentialManager::at(&format!(
            "rfirma-test/{tag}-{}",
            std::process::id()
        )))
    }
}

impl Drop for ScratchCredential {
    fn drop(&mut self) {
        unsafe { CredDeleteW(self.0.target.as_ptr(), CRED_TYPE_GENERIC, 0) };
    }
}

#[test]
fn a_credential_manager_without_the_item_yet_has_no_pin() {
    let scratch = ScratchCredential::new("empty");

    assert_eq!(scratch.0.pin(), Err(KeyringError::PinMissing));
}

#[test]
fn create_pin_can_be_read_back() {
    let scratch = ScratchCredential::new("round-trip");

    let created = scratch.0.create_pin().expect("se guarda");

    assert_eq!(scratch.0.pin().expect("se lee"), created);
    assert_eq!(created.len(), 64);
}

#[test]
fn get_or_create_pin_creates_once_and_then_reads() {
    let scratch = ScratchCredential::new("once");

    let first = scratch.0.get_or_create_pin().expect("lo crea");
    let second = scratch.0.get_or_create_pin().expect("lo lee");

    assert_eq!(first, second);
}
