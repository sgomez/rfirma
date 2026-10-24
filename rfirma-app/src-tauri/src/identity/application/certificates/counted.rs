//! La cuenta de certificados por clase de almacén del panel de estado: el almacén que sabe contar sus titulares no lee ninguno.

use std::path::Path;

use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::Token;

/// Cuántos certificados firmables propios tiene cada clase de almacén que tenga alguno.
pub fn certificates_by_class(
    token: &dyn Token,
    stores: &[Store],
    installed_dir: &Path,
) -> Vec<(StoreClass, usize)> {
    let mut counted: Vec<(StoreClass, usize)> = Vec::new();
    let mut listed_stores = Vec::new();
    for store in stores {
        match token.holders_without_reading(store) {
            Some(holders) => {
                count_in(
                    &mut counted,
                    store.class_under(installed_dir),
                    holders.unwrap_or(0),
                );
            }
            None => listed_stores.push(store.clone()),
        }
    }
    let found = token
        .list_each(&listed_stores)
        .certificates()
        .unwrap_or_default();
    for certificate in found
        .iter()
        .filter(|certificate| !certificate.is_a_macos_token_identity())
    {
        count_in(
            &mut counted,
            certificate.reference().store().class_under(installed_dir),
            1,
        );
    }
    counted
}

fn count_in(counted: &mut Vec<(StoreClass, usize)>, class: StoreClass, certificates: usize) {
    if certificates == 0 {
        return;
    }
    match counted.iter_mut().find(|(seen, _)| *seen == class) {
        Some((_, already)) => *already += certificates,
        None => counted.push((class, certificates)),
    }
}
