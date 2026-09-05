//! **Qué certificados hay, cuál eligió la ventana y cuál se recordó.**
//!
//! Ninguna de estas funciones pide el PIN: los certificados son objetos
//! públicos del token y su estado se decide leyendo el DER. Pedir el secreto
//! que desbloquea la clave para luego decir que el certificado caducó es
//! hacerlo teclear para nada.

use std::path::Path;

use tauri_plugin_dialog::FilePath;

use crate::commands::views::{store_name, CertificateView};
use crate::commands::Failure;
use crate::memory::{Configuration, ListedCertificates, Memory};
use crate::pkcs11::{self, CertificateRef, Store, TokenCertificate};

/// **Caso de uso.** Los certificados de los tokens conectados, ya como filas.
///
/// Acuña las asas **aquí** y sustituye a las del listado anterior: la ventana
/// solo puede señalar filas del listado que tiene delante.
///
/// `installed_dir` entra sólo para **clasificar**: un `.p12` instalado es un
/// almacén NSS como el perfil de un navegador, y sin saber dónde viven los
/// instalados cruzaría como [`pkcs11::StoreClass::Nssdb`] y la lista de
/// Preferencias no sabría cuáles puede quitar (ID-198). La ruta no sale de
/// aquí: lo que cruza es la clase (ADR-0011).
pub fn listed_rows(
    stores: &[Store],
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
) -> Result<Vec<CertificateView>, Failure> {
    let found = pkcs11::list_certificates_across(stores)?;
    Ok(rows_of(found, installed_dir, listed, memory))
}

/// Las filas de un listado **ya hecho**, con sus asas recién acuñadas.
///
/// Sale de [`listed_rows`] porque el camino de la sede lista otra cosa: lo que
/// la ventana enseña allí es lo que sobrevive al filtro de la sede
/// ([`super::filtering`], ID-252), y no el listado entero. Pintar una fila es
/// lo mismo en los dos casos; **qué se pinta** es la decisión que cambia.
pub fn rows_of(
    found: Vec<TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
) -> Vec<CertificateView> {
    // Lo recordado se lee **una vez** y no una por fila: es el mismo fichero de
    // estado para todas.
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

/// El OID de `rsaEncryption`, que es la única clave con la que rfirma sabe
/// firmar: el mecanismo es una constante única, `CKM_SHA256_RSA_PKCS` (ID-16).
const RSA_ENCRYPTION: &str = "1.2.840.113549.1.1.1";

/// **Caso de uso.** Instala un `.p12` como **almacén NSS propio por fichero**
/// (ID-192).
///
/// Lo que entra es el **contenido** del fichero y la contraseña que lo abre;
/// del fichero no se recuerda nada, ni la ruta ni una copia (ID-196). Quien
/// descifra es NSS, no rfirma ([`crate::pkcs11::nss`], ID-193), y lo que queda
/// detrás es un almacén NSS corriente que [`listed_rows`] encuentra en el
/// siguiente listado sin que nadie le diga nada.
///
/// **Una clave que no sea RSA se rechaza aquí, no al firmar** (ID-197): el
/// almacén recién escrito se borra entero y no llega a existir para nadie.
/// Rechazarla al firmar sería descubrirlo después del secreto, con el
/// documento delante.
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

    // Lo que entra es el **contenido**; la ruta muere aquí y no llega ni al
    // almacén ni a la ventana (ID-196, ADR-0011).
    let source = chosen
        .into_path()
        .map_err(|error| Failure::new("pkcs12Unreadable", error.to_string()))?;
    let pkcs12 = std::fs::read(&source)
        .map_err(|error| Failure::new("pkcs12Unreadable", error.to_string()))?;

    // El nombre del directorio es un asa acuñada, no el del fichero: el nombre
    // de un `.p12` es del usuario y no tiene por qué acabar en el disco de la
    // aplicación (ID-196, ADR-0011).
    let directory = installed_dir.join(crate::memory::handles::mint());
    std::fs::create_dir_all(&directory).map_err(|error| {
        Failure::new(
            "settingsUnwritable",
            format!("no se ha podido crear el almacen del .p12: {error}"),
        )
    })?;
    // El almacén no lleva contraseña propia (ID-195), así que lo que lo protege
    // son los permisos del directorio.
    let _ = crate::paths::restrict_to_owner(&directory);

    let store = pkcs11::Store::nss(&softoken, &directory);
    let installed =
        pkcs11::with_token_turn(|| pkcs11::nss::import_pkcs12(&directory, &pkcs12, password))
            .and_then(|()| only_rsa_keys(&store));

    if let Err(error) = installed {
        // Media instalación no se queda puesta: o el almacén entero, o nada.
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error.into());
    }

    for file in ["cert9.db", "key4.db"] {
        let _ = crate::paths::restrict_to_owner(&directory.join(file));
    }
    Ok(())
}

