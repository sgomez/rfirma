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

const PSS_OID: &str = "1.2.840.113549.1.1.10";

const PSS_JCA_NAMES: &[&str] = &[
    "RSASSA-PSS",
    "SHA256withRSAandMGF1",
    "SHA384withRSAandMGF1",
    "SHA512withRSAandMGF1",
];

const PSS_XMLDSIG_URI_MARKERS: &[&str] = &["rsa-pss", "rsa-MGF1"];

fn is_rsa_pss(name: &str) -> bool {
    PSS_JCA_NAMES.contains(&name)
        || name.ends_with(&format!("/{PSS_OID}"))
        || name.ends_with(&format!("with{PSS_OID}"))
        || (name.starts_with("http") && PSS_XMLDSIG_URI_MARKERS.iter().any(|m| name.ends_with(m)))
}

/// El nombre de las RFC y el OID de un nombre de algoritmo del puente, si la tabla lo conoce.
pub(super) fn rfc_name_and_oid(bridge_name: &str) -> Option<(&'static str, &'static str)> {
    if is_rsa_pss(bridge_name) {
        return Some(RSA_PSS);
    }
    KNOWN
        .iter()
        .find(|(jca, _, _)| *jca == bridge_name)
        .map(|(_, name, oid)| (*name, *oid))
}

#[cfg(test)]
mod tests;
