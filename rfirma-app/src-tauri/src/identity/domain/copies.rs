//! Las copias de un mismo certificado repartidas por varios almacenes, y la que firma por todas; no sabe de asas ni de ventanas.

use std::collections::HashMap;

use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::store::StoreClass;

/// Agrupa por emisor y número de serie; un certificado ilegible no se agrupa con nada.
pub fn copies_of_each_certificate(found: Vec<TokenCertificate>) -> Vec<Vec<TokenCertificate>> {
    let mut groups: Vec<Vec<TokenCertificate>> = Vec::new();
    let mut group_of: HashMap<(Vec<u8>, Vec<u8>), usize> = HashMap::new();
    for certificate in found {
        match certificate.issuer_and_serial() {
            Some(identity) => match group_of.get(&identity) {
                Some(&group) => groups[group].push(certificate),
                None => {
                    group_of.insert(identity, groups.len());
                    groups.push(vec![certificate]);
                }
            },
            None => groups.push(vec![certificate]),
        }
    }
    groups
}

/// La copia de un certificado con la que se firma, y los almacenes donde están todas.
pub struct ChosenCopy {
    pub certificate: TokenCertificate,
    pub store: StoreClass,
    /// Las clases de almacén de todas las copias, por orden de preferencia.
    pub stores: Vec<StoreClass>,
    pub remembered: bool,
    /// La copia instalada del mismo certificado, si hay una entre las copias.
    pub installed_reference: Option<CertificateRef>,
}

impl ChosenCopy {
    /// La recordada si está entre las copias; si no, la primera por preferencia de almacén.
    pub fn among(
        mut copies: Vec<TokenCertificate>,
        class_of: impl Fn(&CertificateRef) -> StoreClass,
        remembered: Option<&CertificateRef>,
    ) -> Self {
        let classes: Vec<StoreClass> = copies
            .iter()
            .map(|copy| class_of(copy.reference()))
            .collect();
        let installed_reference = copies
            .iter()
            .zip(&classes)
            .find(|(_, class)| **class == StoreClass::Installed)
            .map(|(copy, _)| copy.reference().clone());
        let remembered_copy = remembered.and_then(|one| {
            copies
                .iter()
                .position(|copy| one.is_the_same_as(copy.reference()))
        });
        let chosen = remembered_copy.unwrap_or_else(|| {
            (0..classes.len())
                .min_by_key(|&copy| classes[copy].preference())
                .unwrap_or_default()
        });
        let store = classes[chosen];
        let mut stores = classes;
        stores.sort_by_key(|class| class.preference());
        stores.dedup();
        Self {
            certificate: copies.swap_remove(chosen),
            store,
            stores,
            remembered: remembered_copy.is_some(),
            installed_reference,
        }
    }
}
