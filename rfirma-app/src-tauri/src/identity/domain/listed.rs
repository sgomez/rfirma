//! `Listed`, lo que dicen los almacenes al listarlos —los certificados de los que se abren y el error de los que no—, y por qué falta un certificado en un listado nuevo.

use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::store::Store;

/// Lo que da un almacén listado sin abrir las ranuras de unos lectores: sus certificados y si se ha saltado alguna.
#[derive(Clone, Debug, Default)]
pub struct ListedSkipping {
    pub certificates: Vec<TokenCertificate>,
    pub skipped_a_slot: bool,
}

impl ListedSkipping {
    /// Lo de un almacén que no se ha saltado ninguna ranura.
    pub fn of_every_slot(certificates: Vec<TokenCertificate>) -> Self {
        Self {
            certificates,
            skipped_a_slot: false,
        }
    }
}

/// Lo que han dicho los almacenes al listarlos, uno a uno.
#[derive(Clone, Debug, Default)]
pub struct Listed {
    found: Vec<TokenCertificate>,
    failed: Vec<(Store, TokenError)>,
    opened: usize,
}

impl Listed {
    /// Apunta lo que ha respondido el almacén: sus certificados o su error.
    pub fn heard(&mut self, store: &Store, answer: Result<Vec<TokenCertificate>, TokenError>) {
        match answer {
            Ok(certificates) => {
                self.opened += 1;
                self.found.extend(certificates);
            }
            Err(error) => self.failed.push((store.clone(), error)),
        }
    }

    /// El error de cada almacén que no se ha abierto, en el orden en que se listaron.
    pub fn failed(&self) -> &[(Store, TokenError)] {
        &self.failed
    }

    /// Los certificados de los almacenes que se han abierto; si no se ha abierto ninguno, el primer error.
    pub fn certificates(self) -> Result<Vec<TokenCertificate>, TokenError> {
        if self.opened > 0 {
            return Ok(self.found);
        }
        Err(self
            .failed
            .into_iter()
            .next()
            .map(|(_, error)| error)
            .unwrap_or_else(|| {
                TokenError::new(
                    Situation::ModuleNotFound,
                    "no hay ningun modulo PKCS#11 donde buscar certificados",
                )
            }))
    }
}

/// Por qué falta `wanted` en un listado nuevo: el error de su almacén, si falló, o que el token ya no lo tiene.
pub fn why_missing(failed: &[(Store, TokenError)], wanted: &CertificateRef) -> TokenError {
    failed
        .iter()
        .find(|(store, _)| *store == wanted.store())
        .map(|(_, error)| error.clone())
        .unwrap_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                format!("el token ya no tiene {}", wanted.label()),
            )
        })
}
