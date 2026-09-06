use super::*;

fn store_in(directory: &Path) -> LocalCaStore {
    LocalCaStore::of(&Paths::under(directory))
}

#[test]
fn the_first_boot_finds_no_local_ca_and_that_is_not_a_failure() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    let found = store_in(directory.path())
        .read()
        .expect("no haber nada no es un fallo");

    assert!(found.is_none());
}

#[test]
fn the_local_ca_survives_a_restart() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = store_in(directory.path());
    let ca = LocalCa::generate().expect("deberia generarse");

    store.write(&ca).expect("deberia guardarse");
    let restored = store
        .read()
        .expect("deberia leerse")
        .expect("la CA local se conserva entre arranques");

    assert_eq!(
        restored.certificate().to_pem().unwrap(),
        ca.certificate().to_pem().unwrap()
    );
}

#[test]
fn a_local_ca_that_no_longer_parses_is_said_out_loud() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = store_in(directory.path());
    store
        .write(&LocalCa::generate().expect("deberia generarse"))
        .expect("deberia guardarse");
    std::fs::write(store.certificate_path(), b"esto no es un PEM").expect("deberia escribirse");

    let error = store.read().expect_err("un PEM roto no es 'no hay nada'");

    assert_eq!(error.situation(), Situation::MaterialDamaged);
}

#[test]
fn the_next_local_ca_is_saved_beside_the_serving_one_and_not_over_it() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = store_in(directory.path());
    let serving = LocalCa::generate().expect("deberia generarse");
    let next = LocalCa::generate().expect("deberia generarse");

    store.write(&serving).expect("deberia guardarse la vigente");
    store
        .write_next(&next)
        .expect("deberia guardarse la siguiente");

    assert_eq!(
        store
            .read()
            .unwrap()
            .unwrap()
            .certificate()
            .to_pem()
            .unwrap(),
        serving.certificate().to_pem().unwrap(),
        "la que sirve sigue siendo la vigente"
    );
    assert_eq!(
        store
            .read_next()
            .unwrap()
            .unwrap()
            .certificate()
            .to_pem()
            .unwrap(),
        next.certificate().to_pem().unwrap()
    );
}

#[test]
fn the_next_local_ca_takes_over_and_leaves_its_slot_empty() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = store_in(directory.path());
    store
        .write(&LocalCa::generate().expect("deberia generarse"))
        .expect("deberia guardarse");
    let next = LocalCa::generate().expect("deberia generarse");
    store.write_next(&next).expect("deberia guardarse");

    let promoted = store
        .promote_next()
        .expect("deberia poder relevarse")
        .expect("habia una siguiente esperando");

    assert_eq!(
        promoted.certificate().to_pem().unwrap(),
        next.certificate().to_pem().unwrap()
    );
    assert_eq!(
        store
            .read()
            .unwrap()
            .unwrap()
            .certificate()
            .to_pem()
            .unwrap(),
        next.certificate().to_pem().unwrap()
    );
    assert!(store.read_next().unwrap().is_none());
}

#[test]
fn a_takeover_without_a_next_local_ca_is_not_a_failure() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = store_in(directory.path());

    assert!(store.promote_next().expect("no es un fallo").is_none());
    assert!(store.forget_next().is_ok(), "tirar lo que no hay tampoco");
}
