//! La tarjeta en el lector, vista desde el sistema: si hay un DNIe, sin pedirle nada a la tarjeta.

use std::path::PathBuf;
use std::process::Command;

const THE_OPENSC_MODULES: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so",
    "/usr/lib/aarch64-linux-gnu/opensc-pkcs11.so",
    "/usr/lib64/pkcs11/opensc-pkcs11.so",
    "/usr/lib/opensc-pkcs11.so",
];

/// Si el OpenSC del sistema ve un DNIe en algún lector; `false` si no hay módulo ni `pkcs11-tool`.
pub(crate) fn a_dnie_is_in_the_reader() -> bool {
    let Some(module) = the_opensc_module() else {
        return false;
    };
    Command::new("pkcs11-tool")
        .arg("--module")
        .arg(module)
        .arg("--list-slots")
        .output()
        .map(|output| a_dnie_in(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or(false)
}

fn the_opensc_module() -> Option<PathBuf> {
    std::env::var_os("RFIRMA_OPENSC_MODULE")
        .map(PathBuf::from)
        .into_iter()
        .chain(THE_OPENSC_MODULES.iter().map(PathBuf::from))
        .find(|candidate| candidate.is_file())
}

/// Si el listado de ranuras de `pkcs11-tool` trae una cuyo token es un DNIe.
fn a_dnie_in(listing: &str) -> bool {
    listing.lines().any(|line| {
        line.split_once(':').is_some_and(|(key, label)| {
            key.trim() == "token label" && label.trim().to_lowercase().starts_with("dni electr")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_whose_token_is_a_dnie_is_a_dnie_in_the_reader() {
        let listing = "Available slots:\nSlot 0 (0x0): Lector CCID 00 00\n  token label        : DNI electrónico\n  token manufacturer : DGP-FNMT\n";

        assert!(a_dnie_in(listing));
    }

    #[test]
    fn a_reader_without_a_card_is_not() {
        let listing = "Available slots:\nSlot 0 (0x0): Lector CCID 00 00\n  (empty)\n";

        assert!(!a_dnie_in(listing));
    }

    #[test]
    fn another_card_is_not_a_dnie() {
        let listing = "Slot 0 (0x0): Lector\n  token label        : rfirma-test\n";

        assert!(!a_dnie_in(listing));
    }
}
