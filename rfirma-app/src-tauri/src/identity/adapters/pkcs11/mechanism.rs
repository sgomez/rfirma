//! Elección del mecanismo de firma que ofrece la ranura y firma efectiva con la clave privada.

use cryptoki::error::{Error, RvError};
use cryptoki::mechanism::{Mechanism, MechanismType};
use cryptoki::object::{Attribute, AttributeType, KeyType};
use cryptoki::session::UserType;
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;
use cryptoki::{context::Pkcs11, session::Session};

use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::ecdsa;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;

use super::session::{context, private_key, slot_of, the_store_is_really_there};

pub(super) fn sign_holding_the_turn(
    reference: &CertificateRef,
    secret: &ProtectedSecret,
    algorithm: SignatureAlgorithm,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    let store = reference.store();
    the_store_is_really_there(&store)?;
    let context = context(&store)?;
    let slot = slot_of(&context, reference.token_label())?;
    let offered = the_slot_offers(&context, slot, algorithm)?;
    let session = context.open_ro_session(slot)?;
    let pin = secret
        .as_str()
        .map_err(|_| TokenError::new(Situation::IncorrectPin, "el secreto no es UTF-8 valido"))?;

    match session.login(UserType::User, Some(&AuthPin::new(pin.into()))) {
        Ok(()) => {}
        // Si otra biblioteca del proceso ya autenticó el token, se reutiliza la sesión.
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => {}
        Err(other) => return Err(other.into()),
    }

    let signature = private_key(&session, reference)
        .and_then(|key| {
            the_key_is_of_the_kind(&session, key, algorithm)?;
            Ok(key)
        })
        .and_then(|key| match offered {
            Offered::Composed => session
                .sign(&algorithm.mechanism(), key, data)
                .map_err(TokenError::from),
            Offered::EcdsaOverTheDigest => session
                .sign(&Mechanism::Ecdsa, key, &ecdsa::digest(algorithm, data)?)
                .map_err(TokenError::from),
        })
        .and_then(|signature| match algorithm.key_kind() {
            KeyKind::Ec => ecdsa::der_encoded(&signature),
            KeyKind::Rsa => Ok(signature),
        });

    let _ = session.logout();

    signature
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
