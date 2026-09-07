use super::*;

// Fixtures congeladas contra `CypherDataManager`/`DesCipher` de clienteafirma 1.9.2 (tag
// `v1.9.2`), compilados a mano contra `afirma-core-1.9.2.jar` de `~/.m2`. Se regeneran con:
//
//   mkdir -p /tmp/fixgen/src/es/gob/afirma/standalone/crypto
//   cp <checkout de clienteafirma>/afirma-simple/src/main/java/es/gob/afirma/standalone/crypto/{CypherDataManager,DesCipher}.java \
//     /tmp/fixgen/src/es/gob/afirma/standalone/crypto/
//   # y un Main.java desechable que llama a CypherDataManager.cipherData/decipherData
//   javac -cp ~/.m2/repository/es/gob/afirma/afirma-core/1.9.2/afirma-core-1.9.2.jar \
//     -d /tmp/fixgen/out /tmp/fixgen/src/es/gob/afirma/standalone/crypto/*.java /tmp/fixgen/src/Main.java
//   java -cp "/tmp/fixgen/out:~/.m2/repository/es/gob/afirma/afirma-core/1.9.2/afirma-core-1.9.2.jar" Main
//
// El generador no se versiona (CLAUDE.md: sin duplicar código del original).

const KEY: &str = "12345678";
const OTHER_KEY: &str = "ABCDEFGH";

fn key_of(key: &str) -> CipherKey {
    CipherKey::from_url_parameter(key)
        .expect("clave de 8 bytes valida")
        .expect("clave no vacia")
}

fn plain_of(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex valido"))
        .collect()
}

#[test]
fn ciphers_each_padding_remainder_byte_for_byte_like_the_original() {
    let cases: [(&str, &str); 5] = [
        ("", "0."),
        ("41", "7.ll1z6q7OTXQ="),
        ("31323334353637", "1.GVm6kyoyvrQ="),
        ("3132333435363738", "0.ltACiHjVjIk="),
        ("313233343536373839", "7.ltACiHjVjInHM_oGzgChKw=="),
    ];
    let key = key_of(KEY);

    for (plain_hex, expected) in cases {
        let data = plain_of(plain_hex);
        assert_eq!(cipher(&data, &key), expected, "cifrando {plain_hex}");
    }
}

#[test]
fn ciphers_utf8_and_binary_data_matching_the_frozen_fixtures() {
    let key = key_of(KEY);

    let utf8 = "El rápido zorro marrón salta sobre el perro perezoso. ñÑ".as_bytes();
    assert_eq!(
        cipher(utf8, &key),
        "4.KCGh8P-7AQYcSpQKo6DQ2JqywWkj_Pc0NOYMuyTiZwyvKG11_qxCsrLcsV9YEZ8NT8f3yweiuPMsk9T5owmYJA=="
    );

    let binary =
        plain_of("87aec3c226f2633e7cb241c8767583d7d5da76ba09db10282e681a0df3286bad0ca24151a8");
    assert_eq!(
        cipher(&binary, &key),
        "3.jP5_7AmO4oq6WfXzFhzjdh0DnAWxbnMw1AdD5KDTr29uZNn5hX1J9A=="
    );
}

#[test]
fn ciphers_with_a_second_key_matching_the_frozen_fixture() {
    let key = key_of(OTHER_KEY);
    let data = "contenido de prueba".as_bytes();

    assert_eq!(cipher(data, &key), "5.zx8gMBf4gk__NvdeUdqlkPARe6ds85p3");
}

#[test]
fn deciphers_the_frozen_fixtures_back_to_the_original_bytes() {
    let cases: [(&str, &str); 5] = [
        ("0.", ""),
        ("7.ll1z6q7OTXQ=", "41"),
        ("1.GVm6kyoyvrQ=", "31323334353637"),
        ("0.ltACiHjVjIk=", "3132333435363738"),
        ("7.ltACiHjVjInHM_oGzgChKw==", "313233343536373839"),
    ];
    let key = key_of(KEY);

    for (ciphered, expected_hex) in cases {
        let deciphered = decipher(ciphered, Some(&key)).expect("descifra");
        assert_eq!(deciphered, plain_of(expected_hex), "descifrando {ciphered}");
    }
}

#[test]
fn deciphers_without_a_key_as_plain_base_64() {
    let ciphered = URL_SAFE.encode(b"sin cifrar");

    let deciphered = decipher(&ciphered, None).expect("descifra sin clave");

    assert_eq!(deciphered, b"sin cifrar");
}

#[test]
fn a_key_of_the_right_length_is_accepted() {
    assert!(CipherKey::from_url_parameter("12345678")
        .expect("longitud correcta")
        .is_some());
}

#[test]
fn an_absent_key_means_no_encryption() {
    assert!(CipherKey::from_url_parameter("")
        .expect("clave ausente no es un error")
        .is_none());
}

#[test]
fn a_key_of_the_wrong_length_is_rejected() {
    let too_short = CipherKey::from_url_parameter("1234567");
    let too_long = CipherKey::from_url_parameter("123456789");

    assert!(too_short.is_err());
    assert!(too_long.is_err());
}

#[test]
fn a_malformed_padding_prefix_fails_without_panicking() {
    let key = key_of(KEY);

    let result = decipher("no-es-un-numero.QUJDREVGR0g=", Some(&key));

    assert!(matches!(
        result,
        Err(error) if error.situation() == Situation::DecryptionFailed
    ));
}

#[test]
fn corrupted_ciphertext_fails_without_panicking() {
    let key = key_of(KEY);

    let result = decipher("0.***no-es-base64***", Some(&key));

    assert!(matches!(
        result,
        Err(error) if error.situation() == Situation::DecryptionFailed
    ));
}

#[test]
fn ciphertext_not_a_multiple_of_the_block_size_fails_without_panicking() {
    let key = key_of(KEY);
    // Un solo byte cifrado no es múltiplo de ocho.
    let too_short = URL_SAFE.encode([0u8]);

    let result = decipher(&format!("0.{too_short}"), Some(&key));

    assert!(matches!(
        result,
        Err(error) if error.situation() == Situation::DecryptionFailed
    ));
}

#[test]
fn nothing_prints_the_key_itself() {
    let key = key_of(KEY);
    let error = RelayError::new(Situation::DecryptionFailed, "detalle sin la clave");

    let formatted = format!("{key:?} {error:?} {error}");

    assert!(!formatted.contains(KEY));
}
