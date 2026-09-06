//! Filtrado de certificados según los criterios solicitados por la sede.

use base64::Engine as _;

use crate::commands::Failure;
use crate::ffi::BridgeError;
use crate::memory::ListedCertificates;
use crate::pkcs11::{self, Store, TokenCertificate};
use crate::protocol::SiteFilter;

/// Interfaz para evaluar filtros de certificados contra el motor de filtrado.
pub trait FilterEngine {
    /// Devuelve los índices de los certificados que cumplen los criterios.
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError>;
}

/// Caso de uso: obtiene los certificados de los almacenes aceptados por la sede.
pub fn listing_the_site_accepts<E: FilterEngine>(
    engine: &E,
    stores: &[Store],
    filter: &SiteFilter,
) -> Result<Vec<TokenCertificate>, Failure> {
    let ours = pkcs11::list_certificates_across(stores)?;
    keep_what_the_site_accepts(engine, filter, ours)
}

/// Aplica el filtro de la sede a una lista de certificados ya filtrada localmente.
pub fn keep_what_the_site_accepts<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    certificates: Vec<TokenCertificate>,
) -> Result<Vec<TokenCertificate>, Failure> {
    let accepted = accepted_indexes(engine, filter, &certificates)?;

    Ok(certificates
        .into_iter()
        .enumerate()
        .filter(|(index, _)| accepted.contains(index))
        .map(|(_, certificate)| certificate)
        .collect())
}

/// Caso de uso: comprueba que el certificado seleccionado sigue siendo aceptado por la sede.
pub fn usable_certificate_for_the_site<'a, E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    certificates: &'a [TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
) -> Result<&'a TokenCertificate, Failure> {
    let chosen = super::certificates::usable_certificate(certificates, handle, listed)?;

    let only_this_one = std::slice::from_ref(chosen).to_vec();
    if accepted_indexes(engine, filter, &only_this_one)?.is_empty() {
        return Err(Failure::new(
            "certificateNotFound",
            format!(
                "la sede excluye {}: su filtro ya no lo acepta",
                chosen.reference().label()
            ),
        ));
    }

    Ok(chosen)
}

fn accepted_indexes<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    certificates: &[TokenCertificate],
) -> Result<Vec<usize>, Failure> {
    let accepted = engine.select(&filter.as_java_properties(), &to_der_payload(certificates))?;

    if let Some(out_of_range) = accepted.iter().find(|index| **index >= certificates.len()) {
        return Err(Failure::new(
            "bridgeFailed",
            format!("el motor de filtros ha devuelto el indice {out_of_range}"),
        ));
    }
    Ok(accepted)
}

fn to_der_payload(certificates: &[TokenCertificate]) -> String {
    certificates
        .iter()
        .map(|certificate| base64::engine::general_purpose::STANDARD.encode(certificate.der()))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests;
