//! Guardia del banco de referencia CAdES/XAdES/FacturaE en `testdata/reference/`.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Huellas SHA-256 de las fixtures versionadas, producidas por
/// `rfirma-native-bridge/testbench/make-reference-signatures.sh`.
const FINGERPRINTS: [(&str, &str); 15] = [
    (
        "challenge.bin",
        "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108",
    ),
    (
        "document.xml",
        "7c788bc27125428db2e89d42db1e76ea7be578e45fd30cd9bc3e2c162e39a1b7",
    ),
    (
        "invoice.xml",
        "23920413ea938634e1284a3b2d17a74ca4c42a7e0076c855abd1cf536268ec97",
    ),
    (
        "cades-implicit.p7s",
        "22e4f8d6d974e6ede8bd8697eff367ba4dfae702acdff6e21ed13a7b6d2952bd",
    ),
    (
        "cades-explicit.p7s",
        "9b9af08c7fd56b5301e982c0afcd251bb72124ce505de70035b58d6fe3aaae09",
    ),
    (
        "cades-implicit.cosign.p7s",
        "42fda13b54ccd3baa6777af185f4d576d25adfad97f6b8856c240a12139d822a",
    ),
    (
        "cades-implicit.countersign-tree.p7s",
        "3684b388f54c0d25e3ad8b239c7b1596b55d61ff6bb7b45a4a48771ed148b606",
    ),
    (
        "cades-implicit.countersign-leafs.p7s",
        "c6815317ae456e6bb0e5db0ef6d130fa4acb6859aed2ff4a4455b0a568312fc4",
    ),
    (
        "xades-detached.xml",
        "c0bfcd52ef46290f69d3dd682dfedeb104fd5225c4a3331945b595e07892fc82",
    ),
    (
        "xades-enveloping.xml",
        "233ebc1bd7a49b289f2416925bb4eae2c2bcc2aae65521409166ccf63adab230",
    ),
    (
        "xades-enveloped.xml",
        "318befb9fcedcabd5e8fc56d7c0d7c5418f94f01e5276930c9a3cc1b8bbe06d2",
    ),
    (
        "xades-enveloping.cosign.xml",
        "b6f236eec5d8ea9b20eb0cad4e1976a320a8cdea3cca5a95ef83a28fedbcfc44",
    ),
    (
        "xades-enveloping.countersign-tree.xml",
        "da3cee75d5677febdd3a22d251f983a4b662ed6f2ebcdb41affb0946d948560b",
    ),
    (
        "xades-enveloping.countersign-leafs.xml",
        "013ab8090cf0d54d37804ad0fb388b889b61b939077e9d9b22078c9016d3e1b5",
    ),
    (
        "facturae.xsig",
        "8a4dfb3d761637830868e3c78a53164b00622fe3b96eb6a1b6a3d90c3c0b280e",
    ),
];

fn reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/reference")
        .canonicalize()
        .expect("falta testdata/reference/ en el repositorio")
}

fn fingerprint(path: &Path) -> String {
    let bytes =
        fs::read(path).unwrap_or_else(|e| panic!("no se puede leer {}: {e}", path.display()));
    let mut h = Sha256::new();
    h.update(&bytes);
    format!("{:x}", h.finalize())
}

#[test]
fn all_reference_signature_fixtures_are_the_expected_ones() {
    let dir = reference_dir();
    for (name, expected) in FINGERPRINTS {
        let path = dir.join(name);
        assert!(
            path.is_file(),
            "falta testdata/reference/{name}. Regeneralo con \
             './rfirma-native-bridge/testbench/make-reference-signatures.sh'."
        );
        assert_eq!(
            fingerprint(&path),
            expected,
            "testdata/reference/{name} no es la fixture que este fichero documenta. Si lo has \
             regenerado a proposito, actualiza la huella aqui."
        );
    }
}
