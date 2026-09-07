//! Listado, inspección y selección de certificados en tokens sin pedir PIN.

use std::path::Path;

use tauri_plugin_dialog::FilePath;

use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::holder::{holder_of, issuer_of};
use crate::identity::domain::store::Store;
use crate::identity::ports::{CertificateMemory, Token};
use crate::signing::domain::memory_error::{MemoryError, Situation as StoreSituation};

/// Por qué un `.p12` no se ha podido instalar ni quitar (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallError {
    /// El token o el fichero han dicho que no.
    Token(TokenError),
    /// El almacén del `.p12` no se ha podido crear ni quitar del disco.
    Store(MemoryError),
}

impl From<TokenError> for InstallError {
    fn from(error: TokenError) -> Self {
        Self::Token(error)
    }
}

/// Certificados de los tokens conectados, ya como filas con su asa (ADR-0011).
pub fn listed_rows(
    token: &dyn Token,
    stores: &[Store],
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &dyn CertificateMemory,
) -> Result<Vec<ListedCertificate>, TokenError> {
    let found = token.list_across(stores)?;
    Ok(rows_of(found, installed_dir, listed, memory))
}

/// Filas de un listado con asas acuñadas y estado de selección.
pub fn rows_of(
    found: Vec<TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &dyn CertificateMemory,
) -> Vec<ListedCertificate> {
    let remembered = memory.remembered_certificate();
    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );
    found
        .into_iter()
        .zip(handles)
        .map(|(certificate, id)| {
            let (holder_name, id_number) = holder_of(certificate.subject().as_deref());
            ListedCertificate {
                id,
                label: certificate.reference().label().to_owned(),
                holder_name,
                id_number,
                issuer: issuer_of(certificate.issuer().as_deref()),
                store: certificate.reference().store().class_under(installed_dir),
                status: certificate.status(),
                remembered: remembered
                    .as_ref()
                    .is_some_and(|one| one.is_the_same_as(certificate.reference())),
            }
        })
        .collect()
}

/// OID de rsaEncryption.
const RSA_ENCRYPTION: &str = "1.2.840.113549.1.1.1";

/// Instala un PKCS#12 importándolo a un almacén NSS aislado (ADR-0011).
pub fn install_pkcs12(
    token: &dyn Token,
    installed_dir: &Path,
    chosen: FilePath,
    password: &str,
) -> Result<(), InstallError> {
    let source = chosen
        .into_path()
        .map_err(|error| TokenError::new(Situation::Pkcs12Unreadable, error.to_string()))?;
    let pkcs12 = std::fs::read(&source)
        .map_err(|error| TokenError::new(Situation::Pkcs12Unreadable, error.to_string()))?;

    let directory = installed_dir.join(crate::documents::domain::handles::mint());
    std::fs::create_dir_all(&directory).map_err(|error| {
        InstallError::Store(MemoryError::new(
            StoreSituation::Unwritable,
            format!("no se ha podido crear el almacen del .p12: {error}"),
        ))
    })?;
    let _ = crate::desktop::adapters::paths::restrict_to_owner(&directory);

    let installed = token
        .import_pkcs12(&directory, &pkcs12, password)
        .and_then(|store| only_rsa_keys(token, &store));

    if let Err(error) = installed {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error.into());
    }

    for file in ["cert9.db", "key4.db"] {
        let _ = crate::desktop::adapters::paths::restrict_to_owner(&directory.join(file));
    }
    Ok(())
}

/// Comprueba que el almacén contiene al menos un certificado y todas las claves son RSA.
fn only_rsa_keys(token: &dyn Token, store: &Store) -> Result<(), TokenError> {
    let found = token.list(store)?;
    if found.is_empty() {
        return Err(TokenError::new(
            Situation::Pkcs12Unreadable,
            "el fichero no ha dejado ningun certificado con clave privada dentro",
        ));
    }
    for certificate in &found {
        if !is_rsa(certificate) {
            return Err(TokenError::new(
                Situation::KeyNotRsa,
                format!("{}: la clave no es RSA", certificate.reference().label()),
            ));
        }
    }
    Ok(())
}

/// Comprueba si la clave pública del certificado es RSA a partir de su DER.
fn is_rsa(certificate: &TokenCertificate) -> bool {
    use x509_cert::der::Decode;

    x509_cert::Certificate::from_der(certificate.der()).is_ok_and(|read| {
        read.tbs_certificate()
            .subject_public_key_info()
            .algorithm
            .oid
            .to_string()
            == RSA_ENCRYPTION
    })
}

/// Elimina el almacén correspondiente a un certificado PKCS#12 instalado (ADR-0011).
pub fn remove_installed(
    installed_dir: &Path,
    handle: &str,
    listed: &ListedCertificates,
) -> Result<(), InstallError> {
    let reference = listed.get(handle).ok_or_else(not_from_the_last_listing)?;
    let directory = reference
        .store()
        .installed_directory_under(installed_dir)
        .ok_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                "ese certificado no viene de un .p12 instalado",
            )
        })?;
    std::fs::remove_dir_all(&directory).map_err(|error| {
        InstallError::Store(MemoryError::new(
            StoreSituation::Unwritable,
            format!("no se ha podido quitar el almacen del .p12: {error}"),
        ))
    })
}

fn not_from_the_last_listing() -> TokenError {
    TokenError::new(
        Situation::CertificateNotFound,
        "el certificado elegido no es de la ultima busqueda",
    )
}

/// Guarda en el estado el certificado con el que se acaba de firmar.
pub fn remember_the_certificate(memory: &dyn CertificateMemory, reference: &CertificateRef) {
    if memory.remembered_certificate().as_ref() == Some(reference) {
        return;
    }
    let _ = memory.remember_the_certificate(reference);
}

/// Resuelve el certificado asociado a un asa en el listado actual.
pub fn certificate_behind<'a>(
    certificates: &'a [TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
) -> Result<&'a TokenCertificate, TokenError> {
    let wanted = listed.get(handle).ok_or_else(not_from_the_last_listing)?;
    certificates
        .iter()
        .find(|certificate| certificate.reference() == &wanted)
        .ok_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                format!("el token ya no tiene {}", wanted.label()),
            )
        })
}

/// Verifica la existencia y vigencia del certificado solicitado antes de firmar.
pub fn usable_certificate<'a>(
    certificates: &'a [TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
) -> Result<&'a TokenCertificate, TokenError> {
    let chosen = certificate_behind(certificates, handle, listed)?;
    let status = chosen.status();
    if !status.is_usable() {
        return Err(TokenError::new(
            Situation::CertificateNotFound,
            format!("{}: {status:?}", chosen.reference().label()),
        ));
    }
    Ok(chosen)
}

#[cfg(test)]
mod tests;