/// Los certificados del almacén recién escrito, y **todos con clave RSA**.
///
/// Se abre el almacén de verdad en vez de mirar dentro del `.p12`: es la misma
/// puerta por la que se van a listar luego, así que comprueba a la vez las dos
/// cosas que pueden salir mal —que el fichero no traía nada instalable, y que
/// lo que traía no se puede firmar—.
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

/// Si la clave pública del certificado es RSA, leyéndolo del DER.
///
/// Un DER que no se sabe leer **no es RSA**: lo que no se puede comprobar no se
/// da por bueno, que es la misma regla con la que se decide el estado de un
/// certificado ilegible.
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

/// **Caso de uso.** Quita un `.p12` instalado: borra su almacén entero.
///
/// El asa es la de una fila del último listado, como en cualquier otra orden
/// (ADR-0011); de ella sale el almacén, y de él, el directorio. **Solo se borra
/// dentro de `installed_dir`**: un certificado del perfil de Firefox, o de una
/// tarjeta, tiene la misma forma de asa y aquí se rechaza en vez de tocar nada
/// de nadie.
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

/// El certificado que quedó recordado, si hay estado que leer.
///
/// No hace falta mirar «Recordar mi actividad» aquí: apagarlo borra el fichero
/// de estado, así que con el interruptor apagado no hay nada que leer. Un
/// estado ilegible no es un motivo para no listar certificados: se sigue sin
/// recordado, que es exactamente el primer arranque.
pub fn remembered_certificate(memory: &Memory) -> Option<CertificateRef> {
    memory.state().ok()?.into_value().certificate
}

/// Apunta con qué certificado se acaba de firmar.
///
/// Se llama **desde la postfirma y solo desde ahí** (#110): lo que se recuerda
/// es «con cuál firmé», no «cuál miré». Elegir uno en el desplegable, ver en la
/// vista previa que no era el que se quería y cerrar sin firmar no cambia lo
/// recordado; y de paso no hay una escritura en disco por cada clic.
///
/// Los interruptores los aplica [`Memory::remember_state`], que es donde no se
/// pueden olvidar: con «Recordar mi actividad» apagado esto no escribe nada y
/// borra lo que hubiera. Un fallo al escribir **no tumba la firma**: el
/// documento ya está firmado y en su carpeta, y perder la comodidad de la
/// próxima sesión no puede convertir eso en un error.
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

/// **Caso de uso.** Lo que el recuadro necesita del certificado elegido, leído
/// del DER.
///
/// **Sin PIN**: los certificados son objetos públicos del token.
pub fn stamped_holder_named(
    handle: &str,
    stores: &[Store],
    listed: &ListedCertificates,
) -> Result<StampedHolder, Failure> {
    let certificates = pkcs11::list_certificates_across(stores)?;
    let chosen = certificate_behind(&certificates, handle, listed)?;
    Ok(stamped_holder_of(chosen))
}

/// El certificado que hay detrás de un asa, buscado en el listado de ahora
/// mismo.
///
/// Son dos pasos y no uno: el asa da la [`CertificateRef`] que se apuntó al
/// listar, y la referencia —módulo, init args, token, etiqueta y `CKA_ID`— es
/// lo que empareja con el certificado que el token enseña **ahora**. Comparar
/// la referencia entera y no la etiqueta es lo que hace elegible al segundo de
/// dos certificados con la misma etiqueta.
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

/// El certificado que pide la orden, si sigue estando y sirve para firmar.
///
/// Se mira el estado **otra vez** aunque la ventana ya lo mirara al listar, y
/// no sobra: entre listar y firmar puede haberse retirado la tarjeta o haber
/// pasado la medianoche del `notAfter`. Es la última comprobación antes del
/// PIN, y la única que ve el token de ahora mismo.
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

/// Los pares `atributo=valor` de un nombre distinguido, partidos por comas
/// que **no** estén escapadas con `\`, y con esa barra ya desescapada.
///
/// El `CN` de un DNIe lleva una coma escapada dentro del propio nombre
/// —«APELLIDO1 APELLIDO2\, NOMBRE (FIRMA)»—, y partir por cualquier coma
/// trunca el titular a los apellidos. La barra es la sintaxis de escape del
/// RFC 4514, no un carácter del nombre: una coma va escapada cuando la
/// preceden un número **impar** de barras consecutivas (la barra también se
/// escapa a sí misma, así que `\\,` es una barra literal seguida de una coma
/// que sí separa), y el resultado desescapa esas barras antes de devolver
/// cada par (#194, punto 5; #198).
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

/// Si la coma en `index` va escapada: la preceden un número impar de barras
/// invertidas consecutivas.
fn comma_is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut backslashes = 0;
    while index > backslashes && bytes[index - 1 - backslashes] == b'\\' {
        backslashes += 1;
    }
    backslashes % 2 == 1
}

