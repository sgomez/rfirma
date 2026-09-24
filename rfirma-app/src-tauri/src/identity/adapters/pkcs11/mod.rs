//! Capa de acceso a tokens criptográficos y firma nativa PKCS#11 (ADR-0001).

mod listing;
mod mechanism;
pub mod nss;
pub mod p11kit;
mod session;
pub mod stores;

use std::path::Path;
use std::sync::Mutex;

use cryptoki::error::{Error, RvError};
use cryptoki::session::UserType;
use cryptoki::types::AuthPin;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;
pub use nss::{NssHost, RealNssHost};
use session::{context, slot_of, the_store_is_really_there};

/// El adaptador del puerto [`Token`] sobre los módulos PKCS#11 del sistema.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealToken;

impl Token for RealToken {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        list_certificates(store)
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        list_every_certificate(store.clone())
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        store_secret(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        offers(reference, algorithm)
    }

    fn sign(
        &self,
        reference: &CertificateRef,
        pin: &str,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        sign(reference, pin, algorithm, data)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), TokenError> {
        accepts_the_secret(reference, secret)
    }

    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
    ) -> Result<Store, TokenError> {
        let softoken = stores::softoken().ok_or_else(|| {
            TokenError::new(
                Situation::ModuleNotFound,
                "no esta libsoftokn3.so en ninguna de las rutas conocidas",
            )
        })?;
        with_token_turn(|| nss::import_pkcs12(directory, pkcs12, password))?;
        Ok(Store::nss(&softoken, directory))
    }
}

/// Lista los certificados presentes en todos los almacenes indicados.
pub fn list_certificates_across(stores: &[Store]) -> Result<Vec<TokenCertificate>, TokenError> {
    RealToken.list_across(stores)
}

/// Lista los certificados firmables disponibles en el almacén indicado.
pub fn list_certificates(store: impl Into<Store>) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    with_token_turn(|| listing::list_holding_the_turn(&store))
}

/// Los certificados de un almacén sin filtrar por clave privada: también las autoridades que lo emitieron.
pub fn list_every_certificate(
    store: impl Into<Store>,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    with_token_turn(|| listing::list_every_certificate(store))
}

/// Cómo hay que pedirle el secreto al almacén leyendo las banderas de su ranura.
pub fn store_secret(reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
    with_token_turn(|| {
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        let info = context.get_token_info(slot)?;
        Ok(StoreSecret::of_token(
            info.login_required(),
            info.protected_authentication_path(),
        ))
    })
}

/// Comprueba en el listado de mecanismos de la ranura que el algoritmo se puede cumplir.
pub fn offers(reference: &CertificateRef, algorithm: SignatureAlgorithm) -> Result<(), TokenError> {
    with_token_turn(|| {
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        mechanism::the_slot_offers(&context, slot, algorithm).map(|_| ())
    })
}

/// Firma `data` con la clave privada que acompaña al certificado referenciado.
pub fn sign(
    reference: &CertificateRef,
    pin: &str,
    algorithm: SignatureAlgorithm,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    with_token_turn(|| mechanism::sign_holding_the_turn(reference, pin, algorithm, data))
}

/// Comprueba el PIN abriendo y cerrando la sesión de la ranura del certificado.
pub fn accepts_the_secret(
    reference: &CertificateRef,
    secret: &crate::identity::domain::protected_secret::ProtectedSecret,
) -> Result<(), TokenError> {
    let pin = secret
        .as_str()
        .map_err(|_| TokenError::new(Situation::IncorrectPin, "el secreto no es UTF-8 valido"))?;
    with_token_turn(|| {
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        let session = context.open_ro_session(slot)?;
        match session.login(UserType::User, Some(&AuthPin::new(pin.into()))) {
            Ok(()) => {
                let _ = session.logout();
                Ok(())
            }
            Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => Ok(()),
            Err(other) => Err(other.into()),
        }
    })
}

/// Serializa operaciones contra el token en el proceso para evitar colisiones de sesión.
#[doc(hidden)]
pub fn with_token_turn<T>(operation: impl FnOnce() -> T) -> T {
    static TURN: Mutex<()> = Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}
