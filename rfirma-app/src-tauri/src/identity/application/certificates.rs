//! Listado, inspección y selección de certificados en tokens sin pedir PIN.

use std::collections::HashMap;
use std::path::Path;

use crate::documents::domain::handles::Handles;
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::chain::issuers_of;
use crate::identity::domain::copies::{copies_of_each_certificate, ChosenCopy};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::holder::{
    common_name_of, given_name_and_surname, holder_of, is_pseudonym, is_representative,
};
use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::SecretName;
use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::{
    prompted_until_accepted, CertificateMemory, InstalledFolder, Keyring, OriginWindow,
    PromptedError, SecretPromptRequest, SecretPrompter, Token,
};
use crate::memory_error::{MemoryError, Situation as StoreSituation};
use crate::signing::domain::layer2_text::masked_signer;
use crate::signing::domain::Language;

/// Los certificados del último listado, cada uno tras su asa.
pub type ListedCertificates = Handles<CertificateRef>;

/// Por qué un `.p12` no se ha podido instalar ni quitar (ADR-0011, ADR-0034).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallError {
    /// El token o el fichero han dicho que no.
    Token(TokenError),
    /// El almacén del `.p12` no se ha podido crear ni quitar del disco.
    Store(MemoryError),
    /// El llavero del escritorio no ha entregado el PIN del Almacén de rFirma.
    Keyring(KeyringError),
}

impl From<TokenError> for InstallError {
    fn from(error: TokenError) -> Self {
        Self::Token(error)
    }
}

impl From<KeyringError> for InstallError {
    fn from(error: KeyringError) -> Self {
        Self::Keyring(error)
    }
}

/// Certificados de los tokens conectados, ya como filas con su asa (ADR-0011).
pub fn listed_rows(
    token: &dyn Token,
    stores: &[Store],
    installed_dir: &Path,
    listed: &ListedCertificates,
    installed_copies: &ListedCertificates,
    memory: &dyn CertificateMemory,
) -> Result<Vec<ListedCertificate>, TokenError> {
    let found = token.list_across(stores)?;
    Ok(rows_of(
        found,
        installed_dir,
        listed,
        installed_copies,
        memory,
    ))
}

/// Los certificados de los almacenes, cada uno con los emisores que su propio almacén aporta.
pub fn certificates_with_their_chains(
    token: &dyn Token,
    stores: &[Store],
) -> Result<Vec<TokenCertificate>, TokenError> {
    let found = token.list_across(stores)?;
    let mut neighbours: HashMap<Store, Vec<TokenCertificate>> = HashMap::new();

    Ok(found
        .into_iter()
        .map(|certificate| {
            let store = certificate.reference().store();
            let in_the_store = neighbours
                .entry(store.clone())
                .or_insert_with(|| token.every_certificate(&store).unwrap_or_default());
            let issuers = issuers_of(&certificate, in_the_store);
            certificate.with_its_issuers(issuers)
        })
        .collect())
}

/// Cuántos certificados firmables propios tiene cada clase de almacén que tenga alguno.
pub fn certificates_by_class(
    token: &dyn Token,
    stores: &[Store],
    installed_dir: &Path,
) -> Vec<(StoreClass, usize)> {
    let mut counted: Vec<(StoreClass, usize)> = Vec::new();
    for store in stores {
        let found = token
            .list(store)
            .map_or(0, |certificates| certificates.len());
        if found == 0 {
            continue;
        }
        let class = store.class_under(installed_dir);
        match counted.iter_mut().find(|(seen, _)| *seen == class) {
            Some((_, certificates)) => *certificates += found,
            None => counted.push((class, found)),
        }
    }
    counted
}

/// Filas de un listado, una por certificado aunque esté en varios almacenes, con asas acuñadas.
pub fn rows_of(
    found: Vec<TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    installed_copies: &ListedCertificates,
    memory: &dyn CertificateMemory,
) -> Vec<ListedCertificate> {
    let remembered = memory.remembered_certificate();
    let rows: Vec<ChosenCopy> = copies_of_each_certificate(found)
        .into_iter()
        .map(|copies| {
            ChosenCopy::among(
                copies,
                |reference| reference.store().class_under(installed_dir),
                remembered.as_ref(),
            )
        })
        .collect();
    let handles = listed.replace(rows.iter().map(|row| row.certificate.reference().clone()));
    installed_copies.replace_paired(rows.iter().zip(&handles).filter_map(|(row, id)| {
        row.installed_reference
            .clone()
            .map(|reference| (id.clone(), reference))
    }));
    rows.into_iter()
        .zip(handles)
        .map(|(row, id)| listed_as(row, id))
        .collect()
}