/// Quita las barras de escape del RFC 4514 (`\,` → `,`, `\\` → `\`, …), sin
/// tocar nada más.
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

/// El valor de un atributo de un nombre distinguido, o la cadena vacía si no
/// está.
pub fn attribute(name: &str, distinguished_name: &str) -> String {
    attribute_pairs(distinguished_name)
        .into_iter()
        .find_map(|part| part.trim().strip_prefix(name).map(str::to_owned))
        .unwrap_or_default()
}

/// El titular y el DNI que se leen del **subject**, para el recuadro y para la
/// fila del panel.
pub fn holder_of(subject: Option<&str>) -> (String, String) {
    let subject = subject.unwrap_or_default();
    (
        attribute("CN=", subject),
        attribute("SERIALNUMBER=", subject),
    )
}

/// Lo que se estampa del certificado en el recuadro de la firma visible.
///
/// El `CN` viaja **entero y en claro** —nombre y DNI juntos, que es como lo
/// enseña AutoFirma—: quien tapa el identificador es el compositor del texto,
/// y por eso hace falta saber además si el certificado es de seudónimo, que es
/// la única excepción a la máscara.
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

/// Si el certificado es **de seudónimo**: su subject declara el RDN
/// `2.5.4.65`, que es como lo decide el original (`AOUtil.isPseudonymCert`).
///
/// El nombre del atributo se busca en las tres formas en que puede salir
/// impreso un nombre distinguido —el OID pelado, el OID con prefijo y el
/// nombre corto—, porque quién lo imprime no es cosa nuestra: viene del DER
/// por `x509-cert`.
pub fn is_pseudonym(subject: Option<&str>) -> bool {
    const PSEUDONYM: [&str; 3] = ["2.5.4.65=", "OID.2.5.4.65=", "PSEUDONYM="];
    attribute_pairs(subject.unwrap_or_default())
        .iter()
        .any(|pair| {
            let pair = pair.trim().to_ascii_uppercase();
            PSEUDONYM.iter().any(|name| pair.starts_with(name))
        })
}

