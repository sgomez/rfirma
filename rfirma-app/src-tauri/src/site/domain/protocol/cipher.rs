//! Cifrado DES del servidor intermedio, calcado de `CypherDataManager` de 1.9.2.

use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine;
use cipher::block_padding::NoPadding;
use cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyInit};
use des::Des;

use crate::site::domain::relay_error::{RelayError, Situation};

const BLOCK_SIZE: usize = 8;
const REQUIRED_KEY_LENGTH: usize = 8;
const PADDING_SEPARATOR: char = '.';

/// Clave de cifrado DES derivada del parámetro `key` de la URL; nunca se imprime ni se registra.
#[derive(Clone, PartialEq, Eq)]
pub struct CipherKey([u8; REQUIRED_KEY_LENGTH]);

impl CipherKey {
    /// Deriva la clave desde el parámetro `key` de la URL, o `None` si la sede no pidió cifrado.
    pub fn from_url_parameter(key: &str) -> Result<Option<Self>, RelayError> {
        if key.is_empty() {
            return Ok(None);
        }
        let bytes = key.as_bytes();
        if bytes.len() != REQUIRED_KEY_LENGTH {
            return Err(RelayError::new(
                Situation::DecryptionFailed,
                "la longitud de la clave de cifrado no es correcta",
            ));
        }
        let mut buffer = [0u8; REQUIRED_KEY_LENGTH];
        buffer.copy_from_slice(bytes);
        Ok(Some(Self(buffer)))
    }
}

impl std::fmt::Debug for CipherKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CipherKey(***)")
    }
}

impl Drop for CipherKey {
    fn drop(&mut self) {
        self.0 = [0u8; REQUIRED_KEY_LENGTH];
    }
}

/// Cifra el original en el formato de `CypherDataManager.cipherData`: `<n>.` y el Base64 URL-safe.
pub fn cipher(data: &[u8], key: &CipherKey) -> String {
    let padding = (BLOCK_SIZE - data.len() % BLOCK_SIZE) % BLOCK_SIZE;
    let mut padded = data.to_vec();
    padded.resize(data.len() + padding, 0);

    let mut ciphered = vec![0u8; padded.len()];
    ecb::Encryptor::<Des>::new(&key.0.into())
        .encrypt_padded_b2b::<NoPadding>(&padded, &mut ciphered)
        .expect("los datos ya vienen rellenados a un multiplo del tamaño de bloque");

    format!("{padding}{PADDING_SEPARATOR}{}", URL_SAFE.encode(ciphered))
}

/// Descifra lo que llegó del servlet, como `CypherDataManager.decipherData`: sin clave, solo Base64.
pub fn decipher(ciphered: &str, key: Option<&CipherKey>) -> Result<Vec<u8>, RelayError> {
    let recovered = ciphered.replace('_', "/").replace('-', "+");

    let Some(key) = key else {
        return STANDARD
            .decode(&recovered)
            .map_err(|error| RelayError::new(Situation::DecryptionFailed, error.to_string()));
    };

    let (padding, body) = split_padding_prefix(&recovered)?;
    let ciphertext = STANDARD
        .decode(body)
        .map_err(|error| RelayError::new(Situation::DecryptionFailed, error.to_string()))?;

    let mut plain = vec![0u8; ciphertext.len()];
    let plain = ecb::Decryptor::<Des>::new(&key.0.into())
        .decrypt_padded_b2b::<NoPadding>(&ciphertext, &mut plain)
        .map_err(|_| {
            RelayError::new(
                Situation::DecryptionFailed,
                "los datos cifrados no son multiplo del tamaño de bloque",
            )
        })?;

    if padding > plain.len() {
        return Err(RelayError::new(
            Situation::DecryptionFailed,
            "el relleno declarado excede los datos descifrados",
        ));
    }
    Ok(plain[..plain.len() - padding].to_vec())
}

fn split_padding_prefix(text: &str) -> Result<(usize, &str), RelayError> {
    match text.find(PADDING_SEPARATOR) {
        Some(dot) => {
            let padding = text[..dot].parse().map_err(|_| {
                RelayError::new(
                    Situation::DecryptionFailed,
                    "prefijo de relleno mal formado",
                )
            })?;
            Ok((padding, &text[dot + 1..]))
        }
        None => Ok((0, text)),
    }
}

#[cfg(test)]
mod tests;
