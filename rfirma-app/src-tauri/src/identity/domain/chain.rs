//! Los emisores que acompañan al firmante en la cadena de certificación de una firma.

use crate::identity::domain::certificate::TokenCertificate;

/// Los emisores del certificado que estén en su mismo almacén, del más cercano al más lejano y sin la raíz.
pub fn issuers_of(
    certificate: &TokenCertificate,
    in_the_store: &[TokenCertificate],
) -> Vec<Vec<u8>> {
    let mut issuers: Vec<Vec<u8>> = Vec::new();
    let mut wanted = certificate.issuer();

    while let Some(issuer) = the_one_named(wanted.as_deref(), in_the_store) {
        if is_self_issued(issuer) || issuer.der() == certificate.der() || already(&issuers, issuer)
        {
            break;
        }
        issuers.push(issuer.der().to_vec());
        wanted = issuer.issuer();
    }

    issuers
}

fn the_one_named<'a>(
    subject: Option<&str>,
    in_the_store: &'a [TokenCertificate],
) -> Option<&'a TokenCertificate> {
    let subject = subject?;
    in_the_store
        .iter()
        .find(|candidate| candidate.subject().as_deref() == Some(subject))
}

fn is_self_issued(certificate: &TokenCertificate) -> bool {
    certificate.subject() == certificate.issuer()
}

fn already(issuers: &[Vec<u8>], candidate: &TokenCertificate) -> bool {
    issuers.iter().any(|der| der == candidate.der())
}

#[cfg(test)]
mod tests;
