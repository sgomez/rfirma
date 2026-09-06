//! Ciclo de persistencia del certificado recordado entre sesiones contra token SoftHSM (ADR-0014).

use std::path::PathBuf;

use rfirma_lib::memory::{Configuration, Memory};
use rfirma_lib::paths::Paths;
use rfirma_lib::pkcs11::{self, CertificateRef, TokenCertificate};

const TOKEN: &str = "rfirma-test";
const ACTIVE: &str = "FNMT-ACTIVO-99999999R";
/// Certificados gemelos que comparten `CKA_LABEL` pero difieren en `CKA_ID`.
const TWIN: &str = "FNMT-GEMELO-99999999R";

fn module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Las pruebas de grada B necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just token",
        module.display()
    );
    module
}

fn certificates() -> Vec<TokenCertificate> {
    let found = pkcs11::list_certificates(module()).expect("no se ha podido listar el token");
    assert!(
        !found.is_empty(),
        "el token {TOKEN} esta vacio o no existe. Montalo con:\n  just token"
    );
    found
}

/// Todas las referencias con esa etiqueta, tal y como salen del token.
fn references_labelled(label: &str) -> Vec<CertificateRef> {
    let found: Vec<CertificateRef> = certificates()
        .into_iter()
        .filter(|certificate| certificate.reference().label() == label)
        .map(|certificate| certificate.reference().clone())
        .collect();
    assert!(
        !found.is_empty(),
        "el token {TOKEN} no tiene ningun certificado {label}. Montalo con: just token"
    );
    found
}

fn reference(label: &str) -> CertificateRef {
    references_labelled(label)
        .into_iter()
        .next()
        .expect("acaba de comprobarse que hay alguno")
}

/// Memoria de sesión en un directorio temporal.
fn a_session(root: &std::path::Path) -> Memory {
    Memory::at(&Paths::under(root))
}

/// Persiste el certificado en el estado de la sesión.
fn remember(memory: &Memory, reference: &CertificateRef) {
    let mut state = memory
        .state()
        .expect("deberia leerse el estado")
        .into_value();
    state.certificate = Some(reference.clone());
    memory
        .remember_state(&Configuration::default(), &state)
        .expect("deberia guardarse el estado");
}

/// Busca el certificado recordado en el token actual.
fn found_again(memory: &Memory) -> Option<CertificateRef> {
    let remembered = memory
        .state()
        .expect("deberia leerse el estado")
        .into_value()
        .certificate?;
    certificates()
        .into_iter()
        .map(|certificate| certificate.reference().clone())
        .find(|listed| remembered.is_the_same_as(listed))
}

#[test]
fn the_certificate_signed_with_comes_back_in_the_next_session() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let used = reference(ACTIVE);

    remember(&a_session(directory.path()), &used);

    assert_eq!(found_again(&a_session(directory.path())), Some(used));
}

#[test]
fn the_twin_that_was_used_is_the_one_that_comes_back() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let twins = references_labelled(TWIN);
    assert_eq!(twins.len(), 2, "el token de pruebas tiene dos gemelos");
    let second = twins[1].clone();

    remember(&a_session(directory.path()), &second);

    assert_eq!(found_again(&a_session(directory.path())), Some(second));
}

#[test]
fn a_remembered_certificate_that_is_gone_is_simply_not_found() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    remember(
        &a_session(directory.path()),
        &CertificateRef::new(module(), TOKEN, "EL-QUE-YA-NO-ESTA", vec![0xfe]),
    );

    assert_eq!(found_again(&a_session(directory.path())), None);
}

#[test]
fn with_the_activity_switch_off_no_certificate_reaches_the_disk() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_session(directory.path());
    let mut state = memory
        .state()
        .expect("deberia leerse el estado")
        .into_value();
    state.certificate = Some(reference(ACTIVE));

    memory
        .remember_state(
            &Configuration {
                remember_activity: false,
                ..Configuration::default()
            },
            &state,
        )
        .expect("no guardar no es un fallo");

    assert!(
        !Paths::under(directory.path()).state_file().exists(),
        "con el interruptor apagado no queda nada en el disco"
    );
    assert_eq!(found_again(&a_session(directory.path())), None);
}

#[test]
fn a_certificate_remembered_by_an_older_version_still_starts_the_session() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let state_file = Paths::under(directory.path()).state_file();
    std::fs::create_dir_all(state_file.parent().expect("deberia tener padre"))
        .expect("deberia crearse el directorio de estado");
    let older = serde_json::json!({
        "version": 1,
        "certificate": {
            "module": module(),
            "token_label": TOKEN,
            "label": ACTIVE,
        },
    });
    std::fs::write(&state_file, older.to_string()).expect("deberia escribirse el estado antiguo");

    let found = found_again(&a_session(directory.path()))
        .expect("el certificado sigue en el token y tiene que reencontrarse");

    assert_eq!(found.label(), ACTIVE);
    assert!(
        found.cka_id().is_some(),
        "el reencontrado trae las cinco coordenadas, que es lo que se reescribe al firmar"
    );
}
