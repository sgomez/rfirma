//! Elección del mecanismo de firma que ofrece la ranura y firma efectiva con la clave privada.

use cryptoki::mechanism::{Mechanism, MechanismType};
use cryptoki::object::{Attribute, AttributeType, KeyType};
use cryptoki::slot::Slot;
use cryptoki::{context::Pkcs11, session::Session};

use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::ecdsa;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;

use super::one_login::{self, Refused};
use super::session::{
    context, context_logged_in, logged_in, private_key, slot_of, the_store_is_really_there,
};

pub(super) fn sign_holding_the_turn(
    reference: &CertificateRef,
    secret: &ProtectedSecret,
    algorithm: SignatureAlgorithm,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    if let Some(cut) = one_login::cut_short(reference) {
        return Err(cut);
    }
    let store = reference.store();
    the_store_is_really_there(&store)?;
    let context = context(&store)?;
    let slot = slot_of(&context, reference.token_label())?;
    let offered = the_slot_offers(&context, slot, algorithm)?;
    let log_in = || logged_in(&context, slot, secret);
    let context_login = |session: &Session| context_logged_in(&context, slot, session, secret);
    let sign =
        |session: &Session| signed_in(session, reference, offered, algorithm, data, &context_login);

    if let Some(signature) = one_login::within(reference, secret, log_in, sign, Refused::CutsIt) {
        return signature;
    }
    let session = log_in()?;
    let signature = sign(&session);
    let _ = session.logout();
    signature
}

fn signed_in(
    session: &Session,
    reference: &CertificateRef,
    offered: Offered,
    algorithm: SignatureAlgorithm,
    data: &[u8],
    context_login: &dyn Fn(&Session) -> Result<(), TokenError>,
) -> Result<Vec<u8>, TokenError> {
    let key = private_key(session, reference)?;
    the_key_is_of_the_kind(session, key, algorithm)?;
    let (mechanism, bytes) = match offered {
        Offered::Composed => (algorithm.mechanism(), data.to_vec()),
        Offered::EcdsaOverTheDigest => (Mechanism::Ecdsa, ecdsa::digest(algorithm, data)?),
    };
    let signature = if the_key_always_authenticates(session, key)? {
        session.sign_init(&mechanism, key)?;
        context_login(session)?;
        session.sign_update(&bytes)?;
        session.sign_final()?
    } else {
        session.sign(&mechanism, key, &bytes)?
    };
    match algorithm.key_kind() {
        KeyKind::Ec => ecdsa::der_encoded(&signature),
        KeyKind::Rsa => Ok(signature),
    }
}

fn the_key_always_authenticates(
    session: &Session,
    key: cryptoki::object::ObjectHandle,
) -> Result<bool, TokenError> {
    Ok(session
        .get_attributes(key, &[AttributeType::AlwaysAuthenticate])?
        .into_iter()
        .any(|attribute| matches!(attribute, Attribute::AlwaysAuthenticate(true))))
}

/// Con qué mecanismo de la ranura se cumple el algoritmo, y sobre qué bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Offered {
    /// El mecanismo compuesto, que resume y firma los bytes tal cual.
    Composed,
    /// El mecanismo crudo de curva elíptica, que firma el resumen calculado aquí.
    EcdsaOverTheDigest,
}

/// El mecanismo del algoritmo, buscado en el listado de la ranura antes de pedir el PIN.
pub(super) fn the_slot_offers(
    context: &Pkcs11,
    slot: Slot,
    algorithm: SignatureAlgorithm,
) -> Result<Offered, TokenError> {
    let objection = match objection_to(context, slot, algorithm.mechanism_type())? {
        None => return Ok(Offered::Composed),
        Some(why) => why,
    };

    if algorithm.key_kind() == KeyKind::Ec
        && objection_to(context, slot, ecdsa::OVER_A_DIGEST)?.is_none()
    {
        return Ok(Offered::EcdsaOverTheDigest);
    }

    Err(mechanism_not_offered(algorithm, objection))
}

/// Por qué la ranura no firma con el mecanismo, o nada si firma.
fn objection_to(
    context: &Pkcs11,
    slot: Slot,
    wanted: MechanismType,
) -> Result<Option<&'static str>, TokenError> {
    if !context.get_mechanism_list(slot)?.contains(&wanted) {
        return Ok(Some("no esta entre los mecanismos de la ranura"));
    }

    if !context.get_mechanism_info(slot, wanted)?.sign() {
        return Ok(Some("la ranura lo ofrece sin la bandera CKF_SIGN"));
    }

    Ok(None)
}

fn the_key_is_of_the_kind(
    session: &Session,
    key: cryptoki::object::ObjectHandle,
    algorithm: SignatureAlgorithm,
) -> Result<(), TokenError> {
    let declared = session
        .get_attributes(key, &[AttributeType::KeyType])?
        .into_iter()
        .find_map(|attribute| match attribute {
            Attribute::KeyType(key_type) => Some(key_type),
            _ => None,
        });

    match declared {
        Some(key_type) if kind_of(key_type) == Some(algorithm.key_kind()) => Ok(()),
        Some(key_type) => Err(mechanism_not_offered(
            algorithm,
            &format!("la clave privada del certificado es {key_type}"),
        )),
        None => Ok(()),
    }
}

fn kind_of(key_type: KeyType) -> Option<KeyKind> {
    if key_type == KeyType::RSA {
        return Some(KeyKind::Rsa);
    }
    if key_type == KeyType::EC {
        return Some(KeyKind::Ec);
    }
    None
}

fn mechanism_not_offered(algorithm: SignatureAlgorithm, why: &str) -> TokenError {
    TokenError::new(
        Situation::MechanismNotOffered,
        format!(
            "el token no firma {} con {}: {why}",
            algorithm.name(),
            algorithm.mechanism_type()
        ),
    )
}
