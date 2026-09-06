//! Listado, inspección y selección de certificados en tokens sin pedir PIN.

use std::path::Path;

use tauri_plugin_dialog::FilePath;

use crate::commands::Failure;
use crate::identity::adapters::pkcs11;
use crate::identity::adapters::views::{store_name, CertificateView};
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::store::Store;
use crate::signing::application::configuration_memory::Configuration;
use crate::Memory;

/// Certificados de los tokens conectados clasificados como filas para la vista (ADR-0011).
pub fn listed_rows(
    stores: &[Store],
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
) -> Result<Vec<CertificateView>, Failure> {
    let found = pkcs11::list_certificates_across(stores)?;
    Ok(rows_of(found, installed_dir, listed, memory))
}

/// Filas de un listado con asas acuñadas y estado de selección.
pub fn rows_of(
    found: Vec<TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
) -> Vec<CertificateView> {
    let remembered = remembered_certificate(memory);
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
            CertificateView {
                id,
                label: certificate.reference().label().to_owned(),
                holder_name,
                id_number,
                issuer: issuer_of(certificate.issuer().as_deref()),
                store: store_name(certificate.reference().store().class_under(installed_dir))
                    .to_owned(),
                status: certificate.status().into(),
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
    installed_dir: &Path,
    chosen: FilePath,
    password: &str,
) -> Result<(), Failure> {
    let softoken = pkcs11::stores::softoken().ok_or_else(|| {
        Failure::new(
            "moduleNotFound",
            "no esta libsoftokn3.so en ninguna de las rutas conocidas",
        )
    })?;

    let source = chosen
        .into_path()
        .map_err(|error| Failure::new("pkcs12Unreadable", error.to_string()))?;
    let pkcs12 = std::fs::read(&source)
        .map_err(|error| Failure::new("pkcs12Unreadable", error.to_string()))?;

    let directory = installed_dir.join(crate::documents::domain::handles::mint());
    std::fs::create_dir_all(&directory).map_err(|error| {
        Failure::new(
            "settingsUnwritable",
            format!("no se ha podido crear el almacen del .p12: {error}"),
        )
    })?;
    let _ = crate::desktop::adapters::paths::restrict_to_owner(&directory);

    let store = pkcs11::Store::nss(&softoken, &directory);
    let installed =
        pkcs11::with_token_turn(|| pkcs11::nss::import_pkcs12(&directory, &pkcs12, password))
            .and_then(|()| only_rsa_keys(&store));

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
fn only_rsa_keys(store: &pkcs11::Store) -> Result<(), pkcs11::TokenError> {
    let found = pkcs11::list_certificates(store)?;
    if found.is_empty() {
        return Err(pkcs11::TokenError::new(
            pkcs11::Situation::Pkcs12Unreadable,
            "el fichero no ha dejado ningun certificado con clave privada dentro",
        ));
    }
    for certificate in &found {
        if !is_rsa(certificate) {
            return Err(pkcs11::TokenError::new(
                pkcs11::Situation::KeyNotRsa,
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
) -> Result<(), Failure> {
    let reference = listed.get(handle).ok_or_else(|| {
        Failure::new(
            "certificateNotFound",
            "el certificado elegido no es de la ultima busqueda",
        )
    })?;
    let directory = reference
        .store()
        .installed_directory_under(installed_dir)
        .ok_or_else(|| {
            Failure::new(
                "certificateNotFound",
                "ese certificado no viene de un .p12 instalado",
            )
        })?;
    std::fs::remove_dir_all(&directory).map_err(|error| {
        Failure::new(
            "settingsUnwritable",
            format!("no se ha podido quitar el almacen del .p12: {error}"),
        )
    })
}

/// El certificado recordado de la sesión anterior si existe en el estado.
pub fn remembered_certificate(memory: &Memory) -> Option<CertificateRef> {
    memory.state().ok()?.into_value().certificate
}

/// Guarda en el estado el certificado con el que se acaba de firmar.
pub fn remember_the_certificate(
    memory: &Memory,
    configuration: &Configuration,
    reference: &CertificateRef,
) {
    let Ok(loaded) = memory.state() else {
        return;
    };
    let mut state = loaded.into_value();
    if state.certificate.as_ref() == Some(reference) {
        return;
    }
    state.certificate = Some(reference.clone());
    let _ = memory.remember_state(configuration, &state);
}

/// Datos del titular del certificado elegido requeridos para la firma visible.
pub fn stamped_holder_named(
    handle: &str,
    stores: &[Store],
    listed: &ListedCertificates,
) -> Result<StampedHolder, Failure> {
    let certificates = pkcs11::list_certificates_across(stores)?;
    let chosen = certificate_behind(&certificates, handle, listed)?;
    Ok(stamped_holder_of(chosen))
}

/// Resuelve el certificado asociado a un asa en el listado actual.
pub fn certificate_behind<'a>(
    certificates: &'a [TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
) -> Result<&'a TokenCertificate, Failure> {
    let wanted = listed.get(handle).ok_or_else(|| {
        Failure::new(
            "certificateNotFound",
            "el certificado elegido no es de la ultima busqueda",
        )
    })?;
    certificates
        .iter()
        .find(|certificate| certificate.reference() == &wanted)
        .ok_or_else(|| {
            Failure::new(
                "certificateNotFound",
                format!("el token ya no tiene {}", wanted.label()),
            )
        })
}

/// Verifica la existencia y vigencia del certificado solicitado antes de firmar.
pub fn usable_certificate<'a>(
    certificates: &'a [TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
) -> Result<&'a TokenCertificate, Failure> {
    let chosen = certificate_behind(certificates, handle, listed)?;
    let status = chosen.status();
    if !status.is_usable() {
        return Err(Failure::new(
            "certificateNotFound",
            format!("{}: {status:?}", chosen.reference().label()),
        ));
    }
    Ok(chosen)
}

/// Pares atributo=valor de un nombre distinguido respetando comas escapadas (RFC 4514).
fn attribute_pairs(distinguished_name: &str) -> Vec<String> {
    let mut pairs = Vec::new();
    let mut start = 0;
    let bytes = distinguished_name.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b',' && !comma_is_escaped(bytes, index) {
            pairs.push(unescape(&distinguished_name[start..index]));
            start = index + 1;
        }
    }
    pairs.push(unescape(&distinguished_name[start..]));
    pairs
}

/// Comprueba si la coma en `index` está precedida por un número impar de barras invertidas.
fn comma_is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut backslashes = 0;
    while index > backslashes && bytes[index - 1 - backslashes] == b'\\' {
        backslashes += 1;
    }
    backslashes % 2 == 1
}

