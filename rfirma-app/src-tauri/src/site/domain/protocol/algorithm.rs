//! El algoritmo de firma que nombra la sede, reducido a la huella que pide.

/// La huella que pide la sede, sea cual sea el nombre con el que la escriba.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskedAlgorithm {
    /// `SHA1`, `SHA1withRSA` o `SHA1withECDSA` (ADR-0023).
    Sha1,
    /// `SHA256`, `SHA256withRSA` o `SHA256withECDSA`.
    Sha256,
    /// `SHA384`, `SHA384withRSA` o `SHA384withECDSA`.
    Sha384,
    /// `SHA512`, `SHA512withRSA` o `SHA512withECDSA`.
    Sha512,
}

impl AskedAlgorithm {
    /// La huella que nombra ese `algorithm`, o nada si es de los que rFirma no firma.
    pub fn named(name: &str) -> Option<Self> {
        let asked = name.trim();
        if asked.is_empty() {
            return None;
        }
        normalize_asked_algorithm(asked)
    }

    /// El nombre de la huella, sin la clave con la que se compone el algoritmo.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha384 => "SHA384",
            Self::Sha512 => "SHA512",
        }
    }
}

fn normalize_asked_algorithm(candidate: &str) -> Option<AskedAlgorithm> {
    let s = candidate.trim();
    if s.is_empty() {
        return None;
    }
    let upper = s.to_ascii_uppercase();

    if is_sha1_alias(&upper) {
        return Some(AskedAlgorithm::Sha1);
    }
    if is_sha256_alias(&upper) {
        return Some(AskedAlgorithm::Sha256);
    }
    if is_sha384_alias(&upper) {
        return Some(AskedAlgorithm::Sha384);
    }
    if is_sha512_alias(&upper) {
        return Some(AskedAlgorithm::Sha512);
    }
    if is_ripemd160_alias(&upper) {
        return None;
    }
    if let Some(sub) = subname_before_with(s, &upper) {
        return normalize_asked_algorithm(sub);
    }
    if let Some(sub) = subname_from_uri(s, &upper) {
        return normalize_asked_algorithm(sub);
    }

    None
}

fn is_sha1_alias(upper: &str) -> bool {
    upper == "SHA"
        || upper == "HTTP://WWW.W3.ORG/2000/09/XMLDSIG#SHA1"
        || upper == "1.3.14.3.2.26"
        || upper.starts_with("SHA1")
        || upper.starts_with("SHA-1")
}

fn is_sha256_alias(upper: &str) -> bool {
    upper == "HTTP://WWW.W3.ORG/2001/04/XMLENC#SHA256"
        || upper == "2.16.840.1.101.3.4.2.1"
        || upper.starts_with("SHA256")
        || upper.starts_with("SHA-256")
}

fn is_sha384_alias(upper: &str) -> bool {
    upper == "HTTP://WWW.W3.ORG/2001/04/XMLENC#SHA384"
        || upper == "2.16.840.1.101.3.4.2.2"
        || upper.starts_with("SHA384")
        || upper.starts_with("SHA-384")
}

fn is_sha512_alias(upper: &str) -> bool {
    upper == "HTTP://WWW.W3.ORG/2001/04/XMLENC#SHA512"
        || upper == "2.16.840.1.101.3.4.2.3"
        || upper.starts_with("SHA512")
        || upper.starts_with("SHA-512")
}

fn is_ripemd160_alias(upper: &str) -> bool {
    upper == "HTTP://WWW.W3.ORG/2001/04/XMLENC#RIPEMD160"
        || upper.starts_with("RIPEMD160")
        || upper.starts_with("RIPEMD-160")
}

fn subname_before_with<'a>(s: &'a str, upper: &str) -> Option<&'a str> {
    let pos = upper.find("WITH")?;
    let sub = &s[..pos];
    (!sub.is_empty() && sub.len() < s.len()).then_some(sub)
}

fn subname_from_uri<'a>(s: &'a str, upper: &str) -> Option<&'a str> {
    const XMLDSIG_PREFIXES: [&str; 3] = [
        "HTTP://WWW.W3.ORG/2001/04/XMLDSIG-MORE#",
        "HTTP://WWW.W3.ORG/2000/09/XMLDSIG#",
        "HTTP://WWW.W3.ORG/2009/XMLDSIG11#",
    ];
    if !XMLDSIG_PREFIXES
        .iter()
        .any(|prefix| upper.starts_with(prefix))
    {
        return None;
    }
    let fragment = s.rsplit_once('#').map(|(_, f)| f).unwrap_or(s);
    let sub = fragment
        .rsplit_once('-')
        .map(|(_, f)| f)
        .unwrap_or(fragment);
    (!sub.is_empty() && sub.len() < s.len()).then_some(sub)
}

#[cfg(test)]
mod tests;
