use std::path::PathBuf;

use windows_sys::Win32::Security::Cryptography::{
    CertUnregisterSystemStore, CERT_STORE_DELETE_FLAG, CERT_SYSTEM_STORE_CURRENT_USER,
};

use super::*;
use crate::site::domain::local_ca::LocalCa;

struct ScratchStore(PathBuf);

impl ScratchStore {
    fn new(tag: &str) -> Self {
        Self(PathBuf::from(format!(
            "{USER_STORE_PREFIX}rfirma-test-{tag}-{}",
            std::process::id()
        )))
    }
}

impl Drop for ScratchStore {
    fn drop(&mut self) {
        let name = user_store_name(&self.0).expect("un almacén de usuario");
        unsafe {
            CertUnregisterSystemStore(
                name.as_ptr().cast(),
                CERT_SYSTEM_STORE_CURRENT_USER | CERT_STORE_DELETE_FLAG,
            )
        };
    }
}

fn a_local_ca() -> Vec<u8> {
    LocalCa::generate()
        .expect("una CA local")
        .certificate()
        .to_der()
        .expect("en DER")
}

#[test]
fn the_trust_profile_is_the_user_root_store() {
    assert_eq!(
        trust_profiles(),
        vec![PathBuf::from("cryptoapi:CurrentUser/Root")]
    );
}

#[test]
fn only_user_store_paths_name_a_store() {
    assert!(user_store_name(Path::new("cryptoapi:CurrentUser/Root")).is_some());
    assert!(user_store_name(Path::new("cryptoapi:CurrentUser/")).is_none());
    assert!(user_store_name(Path::new("C:/Users/someone/firefox")).is_none());
}

#[test]
fn a_path_that_is_not_a_windows_store_is_unreachable() {
    let error = WindowsUserStores
        .trust_of(Path::new("C:/no/es/un/almacen"), &a_local_ca())
        .expect_err("no es un almacén de Windows");
    assert_eq!(error.situation(), Situation::StoreUnreachable);
}

#[test]
fn a_certificate_nobody_installed_is_not_in_the_root_store() {
    let reading = WindowsUserStores
        .trust_of(Path::new(SYSTEM_ROOT_STORE), &a_local_ca())
        .expect("el almacén raíz se lee sin avisos");
    assert_eq!(reading, None);
}

#[test]
fn withdrawing_what_is_not_in_the_root_store_does_nothing() {
    WindowsUserStores
        .withdraw(Path::new(SYSTEM_ROOT_STORE), &a_local_ca())
        .expect("retirar lo que no está no pregunta ni falla");
}

#[test]
fn install_trust_and_withdraw_round_trip_on_a_scratch_store() {
    let scratch = ScratchStore::new("round-trip");
    let der = a_local_ca();

    WindowsUserStores
        .install(&scratch.0, &der, "rFirma")
        .expect("se instala");
    assert_eq!(
        WindowsUserStores
            .trust_of(&scratch.0, &der)
            .expect("se lee"),
        Some(TRUSTED_SSL_CA)
    );
    WindowsUserStores
        .install(&scratch.0, &der, "rFirma")
        .expect("instalar dos veces no falla");

    WindowsUserStores
        .withdraw(&scratch.0, &der)
        .expect("se retira");
    assert_eq!(
        WindowsUserStores
            .trust_of(&scratch.0, &der)
            .expect("se lee"),
        None
    );
}
