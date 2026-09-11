//! Puertos del contexto de identidad: el token, el almacén de los `.p12` instalados y el certificado recordado.

use std::path::Path;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::memory_error::MemoryError;

/// El token visto desde los casos de uso: lista, dice cómo pide el secreto, firma e importa un `.p12` (ADR-0001).
pub trait Token {
    /// Los certificados firmables de un almacén.
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError>;

    /// Todos los certificados de un almacén, también los que no firman: con ellos se completa una cadena.
    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError>;

    /// Cómo hay que pedirle el secreto al almacén del certificado.
    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError>;

    /// Comprueba que la ranura del certificado ofrece el mecanismo del algoritmo, sin pedir el secreto.
    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError>;

    /// Firma `data` con la clave privada que acompaña al certificado, con el algoritmo pedido.
    fn sign(
        &self,
        reference: &CertificateRef,
        pin: &str,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError>;

    /// Si el almacén del certificado acepta el secreto, sin firmar nada.
    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), TokenError>;

    /// Firma `data` con la clave privada que acompaña al certificado y el secreto protegido.
    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        let pin_str = secret.as_str().map_err(|_| {
            TokenError::new(Situation::IncorrectPin, "el secreto no es UTF-8 valido")
        })?;
        self.sign(reference, pin_str, algorithm, data)
    }

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

/// La carpeta donde vive cada `.p12` instalado, con sus permisos (ADR-0011).
pub trait InstalledFolder {
    /// Crea la carpeta del almacén recién instalado.
    fn make(&self, directory: &Path) -> Result<(), String>;

    /// Deja la ruta legible solo por su dueño.
    fn restrict_to_owner(&self, path: &Path);

    /// Borra la carpeta del almacén y todo lo que hubiera dentro.
    fn remove(&self, directory: &Path) -> Result<(), String>;
}

/// El certificado con el que se firmó la última vez, recordado entre sesiones (ADR-0010).
pub trait CertificateMemory {
    /// El certificado recordado, si lo hay.
    fn remembered_certificate(&self) -> Option<CertificateRef>;

    /// Apunta el certificado con el que se acaba de firmar, según permitan los interruptores.
    fn remember_the_certificate(&self, reference: &CertificateRef) -> Result<(), MemoryError>;

    /// Olvida el certificado recordado.
    fn forget_the_certificate(&self) -> Result<(), MemoryError>;
}
