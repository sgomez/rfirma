//! El algoritmo de firma que nombra la sede, reducido a la huella que pide.

/// La huella que pide la sede, sea cual sea el nombre con el que la escriba.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskedAlgorithm {
    /// `SHA256`, `SHA256withRSA` o `SHA256withECDSA`.
    Sha256,
    /// `SHA384`, `SHA384withRSA` o `SHA384withECDSA`.
    Sha384,
    /// `SHA512`, `SHA512withRSA` o `SHA512withECDSA`.
    Sha512,
}

/// Los nombres que la sede puede escribir en `algorithm`, con la huella que nombra cada uno.
pub const ACCEPTED_ALGORITHMS: [(&str, AskedAlgorithm); 9] = [
    ("sha256", AskedAlgorithm::Sha256),
    ("sha384", AskedAlgorithm::Sha384),
    ("sha512", AskedAlgorithm::Sha512),
    ("sha256withrsa", AskedAlgorithm::Sha256),
    ("sha384withrsa", AskedAlgorithm::Sha384),
    ("sha512withrsa", AskedAlgorithm::Sha512),
    ("sha256withecdsa", AskedAlgorithm::Sha256),
    ("sha384withecdsa", AskedAlgorithm::Sha384),
    ("sha512withecdsa", AskedAlgorithm::Sha512),
];

impl AskedAlgorithm {
    /// La huella que nombra ese `algorithm`, o nada si es de los que rFirma no firma.
    pub fn named(name: &str) -> Option<Self> {
        let asked = name.trim().to_ascii_lowercase();
        ACCEPTED_ALGORITHMS
            .into_iter()
            .find(|(accepted, _)| *accepted == asked)
            .map(|(_, algorithm)| algorithm)
    }

    /// El nombre de la huella, sin la clave con la que se compone el algoritmo.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sha256 => "SHA256",
            Self::Sha384 => "SHA384",
            Self::Sha512 => "SHA512",
        }
    }
}

#[cfg(test)]
mod tests;
