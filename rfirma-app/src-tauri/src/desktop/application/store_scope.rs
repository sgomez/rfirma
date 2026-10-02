//! Los certificados de los almacenes a los que acota `-store`; el módulo tiene que ser uno ya descubierto (ADR-0022).

use crate::desktop::domain::store_scope::StoreScope;
use crate::desktop::ports::CertificateStores;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::StoreClass;

/// Los certificados del ámbito, o el módulo nombrado que rFirma no ha descubierto.
pub fn within_the_scope(
    scope: &StoreScope,
    stores: &dyn CertificateStores,
) -> Result<Vec<TokenCertificate>, ScopeFailure> {
    let module = match scope {
        StoreScope::Module(named) => Some(
            stores
                .discovered_module(named)
                .ok_or_else(|| ScopeFailure::ModuleNotDiscovered(named.clone()))?,
        ),
        _ => None,
    };
    let certificates = stores.certificates().map_err(ScopeFailure::Token)?;
    Ok(match (scope, module) {
        (StoreScope::Nss, _) => certificates
            .into_iter()
            .filter(|certificate| certificate.reference().store().class() != StoreClass::Card)
            .collect(),
        (_, Some(module)) => certificates
            .into_iter()
            .filter(|certificate| certificate.reference().module() == module)
            .collect(),
        _ => certificates,
    })
}

/// Por qué no se han podido listar los certificados del ámbito.
#[derive(Debug)]
pub enum ScopeFailure {
    /// Ningún almacén se ha podido abrir.
    Token(crate::identity::domain::error::TokenError),
    /// `-store pkcs11:<ruta>` nombra un módulo que rFirma no ha descubierto.
    ModuleNotDiscovered(String),
}