fn listed_as(row: ChosenCopy, id: String) -> ListedCertificate {
    let certificate = row.certificate;
    let subject = certificate.subject();
    let (holder_name, id_number) = holder_of(subject.as_deref());
    let (given_name, surname) = given_name_and_surname(subject.as_deref());
    let organization_identifier = certificate.organization_identifier();
    let entity_name = is_representative(organization_identifier.as_deref(), &given_name, &surname)
        .then(|| certificate.organization_name())
        .flatten();
    ListedCertificate {
        id,
        label: certificate.reference().label().to_owned(),
        stamped_signer: masked_signer(&holder_name, is_pseudonym(subject.as_deref())),
        holder_name,
        given_name,
        surname,
        id_number,
        organization_identifier,
        entity_name,
        issuer: common_name_of(certificate.issuer().as_deref()),
        certificate_serial_number: certificate.serial_number().unwrap_or_default(),
        store: row.store,
        stores: row.stores,
        status: certificate.status(),
        remembered: row.remembered,
    }
}

/// El PIN con el que cifrar la instalación: se crea si el almacén es nuevo, nunca si ya existía (ADR-0034).
fn pin_for_installing(
    keyring: &dyn Keyring,
    already_existed: bool,
) -> Result<ProtectedSecret, KeyringError> {
    if already_existed {
        keyring.pin()
    } else {
        keyring.get_or_create_pin()
    }
}

/// Instala un PKCS#12 en el Almacén de rFirma, cifrado con el PIN del llavero (ADR-0034).
pub fn install_pkcs12(
    token: &dyn Token,
    folder: &dyn InstalledFolder,
    keyring: &dyn Keyring,
    installed_dir: &Path,
    pkcs12: &[u8],
    password: &str,
) -> Result<(), InstallError> {
    let already_existed = installed_dir.join("cert9.db").is_file();
    let pin = pin_for_installing(keyring, already_existed)?;
    validate_pkcs12_alone(token, folder, pkcs12, password)?;

    folder.make(installed_dir).map_err(|error| {
        InstallError::Store(MemoryError::new(
            StoreSituation::Unwritable,
            format!("no se ha podido crear el Almacen de rFirma: {error}"),
        ))
    })?;
    folder.restrict_to_owner(installed_dir);

    let installed = token.import_pkcs12(installed_dir, pkcs12, password, &pin);

    if let Err(error) = installed {
        if !already_existed {
            for file in ["cert9.db", "key4.db"] {
                folder.remove_file(&installed_dir.join(file));
            }
        }
        return Err(error.into());
    }

    for file in ["cert9.db", "key4.db"] {
        folder.restrict_to_owner(&installed_dir.join(file));
    }
    Ok(())
}

/// Importa el `.p12` en un almacén desechable para comprobarlo antes de tocar el Almacén de rFirma compartido.
fn validate_pkcs12_alone(
    token: &dyn Token,
    folder: &dyn InstalledFolder,
    pkcs12: &[u8],
    password: &str,
) -> Result<(), InstallError> {
    let staging = folder.staging_directory();
    folder.make(&staging).map_err(|error| {
        InstallError::Store(MemoryError::new(
            StoreSituation::Unwritable,
            format!("no se ha podido preparar un almacen temporal para comprobar el .p12: {error}"),
        ))
    })?;
    folder.restrict_to_owner(&staging);

    let staging_pin = ProtectedSecret::from_str("comprobacion-temporal-del-p12");
    let checked = token
        .import_pkcs12(&staging, pkcs12, password, &staging_pin)
        .and_then(|store| only_supported_keys(token, &store, &staging_pin));

    let _ = folder.remove(&staging);
    checked.map(|_| ()).map_err(InstallError::from)
}

