//! Puertos del contexto de identidad: el token y la carga compartida de NSS.

use std::path::Path;

use libloading::Library;

use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{NssUnavailable, Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;

/// El token visto desde los casos de uso: lista, dice cómo pide el secreto, firma e importa un `.p12` (ADR-0001).
pub trait Token {
    /// Los certificados firmables de un almacén.
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError>;

    /// Cómo hay que pedirle el secreto al almacén del certificado.
    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError>;

    /// Firma `data` con la clave privada que acompaña al certificado.
    fn sign(
        &self,
        reference: &CertificateRef,
        pin: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError>;

    /// Importa un `.p12` a un almacén NSS nuevo en ese directorio y devuelve el almacén.
    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
    ) -> Result<Store, TokenError>;

    /// Los certificados de todos los almacenes: falla solo si ninguno se ha podido abrir.
    fn list_across(&self, stores: &[Store]) -> Result<Vec<TokenCertificate>, TokenError> {
        if stores.is_empty() {
            return Err(TokenError::new(
                Situation::ModuleNotFound,
                "no hay ningun modulo PKCS#11 donde buscar certificados",
            ));
        }

        let mut found = Vec::new();
        let mut any_loaded = false;
        let mut refused: Option<TokenError> = None;

        for store in stores {
            match self.list(store) {
                Ok(certificates) => {
                    any_loaded = true;
                    found.extend(certificates);
                }
                Err(error) => refused = refused.or(Some(error)),
            }
        }

        match refused {
            Some(error) if !any_loaded => Err(error),
            _ => Ok(found),
        }
    }
}

/// Puerto para interactuar con la biblioteca NSS y el turno global del token.
pub trait NssHost {
    /// Biblioteca `libnss3.so` del sistema cargada en memoria.
    fn library(&self) -> Result<&'static Library, NssUnavailable>;

    /// Ejecuta una operación bajo el turno global del token.
    fn with_token_turn<T>(&self, work: impl FnOnce() -> T) -> T;
}
