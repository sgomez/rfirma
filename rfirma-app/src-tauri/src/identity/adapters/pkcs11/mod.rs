//! Capa de acceso a tokens criptográficos y firma nativa PKCS#11: `RealToken`, el adaptador del puerto `Token` en Linux y macOS, y el que usa `WindowsToken` para los módulos PKCS#11 (ADR-0001).

mod listing;
mod mechanism;
pub mod nss;
mod one_login;
pub mod p11kit;
pub mod probe;
mod removal;
mod session;
pub mod stores;

use std::path::Path;
use std::sync::Mutex;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;
pub use nss::{NssHost, RealNssHost};
use one_login::Refused;
use session::{
    context, logged_in, pin_warning_of, slot_of, the_store_is_really_there,
    token_info_unless_locked,
};

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

    fn list_authenticated(
        &self,
        store: &Store,
        pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        with_token_turn(|| listing::list_authenticated(store, pin))
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        store_secret(reference)
    }

    fn pin_warning(&self, reference: &CertificateRef) -> Result<PinWarning, TokenError> {
        pin_warning(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), TokenError> {
        accepts_the_secret(reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        sign_with_secret(reference, secret, algorithm, data)
    }

    fn hold_one_login(&self, reference: &CertificateRef) {
        one_login::hold(reference);
    }

    fn release_the_login(&self, reference: &CertificateRef) {
        with_token_turn(|| one_login::release(reference));
    }

    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
        pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        let softoken = stores::softoken().ok_or_else(|| {
            TokenError::new(
                Situation::ModuleNotFound,
                "no esta libsoftokn3.so en ninguna de las rutas conocidas",
            )
        })?;
        with_token_turn(|| nss::import_pkcs12(directory, pkcs12, password, pin))?;
        Ok(Store::nss(&softoken, directory))
    }

    fn remove_certificate(
        &self,
        directory: &Path,
        reference: &CertificateRef,
        pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        with_token_turn(|| removal::remove_certificate(directory, reference, pin))
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
        let info = token_info_unless_locked(&context, slot)?;
        Ok(StoreSecret::of_token(
            info.login_required(),
            info.protected_authentication_path(),
        ))
    })
}

/// Lo que la tarjeta del certificado dice de sus intentos, leído ahora (ADR-0047).
pub fn pin_warning(reference: &CertificateRef) -> Result<PinWarning, TokenError> {
    with_token_turn(|| {
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        pin_warning_of(&context, slot)
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

/// Firma `data` con la clave privada que acompaña al certificado referenciado y el secreto protegido (ADR-0001).
pub fn sign_with_secret(
    reference: &CertificateRef,
    secret: &ProtectedSecret,
    algorithm: SignatureAlgorithm,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    with_token_turn(|| mechanism::sign_holding_the_turn(reference, secret, algorithm, data))
}

/// Comprueba el PIN abriendo y cerrando la sesión de la ranura del certificado.
pub fn accepts_the_secret(
    reference: &CertificateRef,
    secret: &crate::identity::domain::protected_secret::ProtectedSecret,
) -> Result<(), TokenError> {
    with_token_turn(|| {
        if let Some(cut) = one_login::cut_short(reference) {
            return Err(cut);
        }
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        let log_in = || logged_in(&context, slot, secret);
        if let Some(accepted) =
            one_login::within(reference, secret, log_in, |_| Ok(()), Refused::LeavesItOpen)
        {
            return accepted;
        }
        let session = log_in()?;
        let _ = session.logout();
        Ok(())
    })
}

/// Serializa operaciones contra el token en el proceso para evitar colisiones de sesión.
#[doc(hidden)]
pub fn with_token_turn<T>(operation: impl FnOnce() -> T) -> T {
    static TURN: Mutex<()> = Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}
