//! Listado de certificados en las ranuras de un almacén, con o sin filtro de clave privada.

use std::collections::HashSet;

use cryptoki::error::{Error, RvError};
use cryptoki::object::{Attribute, AttributeType, ObjectClass};
use cryptoki::session::{Session, UserType};

use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::store::{Store, StoreClass};

use super::session::{context, the_store_is_really_there, usable_slots};

pub(super) fn list_holding_the_turn(store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
    the_store_is_really_there(store)?;
    let context = context(store)?;
    let mut found = Vec::new();

    for slot in usable_slots(&context)? {
        let info = context.get_token_info(slot)?;
        let token_label = info.label().trim().to_owned();
        let session = context.open_ro_session(slot)?;

        let logged_in = log_in_before_listing(&session, &info);
        found.extend(signable_certificates(
            &session,
            store,
            &token_label,
            logged_in,
        )?);
        if logged_in {
            let _ = session.logout();
        }
    }

    Ok(found)
}

/// Inicia sesión automáticamente antes de listar si la ranura lo requiere.
fn log_in_before_listing(session: &Session, info: &cryptoki::slot::TokenInfo) -> bool {
    if !should_attempt_blind_login(info.login_required(), info.protected_authentication_path()) {
        return false;
    }

    match session.login(UserType::User, None) {
        Ok(()) => true,
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => false,
        Err(_) => false,
    }
}

/// Determina si procede intentar un inicio de sesión ciego para listar.
fn should_attempt_blind_login(login_required: bool, protected_authentication_path: bool) -> bool {
    login_required && !protected_authentication_path
}

/// Filtra certificados de una ranura conservando aquellos con clave privada emparejada.
fn signable_certificates(
    session: &Session,
    store: &Store,
    token_label: &str,
    logged_in: bool,
) -> Result<Vec<TokenCertificate>, TokenError> {
    if store.class() == StoreClass::Card && !logged_in {
        // Sin sesión no hay clave privada que emparejar: se filtra por contenido (ADR-0025).
        return Ok(all_certificates_in_session(session, store, token_label)?
            .into_iter()
            .filter(|certificate| !certificate.cannot_sign_by_content())
            .collect());
    }

    let visible_private_keys = private_key_ids(session)?;

    let mut found = Vec::new();
    for certificate in all_certificates_in_session(session, store, token_label)? {
        if certificate
            .reference()
            .cka_id()
            .is_some_and(|cka_id| visible_private_keys.contains(cka_id))
        {
            found.push(certificate);
        }
    }

    Ok(found)
}

/// Los `CKA_ID` de las claves privadas visibles en la sesión.
fn private_key_ids(session: &Session) -> Result<HashSet<Vec<u8>>, TokenError> {
    let mut ids = HashSet::new();

    for object in session.find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)])? {
        let Ok(attributes) = session.get_attributes(object, &[AttributeType::Id]) else {
            continue;
        };

        for attribute in attributes {
            if let Attribute::Id(bytes) = attribute {
                if !bytes.is_empty() {
                    ids.insert(bytes);
                }
            }
        }
    }

    Ok(ids)
}

/// Certificados de una ranura abierta sin aplicar filtro de clave privada.
fn all_certificates_in_session(
    session: &Session,
    store: &Store,
    token_label: &str,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let mut found = Vec::new();

    for object in session.find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])? {
        let attributes = session.get_attributes(
            object,
            &[
                AttributeType::Label,
                AttributeType::Value,
                AttributeType::Id,
            ],
        )?;

        let mut label = None;
        let mut der = None;
        let mut cka_id = None;
        for attribute in attributes {
            match attribute {
                Attribute::Label(bytes) => {
                    label = Some(String::from_utf8_lossy(&bytes).trim().to_owned())
                }
                Attribute::Value(bytes) => der = Some(bytes),
                Attribute::Id(bytes) if !bytes.is_empty() => cka_id = Some(bytes),
                _ => {}
            }
        }

        if let (Some(label), Some(der)) = (label, der) {
            if !label.is_empty() {
                found.push(TokenCertificate::new(
                    CertificateRef::new(store, token_label, label, cka_id),
                    der,
                ));
            }
        }
    }

    Ok(found)
}

/// Los certificados de un almacén sin filtrar por clave privada: también las autoridades que lo emitieron.
pub(super) fn list_every_certificate(
    store: impl Into<Store>,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    the_store_is_really_there(&store)?;
    let context = context(&store)?;
    let mut found = Vec::new();

    for slot in usable_slots(&context)? {
        let info = context.get_token_info(slot)?;
        let token_label = info.label().trim().to_owned();
        let session = context.open_ro_session(slot)?;

        let logged_in = log_in_before_listing(&session, &info);
        found.extend(all_certificates_in_session(&session, &store, &token_label)?);
        if logged_in {
            let _ = session.logout();
        }
    }

    Ok(found)
}

#[cfg(test)]
mod log_in_before_listing_tests {
    use super::should_attempt_blind_login;

    #[test]
    fn skips_a_slot_that_does_not_require_login() {
        assert!(!should_attempt_blind_login(false, false));
    }

    #[test]
    fn attempts_a_blind_login_on_a_slot_without_a_reader_keypad() {
        assert!(should_attempt_blind_login(true, false));
    }

    #[test]
    fn skips_a_slot_with_a_reader_keypad_even_if_login_is_required() {
        assert!(!should_attempt_blind_login(true, true));
    }
}