/// Desescapa caracteres según RFC 4514.
fn unescape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            if let Some(escaped) = chars.next() {
                result.push(escaped);
                continue;
            }
        }
        result.push(character);
    }
    result
}

/// Extrae el valor de un atributo de un nombre distinguido.
pub fn attribute(name: &str, distinguished_name: &str) -> String {
    attribute_pairs(distinguished_name)
        .into_iter()
        .find_map(|part| part.trim().strip_prefix(name).map(str::to_owned))
        .unwrap_or_default()
}

/// Extrae el nombre común (CN) y número de serie del subject del certificado.
pub fn holder_of(subject: Option<&str>) -> (String, String) {
    let subject = subject.unwrap_or_default();
    (
        attribute("CN=", subject),
        attribute("SERIALNUMBER=", subject),
    )
}

/// Datos del titular a estampar en la firma visible.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StampedHolder {
    /// El `CN` del subject, entero.
    pub common_name: String,
    /// La autoridad emisora, la misma que enseña el desplegable.
    pub issuer: String,
    /// Si el certificado es de seudónimo.
    pub pseudonym: bool,
}

/// Lo que el recuadro estampa de un certificado, leído del DER.
pub fn stamped_holder_of(certificate: &TokenCertificate) -> StampedHolder {
    let subject = certificate.subject();
    StampedHolder {
        common_name: attribute("CN=", subject.as_deref().unwrap_or_default()),
        issuer: issuer_of(certificate.issuer().as_deref()),
        pseudonym: is_pseudonym(subject.as_deref()),
    }
}

/// Comprueba si el certificado es de seudónimo según el RDN 2.5.4.65.
pub fn is_pseudonym(subject: Option<&str>) -> bool {
    const PSEUDONYM: [&str; 3] = ["2.5.4.65=", "OID.2.5.4.65=", "PSEUDONYM="];
    attribute_pairs(subject.unwrap_or_default())
        .iter()
        .any(|pair| {
            let pair = pair.trim().to_ascii_uppercase();
            PSEUDONYM.iter().any(|name| pair.starts_with(name))
        })
}

/// Extrae el nombre de la autoridad emisora a partir del emisor del certificado.
pub fn issuer_of(issuer: Option<&str>) -> String {
    let issuer = issuer.unwrap_or_default().trim();
    let common_name = attribute("CN=", issuer);
    if !common_name.is_empty() {
        return common_name;
    }
    let organisation = attribute("O=", issuer);
    if !organisation.is_empty() {
        return organisation;
    }
    issuer.to_owned()
}

#[cfg(test)]
mod tests;
