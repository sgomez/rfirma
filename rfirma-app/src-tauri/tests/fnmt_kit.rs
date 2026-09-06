//! Guardia del kit de certificados de prueba FNMT en `testdata/fnmt/` (ADR-0014).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// URL de descarga de STCERES para renovación del kit.
const STCERES: &str =
    "https://www.sede.fnmt.gob.es/documents/10445900/10649507/Certificados_pruebas_todas_CAs.rar";

/// `notAfter` de `active-rsa.p12` en segundos desde la época UNIX.
const ACTIVE_EXPIRY_EPOCH: u64 = 1_856_513_219;
const ACTIVE_EXPIRY_ISO: &str = "2028-10-30";

/// Huellas SHA-256 de los ficheros `.p12` versionados.
const FINGERPRINTS: [(&str, &str); 3] = [
    (
        "active-rsa.p12",
        "6e0cad97b78be2918ed54a64a0dd4f3f6e4c16e01b405ef0836fb91b77a3ffb4",
    ),
    (
        "revoked-rsa.p12",
        "a8ff78c1a7b13bcdc12347f683dd5395b6e0ac1d9c3cad23e3668823ae2b1425",
    ),
    (
        "expired-rsa.p12",
        "901df49ac10cceb0524c8cb50833d1407d0974f42f9d45a5b4b71c0eefa4e91f",
    ),
];

fn kit_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/fnmt")
        .canonicalize()
        .expect("falta testdata/fnmt/ en el repositorio")
}

fn fingerprint(path: &Path) -> String {
    let bytes =
        fs::read(path).unwrap_or_else(|e| panic!("no se puede leer {}: {e}", path.display()));
    let mut h = Sha256::new();
    h.update(&bytes);
    format!("{:x}", h.finalize())
}

#[test]
fn all_three_kit_p12_files_are_the_expected_ones() {
    let dir = kit_dir();
    for (name, expected) in FINGERPRINTS {
        let path = dir.join(name);
        assert!(
            path.is_file(),
            "falta testdata/fnmt/{name}. El kit se descarga de {STCERES} \
             (ver testdata/fnmt/README.md)."
        );
        assert_eq!(
            fingerprint(&path),
            expected,
            "testdata/fnmt/{name} no es el fichero que documenta \
             testdata/fnmt/README.md. Si lo has renovado, actualiza la huella \
             y, en active-rsa.p12, tambien ACTIVE_EXPIRY_EPOCH."
        );
    }
}

#[test]
fn active_rsa_has_not_expired_yet() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("el reloj del sistema esta antes de 1970")
        .as_secs();

    assert!(
        now < ACTIVE_EXPIRY_EPOCH,
        "testdata/fnmt/active-rsa.p12 CADUCO el {ACTIVE_EXPIRY_ISO}. Ya no sirve \
         como camino feliz y todas las pruebas que dependan de el mienten. \
         Descarga el kit nuevo de {STCERES}, sustituye los .p12, y actualiza las \
         huellas y las fechas de testdata/fnmt/README.md y de este fichero."
    );
}
