//! La traducción del nombre JCA del algoritmo de una firma al nombre de las RFC y su OID.

const RSA_PSS: (&str, &str) = ("rsassaPss", "1.2.840.113549.1.1.10");

const KNOWN: &[(&str, &str, &str)] = &[
    (
        "SHA256withRSA",
        "sha256WithRSAEncryption",
        "1.2.840.113549.1.1.11",
    ),
    (
        "SHA384withRSA",
        "sha384WithRSAEncryption",
        "1.2.840.113549.1.1.12",
    ),
    (
        "SHA512withRSA",
        "sha512WithRSAEncryption",
        "1.2.840.113549.1.1.13",
    ),
    (
        "SHA256withECDSA",
        "ecdsa-with-SHA256",
        "1.2.840.10045.4.3.2",
    ),
    (
        "SHA384withECDSA",
        "ecdsa-with-SHA384",
        "1.2.840.10045.4.3.3",
    ),
    (
        "SHA512withECDSA",
        "ecdsa-with-SHA512",
        "1.2.840.10045.4.3.4",
    ),
];

const PSS_JCA_NAMES: &[&str] = &[
    "RSASSA-PSS",
    "SHA256withRSAandMGF1",
    "SHA384withRSAandMGF1",
    "SHA512withRSAandMGF1",
];

/// El nombre de las RFC y el OID de un nombre JCA, si la tabla lo conoce.
pub(super) fn rfc_name_and_oid(jca_name: &str) -> Option<(&'static str, &'static str)> {
    if PSS_JCA_NAMES.contains(&jca_name) {
        return Some(RSA_PSS);
    }
    KNOWN
        .iter()
        .find(|(jca, _, _)| *jca == jca_name)
        .map(|(_, name, oid)| (*name, *oid))
}

#[cfg(test)]
mod tests;
