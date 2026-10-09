//! La tarjeta en el lector, vista desde el sistema: si hay un DNIe, sin pedirle nada a la tarjeta.

use std::path::PathBuf;
use std::process::Command;

const THE_OPENSC_MODULES: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so",
    "/usr/lib/aarch64-linux-gnu/opensc-pkcs11.so",
    "/usr/lib64/pkcs11/opensc-pkcs11.so",
    "/usr/lib/opensc-pkcs11.so",
];

const THE_SMART_CARD_KEY_PROVIDER: &str = "Microsoft Smart Card Key Storage Provider";

/// Si hay un DNIe en el lector por la vía de la plataforma: el minidriver en Windows, OpenSC en el resto.
pub(crate) fn a_dnie_is_in_the_reader() -> bool {
    if std::env::consts::OS == "windows" {
        a_dnie_is_in_the_windows_store()
    } else {
        a_dnie_is_in_opensc()
    }
}

/// Si el KSP de tarjeta enseña ahora la clave de un certificado del DNIe del Almacén de Windows, que conserva las copias de las tarjetas que no están.
fn a_dnie_is_in_the_windows_store() -> bool {
    let card_keys = silent_certutil(&["-csp", THE_SMART_CARD_KEY_PROVIDER, "-key"]);
    a_dnie_certificate_in(&silent_certutil(&["-user", "-store", "My"]), &card_keys)
}

/// La salida de `certutil` con `-silent`, que no abre ninguna ventana de la tarjeta; vacía si no corre.
fn silent_certutil(arguments: &[&str]) -> String {
    Command::new("certutil")
        .arg("-silent")
        .args(arguments)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

/// Si el listado del almacén trae un certificado del DNIe cuya clave lista ahora el KSP de tarjeta.
fn a_dnie_certificate_in(store: &str, card_keys: &str) -> bool {
    store
        .split("================")
        .filter(|certificate| {
            certificate.contains(THE_SMART_CARD_KEY_PROVIDER)
                && certificate.to_lowercase().contains("dnie")
        })
        .any(|certificate| a_key_of(certificate, card_keys))
}

/// Si algún valor del certificado (`<etiqueta> = <valor>`, en el idioma del sistema) es una clave del listado del KSP.
fn a_key_of(certificate: &str, card_keys: &str) -> bool {
    certificate
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(_, value)| value.trim()))
        .filter(|value| !value.is_empty() && *value != THE_SMART_CARD_KEY_PROVIDER)
        .any(|value| card_keys.lines().any(|key| key.trim() == value))
}

/// Si el OpenSC del sistema ve un DNIe en algún lector; `false` si no hay módulo ni `pkcs11-tool`.
fn a_dnie_is_in_opensc() -> bool {
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

    const CARD_KEYS: &str =
        "Microsoft Smart Card Key Storage Provider:\n  dnie-firma-0001\n  RSA\n    AT_SIGNATURE\n";

    #[test]
    fn a_certificate_of_the_dnie_whose_key_the_card_shows_is_a_dnie_in_the_reader() {
        let store = "================ Certificate 0 ================\nIssuer: CN=AC DNIE 004\n  Key Container = dnie-firma-0001\n  Provider = Microsoft Smart Card Key Storage Provider\n";

        assert!(a_dnie_certificate_in(store, CARD_KEYS));
    }

    #[test]
    fn a_copy_of_the_dnie_in_the_store_without_the_card_is_not_a_dnie_in_the_reader() {
        let store = "================ Certificate 0 ================\nIssuer: CN=AC DNIE 004\n  Key Container = dnie-firma-0001\n  Provider = Microsoft Smart Card Key Storage Provider\n";

        assert!(!a_dnie_certificate_in(store, ""));
    }

    #[test]
    fn a_copy_of_the_dnie_while_another_card_is_in_is_not_a_dnie_in_the_reader() {
        let store = "================ Certificate 0 ================\nIssuer: CN=AC DNIE 004\n  Key Container = dnie-firma-0001\n  Provider = Microsoft Smart Card Key Storage Provider\n";
        let other_card = "Microsoft Smart Card Key Storage Provider:\n  otra-tarjeta-0002\n  RSA\n";

        assert!(!a_dnie_certificate_in(store, other_card));
    }

    #[test]
    fn a_certificate_of_another_card_is_not_a_dnie_in_the_reader() {
        let store = "================ Certificate 0 ================\nIssuer: CN=Otra CA\n  Key Container = dnie-firma-0001\n  Provider = Microsoft Smart Card Key Storage Provider\n";

        assert!(!a_dnie_certificate_in(store, CARD_KEYS));
    }

    #[test]
    fn a_dnie_certificate_with_a_software_key_is_not_a_dnie_in_the_reader() {
        let store = "================ Certificate 0 ================\nIssuer: CN=AC DNIE 004\n  Key Container = dnie-firma-0001\n  Provider = Microsoft Software Key Storage Provider\n";

        assert!(!a_dnie_certificate_in(store, CARD_KEYS));
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
