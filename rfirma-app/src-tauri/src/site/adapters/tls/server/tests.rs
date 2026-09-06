use openssl::stack::Stack;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::{X509StoreContext, X509VerifyResult};

use super::*;

fn verdict(ca: &LocalCa, certificate: &X509) -> X509VerifyResult {
    let mut store = X509StoreBuilder::new().expect("deberia haber almacen");
    store
        .add_cert(ca.certificate().clone())
        .expect("deberia entrar la CA local");
    let store = store.build();

    let chain = Stack::new().expect("deberia haber pila");
    let mut context = X509StoreContext::new().expect("deberia haber contexto");
    context
        .init(&store, certificate, &chain, |context| {
            let _ = context.verify_cert();
            Ok(context.error())
        })
        .expect("deberia verificarse")
}

#[test]
fn the_sede_reaches_the_local_server_by_name_and_by_address() {
    let ca = LocalCa::generate().expect("deberia generarse");
    let server = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

    let text = String::from_utf8(server.certificate().to_text().expect("deberia imprimirse"))
        .expect("deberia ser UTF-8");

    let common_name = server
        .certificate()
        .subject_name()
        .entries_by_nid(Nid::COMMONNAME)
        .next()
        .expect("deberia haber CN")
        .data()
        .to_string()
        .expect("deberia ser UTF-8");
    assert_eq!(common_name, "localhost");
    assert!(
        text.contains("DNS:localhost, IP Address:127.0.0.1"),
        "hacen falta las dos entradas en la SAN:\n{text}"
    );
}

#[test]
fn a_browser_that_trusts_the_local_ca_accepts_the_local_server_certificate() {
    let ca = LocalCa::generate().expect("deberia generarse");
    let server = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

    assert_eq!(verdict(&ca, server.certificate()), X509VerifyResult::OK);
}

#[test]
fn the_local_ca_cannot_vouch_for_a_site_outside_the_loopback() {
    let ca = LocalCa::generate().expect("deberia generarse");
    let key = generate_key().expect("deberia generarse");

    let impostor = issue(&ca, &key, "sede.example", |names| {
        names.dns("sede.example");
    })
    .expect("emitirlo se puede: quien lo rechaza es el verificador");

    let verdict = verdict(&ca, &impostor);
    assert_ne!(
        verdict,
        X509VerifyResult::OK,
        "la restriccion de nombres es lo que hace inofensiva una CA local abandonada"
    );
    assert!(
        verdict.error_string().contains("permitted subtree"),
        "lo rechaza `nameConstraints` y no otra cosa: {}",
        verdict.error_string()
    );
}

#[test]
fn the_local_server_certificate_is_not_an_authority() {
    let ca = LocalCa::generate().expect("deberia generarse");
    let server = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

    let text = String::from_utf8(server.certificate().to_text().expect("deberia imprimirse"))
        .expect("deberia ser UTF-8");

    assert!(text.contains("CA:FALSE"), "{text}");
    assert!(text.contains("TLS Web Server Authentication"), "{text}");
}

#[test]
fn every_boot_gets_a_brand_new_local_server_certificate() {
    let ca = LocalCa::generate().expect("deberia generarse");

    let one = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");
    let another = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

    assert_ne!(
        one.private_key_pem().expect("deberia salir en PEM"),
        another.private_key_pem().expect("deberia salir en PEM"),
        "las claves generadas deben ser distintas"
    );
}