/// La autoridad emisora, tal como se enseña en el panel («Emitido por …»).
///
/// Sale del **issuer**, no del `O=` del subject: ese es la organización del
/// titular. Un certificado de persona física de la FNMT no lleva `O=` en el
/// subject —así que ahí el panel se quedaba en «Emitido por »— y uno de
/// empleado público sí, con el organismo del titular, que el panel afirmaba
/// que había emitido el certificado. El emisor es el dato con el que alguien
/// decide si se fía, y no admite un valor aproximado.
///
/// Se enseña el `CN=` del issuer, que es como se nombra a una autoridad («AC
/// FNMT Usuarios»); si no lo lleva se cae al `O=` del issuer, y si tampoco, al
/// nombre distinguido entero, que es feo pero cierto.
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

    /// Los certificados de seudónimo quedan exentos de la máscara del
    /// recuadro, y lo que los distingue es el RDN `2.5.4.65`, salga impreso
    /// como salga.
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

    /// El caso que rompía: el subject de un certificado de persona física de la
    /// FNMT **no lleva `O=`**, así que leer el emisor de ahí dejaba el panel en
    /// «Emitido por » y nada más.
    #[test]
    fn the_issuer_is_the_authority_and_not_the_organisation_of_the_holder() {
        let subject =
            "CN=EIDAS CERTIFICADO PRUEBAS - 99999999R, serialNumber=IDCES-99999999R, C=ES";
        let issuer = "CN=AC FNMT Usuarios, OU=Ceres, O=FNMT-RCM, C=ES";

        assert_eq!(issuer_of(Some(issuer)), "AC FNMT Usuarios");
        assert_eq!(attribute("O=", subject), "");
    }

    /// El otro caso malo: el `O=` del subject de un empleado público es su
    /// organismo, y enseñarlo como emisor afirmaba que ese organismo emitió el
    /// certificado.
    #[test]
    fn the_organisation_of_a_public_employee_is_never_read_as_the_issuer() {
        let subject = "CN=LOVELACE BYRON ADA, O=AYUNTAMIENTO DE CADIZ, C=ES";
        let issuer = "CN=AC Administracion Publica, O=FNMT-RCM, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "LOVELACE BYRON ADA");
        assert_eq!(id, "");
        assert_eq!(issuer_of(Some(issuer)), "AC Administracion Publica");
    }

    /// El caso de representante de empresa sigue funcionando igual que antes:
    /// varios atributos separados por comas sin ninguna escapada de por medio.
    #[test]
    fn the_holder_of_a_company_representative_is_read_whole() {
        let subject = "CN=LOVELACE BYRON ADA - R: B00000000, SERIALNUMBER=IDCES-00000000T, \
                        O=ANALYTICAL ENGINES SL, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "LOVELACE BYRON ADA - R: B00000000");
        assert_eq!(id, "IDCES-00000000T");
    }

    /// El caso que rompía el DNIe: su `CN` lleva una coma escapada dentro
    /// —«APELLIDO1 APELLIDO2\, NOMBRE (FIRMA)»—, y partir por comas sin
    /// respetar el escapado truncaba el titular a los apellidos. La barra de
    /// escape no es parte del nombre: no debe llegar al recuadro ni a la fila.
    #[test]
    fn a_common_name_with_an_escaped_comma_is_read_whole() {
        let subject = "CN=APELLIDO1 APELLIDO2\\, NOMBRE (FIRMA), SERIALNUMBER=00000000T, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "APELLIDO1 APELLIDO2, NOMBRE (FIRMA)");
        assert_eq!(id, "00000000T");
    }

    /// Una barra literal delante de la coma no la escapa: el RFC 4514 escapa
    /// la barra a sí misma, así que `\\,` es una barra seguida de una coma que
    /// sí separa. Contar solo el byte anterior confundiría esto con una coma
    /// escapada y fundiría el nombre distinguido entero en un único par.
    #[test]
    fn a_literal_backslash_before_the_comma_does_not_escape_it() {
        let subject = "CN=FOO\\\\,SERIALNUMBER=00000000T";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "FOO\\");
        assert_eq!(id, "00000000T");
    }

    /// Un issuer sin `CN=` no deja el panel mudo: se cae al `O=`, y sin ninguno
    /// de los dos, al nombre distinguido entero.
    #[test]
    fn an_issuer_without_a_common_name_falls_back_instead_of_going_blank() {
        assert_eq!(issuer_of(Some("O=FNMT-RCM, C=ES")), "FNMT-RCM");
        assert_eq!(issuer_of(Some("OU=Ceres, C=ES")), "OU=Ceres, C=ES");
        assert_eq!(issuer_of(None), "");
    }

    /// Sin ningún almacén donde buscar el listado **no sale vacío**: sale con
    /// su fallo. Una lista vacía diría «no tienes certificados», que es otra
    /// cosa y manda a mirar donde no es.
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

        assert!(!failure.detail.is_empty(), "con su detalle crudo (ID-29)");
    }

    #[test]
    fn refuses_a_certificate_that_is_no_longer_in_the_token() {
        let certificates = [a_certificate("FIRMA", &[])];
        let (listed, handles) = listed_from(&certificates);

        let failure = usable_certificate(&[], &handles[0], &listed).expect_err("ya no esta");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("FIRMA"), "{}", failure.detail);
    }

    /// Un asa que no salió de la última búsqueda no elige nada: la ventana solo
    /// puede señalar filas del listado que tiene delante.
    #[test]
    fn refuses_a_handle_that_is_not_from_the_last_listing() {
        let listed = ListedCertificates::new();

        let failure = usable_certificate(&[], "00000000000000000000000000000000", &listed)
            .expect_err("no es de la ultima busqueda");

        assert_eq!(failure.situation, "certificateNotFound");
    }

    /// El caso que hacía falta el asa: dos certificados con la **misma
    /// etiqueta** son elegibles por separado, y elegir el segundo firma con el
    /// segundo. Buscando por etiqueta se cogía siempre el primero.
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
        // La ventana ya lo miró al listar, y aun así se vuelve a mirar: entre
        // una cosa y otra puede haberse retirado la tarjeta o haber pasado la
        // medianoche del `notAfter`.
        let certificates = [a_certificate("FIRMA", &[0x00, 0x01, 0x02])];
        let (listed, handles) = listed_from(&certificates);

        let failure =
            usable_certificate(&certificates, &handles[0], &listed).expect_err("no es legible");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("Unreadable"), "{}", failure.detail);
    }

    /// Lo pedido: tras una firma con éxito el certificado usado queda escrito,
    /// y en la sesión siguiente se vuelve a encontrar.
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

    /// El certificado es actividad, y «Recordar mi actividad» manda: con el
    /// interruptor apagado no se escribe nada.
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

    /// Y apagar el interruptor **borra** el que hubiera, que es la otra mitad
    /// del ID-34. Lo hace `Memory::remember_configuration`; lo que se comprueba
    /// aquí es que el certificado se va con el resto y no sobrevive aparte.
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

    /// Un certificado recordado que ya no está en el token **no marca ninguna
    /// fila**: el panel vuelve a «Sin certificado» y no hay error que contar
    /// (ADR-0010).
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

    /// Y el primer arranque no recuerda nada ni se queja de nada: no hay
    /// fichero de estado que leer.
    #[test]
    fn a_first_run_has_no_remembered_certificate() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");

        assert_eq!(remembered_certificate(&a_memory(documents.path())), None);
    }
}
