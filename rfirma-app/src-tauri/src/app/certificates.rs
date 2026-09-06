//! Listado, inspección y selección de certificados en tokens sin pedir PIN.

use std::path::Path;

use tauri_plugin_dialog::FilePath;

use crate::commands::views::{store_name, CertificateView};
use crate::commands::Failure;
use crate::memory::{Configuration, ListedCertificates, Memory};
use crate::pkcs11::{self, CertificateRef, Store, TokenCertificate};

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

    let directory = installed_dir.join(crate::memory::handles::mint());
    std::fs::create_dir_all(&directory).map_err(|error| {
        Failure::new(
            "settingsUnwritable",
            format!("no se ha podido crear el almacen del .p12: {error}"),
        )
    })?;
    let _ = crate::paths::restrict_to_owner(&directory);

    let store = pkcs11::Store::nss(&softoken, &directory);
    let installed =
        pkcs11::with_token_turn(|| pkcs11::nss::import_pkcs12(&directory, &pkcs12, password))
            .and_then(|()| only_rsa_keys(&store));

    if let Err(error) = installed {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error.into());
    }

    for file in ["cert9.db", "key4.db"] {
        let _ = crate::paths::restrict_to_owner(&directory.join(file));
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
mod tests {
    use super::{
        attribute, certificate_behind, holder_of, is_pseudonym, issuer_of, listed_rows,
        remember_the_certificate, remembered_certificate, usable_certificate,
    };
    use crate::app::fixtures::{a_certificate, a_certificate_with_id, a_memory, listed_from};
    use crate::memory::{Configuration, ListedCertificates};

    #[test]
    fn reads_the_holder_and_the_id_out_of_the_subject() {
        let (name, id) = holder_of(Some(
            "CN=LOVELACE BYRON ADA, SERIALNUMBER=IDCES-00000000T, O=FNMT-RCM",
        ));

        assert_eq!(name, "LOVELACE BYRON ADA");
        assert_eq!(id, "IDCES-00000000T");
    }

    #[test]
    fn a_subject_without_the_fields_gives_empty_strings_and_not_a_panic() {
        assert_eq!(holder_of(None), (String::new(), String::new()));
    }

    #[test]
    fn a_subject_with_the_pseudonym_rdn_is_a_pseudonym_certificate() {
        for subject in [
            "CN=SEUDONIMO, 2.5.4.65=ADA, C=ES",
            "CN=SEUDONIMO, OID.2.5.4.65=ADA, C=ES",
            "CN=SEUDONIMO, pseudonym=ADA, C=ES",
        ] {
            assert!(is_pseudonym(Some(subject)), "«{subject}» es de seudónimo");
        }
    }

    #[test]
    fn a_subject_without_that_rdn_is_not_a_pseudonym_certificate() {
        assert!(!is_pseudonym(Some(
            "CN=LOVELACE BYRON ADA - 99999999R, serialNumber=IDCES-99999999R, C=ES"
        )));
        assert!(!is_pseudonym(None));
    }

    #[test]
    fn the_issuer_is_the_authority_and_not_the_organisation_of_the_holder() {
        let subject =
            "CN=EIDAS CERTIFICADO PRUEBAS - 99999999R, serialNumber=IDCES-99999999R, C=ES";
        let issuer = "CN=AC FNMT Usuarios, OU=Ceres, O=FNMT-RCM, C=ES";

        assert_eq!(issuer_of(Some(issuer)), "AC FNMT Usuarios");
        assert_eq!(attribute("O=", subject), "");
    }

    #[test]
    fn the_organisation_of_a_public_employee_is_never_read_as_the_issuer() {
        let subject = "CN=LOVELACE BYRON ADA, O=AYUNTAMIENTO DE CADIZ, C=ES";
        let issuer = "CN=AC Administracion Publica, O=FNMT-RCM, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "LOVELACE BYRON ADA");
        assert_eq!(id, "");
        assert_eq!(issuer_of(Some(issuer)), "AC Administracion Publica");
    }

    #[test]
    fn the_holder_of_a_company_representative_is_read_whole() {
        let subject = "CN=LOVELACE BYRON ADA - R: B00000000, SERIALNUMBER=IDCES-00000000T, \
                        O=ANALYTICAL ENGINES SL, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "LOVELACE BYRON ADA - R: B00000000");
        assert_eq!(id, "IDCES-00000000T");
    }

    #[test]
    fn a_common_name_with_an_escaped_comma_is_read_whole() {
        let subject = "CN=APELLIDO1 APELLIDO2\\, NOMBRE (FIRMA), SERIALNUMBER=00000000T, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "APELLIDO1 APELLIDO2, NOMBRE (FIRMA)");
        assert_eq!(id, "00000000T");
    }

    #[test]
    fn a_literal_backslash_before_the_comma_does_not_escape_it() {
        let subject = "CN=FOO\\\\,SERIALNUMBER=00000000T";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "FOO\\");
        assert_eq!(id, "00000000T");
    }

    #[test]
    fn an_issuer_without_a_common_name_falls_back_instead_of_going_blank() {
        assert_eq!(issuer_of(Some("O=FNMT-RCM, C=ES")), "FNMT-RCM");
        assert_eq!(issuer_of(Some("OU=Ceres, C=ES")), "OU=Ceres, C=ES");
        assert_eq!(issuer_of(None), "");
    }

    #[test]
    fn with_nowhere_to_look_the_listing_says_so_instead_of_coming_back_empty() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");

        let failure = listed_rows(
            &[],
            &home.path().join("certificates"),
            &ListedCertificates::new(),
            &a_memory(home.path()),
        )
        .expect_err("no hay donde buscar");

        assert!(!failure.detail.is_empty(), "con su detalle crudo");
    }

    #[test]
    fn refuses_a_certificate_that_is_no_longer_in_the_token() {
        let certificates = [a_certificate("FIRMA", &[])];
        let (listed, handles) = listed_from(&certificates);

        let failure = usable_certificate(&[], &handles[0], &listed).expect_err("ya no esta");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("FIRMA"), "{}", failure.detail);
    }

    #[test]
    fn refuses_a_handle_that_is_not_from_the_last_listing() {
        let listed = ListedCertificates::new();

        let failure = usable_certificate(&[], "00000000000000000000000000000000", &listed)
            .expect_err("no es de la ultima busqueda");

        assert_eq!(failure.situation, "certificateNotFound");
    }

    #[test]
    fn two_certificates_with_the_same_label_are_chosen_apart() {
        let certificates = [
            a_certificate_with_id("FNMT-GEMELO-99999999R", 0x04, &[]),
            a_certificate_with_id("FNMT-GEMELO-99999999R", 0x05, &[]),
        ];
        let (listed, handles) = listed_from(&certificates);

        let first = certificate_behind(&certificates, &handles[0], &listed).expect("el primero");
        let second = certificate_behind(&certificates, &handles[1], &listed).expect("el segundo");

        assert_ne!(handles[0], handles[1]);
        assert_eq!(first.reference().cka_id(), Some([0x04].as_slice()));
        assert_eq!(second.reference().cka_id(), Some([0x05].as_slice()));
    }

    #[test]
    fn looks_at_the_status_again_between_listing_and_signing() {
        let certificates = [a_certificate("FIRMA", &[0x00, 0x01, 0x02])];
        let (listed, handles) = listed_from(&certificates);

        let failure =
            usable_certificate(&certificates, &handles[0], &listed).expect_err("no es legible");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("Unreadable"), "{}", failure.detail);
    }

    #[test]
    fn the_certificate_signed_with_is_written_into_the_state() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(documents.path());
        let used = a_certificate("FNMT-ACTIVO-99999999R", b"da igual");

        remember_the_certificate(&memory, &Configuration::default(), used.reference());

        assert_eq!(
            remembered_certificate(&memory).as_ref(),
            Some(used.reference()),
            "la proxima sesion tiene que encontrarlo"
        );
    }

    #[test]
    fn the_certificate_is_not_remembered_with_the_activity_switch_off() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let paths = crate::paths::Paths::under(documents.path());
        let memory = a_memory(documents.path());
        let switched_off = Configuration {
            remember_activity: false,
            ..Configuration::default()
        };
        let used = a_certificate("FNMT-ACTIVO-99999999R", b"da igual");

        remember_the_certificate(&memory, &switched_off, used.reference());

        assert!(
            !paths.state_file().exists(),
            "con el interruptor apagado no se escribe ningun certificado"
        );
        assert_eq!(remembered_certificate(&memory), None);
    }

    #[test]
    fn turning_the_activity_switch_off_erases_the_certificate_already_remembered() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(documents.path());
        remember_the_certificate(
            &memory,
            &Configuration::default(),
            a_certificate("FNMT-ACTIVO-99999999R", b"da igual").reference(),
        );

        memory
            .remember_configuration(&Configuration {
                remember_activity: false,
                ..Configuration::default()
            })
            .expect("deberia guardarse la configuracion");

        assert_eq!(remembered_certificate(&memory), None);
    }

    #[test]
    fn a_remembered_certificate_that_is_gone_marks_no_row() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(documents.path());
        remember_the_certificate(
            &memory,
            &Configuration::default(),
            a_certificate("EL-QUE-YA-NO-ESTA", b"da igual").reference(),
        );
        let remembered = remembered_certificate(&memory).expect("algo se recordo");

        let present = a_certificate("FNMT-ACTIVO-99999999R", b"da igual");

        assert!(!remembered.is_the_same_as(present.reference()));
    }

    #[test]
    fn a_first_run_has_no_remembered_certificate() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");

        assert_eq!(remembered_certificate(&a_memory(documents.path())), None);
    }
}
