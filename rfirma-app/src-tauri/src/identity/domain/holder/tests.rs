use super::{attribute, holder_of, is_pseudonym, issuer_of};

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
