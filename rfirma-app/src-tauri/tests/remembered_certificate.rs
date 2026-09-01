//! El certificado que se recordó, contra un token de verdad. **Grada B**
//! (SoftHSM): carril rápido, segundos (ADR-0014, TD-01).
//!
//! El token lo monta `just token` desde `testdata/fnmt/`, igual que para
//! `tests/pkcs11_token.rs`; la tabla de sus cinco certificados está
//! documentada allí.
//!
//! Lo que se comprueba aquí es el **ciclo entre sesiones**, que es lo único
//! que necesita el token y que las pruebas de grada A no pueden decir: firmar,
//! cerrar, volver a abrir y encontrar puesto el mismo certificado. Las
//! coordenadas que se escriben salen del listado real y no de una referencia
//! inventada, porque una referencia fabricada a mano probaría que dos
//! literales son iguales y no que el certificado se reencuentra.
//!
//! No hay PIN en ninguna de estas pruebas, y no es un atajo: los certificados
//! son objetos públicos del token, y reencontrar el recordado al arrancar tiene
//! que poder hacerse sin pedirle nada a nadie (#110).

use std::path::PathBuf;

use rfirma_lib::memory::{Configuration, Memory};
use rfirma_lib::paths::Paths;
use rfirma_lib::pkcs11::{self, CertificateRef, TokenCertificate};

const TOKEN: &str = "rfirma-test";
const ACTIVE: &str = "FNMT-ACTIVO-99999999R";
/// Los dos que comparten `CKA_LABEL` y no comparten clave: son la razón de que
/// la referencia lleve `CKA_ID`.
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

/// La memoria de una sesión: dos ficheros en un directorio temporal, como los
/// tendría quien acaba de instalar la aplicación.
fn a_session(root: &std::path::Path) -> Memory {
    Memory::at(&Paths::under(root))
}

/// Con qué certificado se firmó, apuntado como lo apunta la postfirma.
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

/// Lo que hace el arranque: leer lo recordado y buscarlo en el token de ahora.
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

/// El ciclo entero: se firma con uno, se cierra la sesión, y la siguiente
/// arranca con ese mismo puesto.
#[test]
fn the_certificate_signed_with_comes_back_in_the_next_session() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let used = reference(ACTIVE);

    remember(&a_session(directory.path()), &used);

    // Otra `Memory` sobre las mismas rutas: es lo que hay al volver a abrir.
    assert_eq!(found_again(&a_session(directory.path())), Some(used));
}

/// Y vuelve el **exacto**, no el primero que comparta etiqueta: los dos
/// gemelos del token tienen el mismo `CKA_LABEL` y distinto `CKA_ID`, que es
/// justo la colisión que hay en un perfil de Firefox de verdad.
#[test]
fn the_twin_that_was_used_is_the_one_that_comes_back() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let twins = references_labelled(TWIN);
    assert_eq!(twins.len(), 2, "el token de pruebas tiene dos gemelos");
    let second = twins[1].clone();

    remember(&a_session(directory.path()), &second);

    assert_eq!(found_again(&a_session(directory.path())), Some(second));
}

/// Un certificado recordado que ya no está —tarjeta fuera, perfil borrado— no
/// se encuentra, y eso **no es un error**: el arranque sigue y el panel vuelve
/// a «Sin certificado» sin ruido (ADR-0010).
#[test]
fn a_remembered_certificate_that_is_gone_is_simply_not_found() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    remember(
        &a_session(directory.path()),
        &CertificateRef::new(module(), TOKEN, "EL-QUE-YA-NO-ESTA", vec![0xfe]),
    );

    assert_eq!(found_again(&a_session(directory.path())), None);
}

/// Con «Recordar mi actividad» apagado no se escribe ningún certificado, ni
/// siquiera pidiéndolo: lo impide `Memory::remember_state`, que es donde no se
/// puede olvidar (ID-34).
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

/// Lo que recordó una versión anterior al #98 y al #99 no lleva `CKA_ID` ni
/// `init_args`, y el arranque tiene que sobrevivirlo: se reencuentra el
/// certificado por las coordenadas que sí tiene.
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
