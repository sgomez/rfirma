//! **Qué certificados hay, cuál eligió la ventana y cuál se recordó.**
//!
//! Ninguna de estas funciones pide el PIN: los certificados son objetos
//! públicos del token y su estado se decide leyendo el DER. Pedir el secreto
//! que desbloquea la clave para luego decir que el certificado caducó es
//! hacerlo teclear para nada.

use crate::commands::views::{store_name, CertificateView};
use crate::commands::Failure;
use crate::memory::{Configuration, ListedCertificates, Memory};
use crate::pkcs11::{self, CertificateRef, Store, TokenCertificate};

/// **Caso de uso.** Los certificados de los tokens conectados, ya como filas.
///
/// Acuña las asas **aquí** y sustituye a las del listado anterior: la ventana
/// solo puede señalar filas del listado que tiene delante.
pub fn listed_rows(
    stores: &[Store],
    listed: &ListedCertificates,
    memory: &Memory,
) -> Result<Vec<CertificateView>, Failure> {
    let found = pkcs11::list_certificates_across(stores)?;
    // Lo recordado se lee **una vez** y no una por fila: es el mismo fichero de
    // estado para todas.
    let remembered = remembered_certificate(memory);
    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );
    Ok(found
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
                store: store_name(certificate.reference().store().class()).to_owned(),
                status: certificate.status().into(),
                remembered: remembered
                    .as_ref()
                    .is_some_and(|one| one.is_the_same_as(certificate.reference())),
            }
        })
        .collect())
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

/// **Caso de uso.** El nombre y el DNI del certificado elegido, leídos del DER.
///
/// **Sin PIN**: los certificados son objetos públicos del token.
pub fn holder_named(
    handle: &str,
    stores: &[Store],
    listed: &ListedCertificates,
) -> Result<(String, String), Failure> {
    let certificates = pkcs11::list_certificates_across(stores)?;
    let chosen = certificate_behind(&certificates, handle, listed)?;
    Ok(holder_of(chosen.subject().as_deref()))
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
        attribute, certificate_behind, holder_of, issuer_of, listed_rows, remember_the_certificate,
        remembered_certificate, usable_certificate,
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

        let failure = listed_rows(&[], &ListedCertificates::new(), &a_memory(home.path()))
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