/// Quién pide la contraseña del `.p12`, en qué idioma y sobre qué ventana.
pub struct PasswordPrompt<'a> {
    /// El diálogo del secreto.
    pub prompter: &'a dyn SecretPrompter,
    /// El idioma del diálogo.
    pub language: Language,
    /// La ventana que pidió instalar, sobre la que el diálogo se hace modal.
    pub origin_window: OriginWindow,
}

/// Pide la contraseña del `.p12` por el diálogo del secreto y lo instala, con reintentos hasta acertar o cancelar.
pub fn install_pkcs12_asking_its_password(
    token: &dyn Token,
    folder: &dyn InstalledFolder,
    keyring: &dyn Keyring,
    installed_dir: &Path,
    pkcs12: &[u8],
    file_name: &str,
    prompt: PasswordPrompt<'_>,
) -> Result<(), PromptedError<InstallError>> {
    let request = SecretPromptRequest {
        secret: SecretName::Pkcs12Password(file_name.to_string()),
        holder: None,
        language: prompt.language,
        incorrect_secret: false,
        origin_window: Some(prompt.origin_window),
    };
    prompted_until_accepted(
        prompt.prompter,
        request,
        |secret| {
            install_pkcs12(
                token,
                folder,
                keyring,
                installed_dir,
                pkcs12,
                secret.as_str().unwrap_or_default(),
            )
        },
        wrong_pkcs12_password,
    )
    .map(|_| ())
}

/// Un `.p12` no se bloquea: solo la contraseña incorrecta merece reintentarse.
fn wrong_pkcs12_password(error: &InstallError) -> bool {
    matches!(
        error,
        InstallError::Token(token) if token.situation() == Situation::IncorrectPkcs12Password
    )
}

/// Comprueba que el almacén contiene al menos un certificado y todas las claves son RSA o de curva elíptica.
fn only_supported_keys(
    token: &dyn Token,
    store: &Store,
    pin: &ProtectedSecret,
) -> Result<(), TokenError> {
    let found = token.list_authenticated(store, pin)?;
    if found.is_empty() {
        return Err(TokenError::new(
            Situation::Pkcs12NoPrivateKey,
            "el fichero no ha dejado ningun certificado con clave privada dentro",
        ));
    }
    for certificate in &found {
        if certificate.key_kind().is_none() {
            return Err(TokenError::new(
                Situation::KeyKindUnsupported,
                format!(
                    "{}: la clave no es RSA ni de curva eliptica",
                    certificate.reference().label()
                ),
            ));
        }
    }
    Ok(())
}

/// Quita un certificado del Almacén de rFirma: lo borra, con su clave, de la base única (ADR-0034).
pub fn remove_installed(
    token: &dyn Token,
    keyring: &dyn Keyring,
    memory: &dyn CertificateMemory,
    installed_dir: &Path,
    handle: &str,
    listed: &ListedCertificates,
    installed_copies: &ListedCertificates,
) -> Result<(), InstallError> {
    let reference = match installed_copies.get(handle) {
        Some(installed) => installed,
        None => listed.get(handle).ok_or_else(not_from_the_last_listing)?,
    };
    reference
        .store()
        .installed_directory_under(installed_dir)
        .ok_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                "ese certificado no viene de un .p12 instalado",
            )
        })?;
    let pin = keyring.pin()?;
    token.remove_certificate(installed_dir, &reference, &pin)?;
    if memory.remembered_certificate().as_ref() == Some(&reference) {
        let _ = memory.forget_the_certificate();
    }
    Ok(())
}

/// Vacía el Almacén de rFirma entero, a petición expresa de la persona tras perder su PIN (ADR-0034).
pub fn empty_the_store(
    folder: &dyn InstalledFolder,
    installed_dir: &Path,
) -> Result<(), InstallError> {
    folder.remove(installed_dir).map_err(|error| {
        InstallError::Store(MemoryError::new(
            StoreSituation::Unwritable,
            format!("no se ha podido vaciar el Almacen de rFirma: {error}"),
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

/// Olvida el certificado recordado entre sesiones.
pub fn forget_the_certificate(memory: &dyn CertificateMemory) {
    if memory.remembered_certificate().is_none() {
        return;
    }
    let _ = memory.forget_the_certificate();
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
