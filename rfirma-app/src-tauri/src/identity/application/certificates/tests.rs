use super::{
    attribute, certificate_behind, holder_of, is_pseudonym, issuer_of, listed_rows,
    remember_the_certificate, remembered_certificate, usable_certificate,
};
use crate::fixtures::{a_certificate, a_certificate_with_id, a_memory, listed_from};
use crate::identity::application::listed::ListedCertificates;
use crate::signing::application::configuration_memory::Configuration;

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
    let subject = "CN=EIDAS CERTIFICADO PRUEBAS - 99999999R, serialNumber=IDCES-99999999R, C=ES";
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
    let paths = crate::desktop::adapters::paths::Paths::under(documents.path());
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
