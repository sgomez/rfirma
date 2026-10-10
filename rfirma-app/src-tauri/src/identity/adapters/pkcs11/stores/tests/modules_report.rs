//! Pruebas del informe de módulos PKCS#11 del descubrimiento.

use super::*;

#[test]
fn the_modules_report_lists_what_is_used_and_what_is_discarded_without_changing_it() {
    let temp = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let usr = temp.path().join("usr");
    let opensc = usr.join("lib/pkcs11/opensc-pkcs11.so");
    std::fs::create_dir_all(opensc.parent().expect("tiene padre"))
        .expect("deberia poder crearse el directorio");
    std::fs::write(&opensc, b"").expect("deberia poder escribirse la biblioteca");
    let registry = temp.path().join("modules");
    std::fs::create_dir_all(&registry).expect("deberia poder crearse el registro");
    std::fs::write(registry.join("opensc.module"), "module: opensc-pkcs11.so\n")
        .expect("deberia poder escribirse el .module");
    std::fs::write(
        registry.join("dnie.module"),
        "module: dnie.so\ndisable-in: rfirma\n",
    )
    .expect("deberia poder escribirse el .module");
    let directories = [registry.clone()];

    let report = module_discovery(&usr, None, &directories);

    let used: Vec<PathBuf> = report
        .iter()
        .filter(|module| module.discard.is_none())
        .filter_map(|module| module.library.clone())
        .collect();
    assert_eq!(used, discovered_modules(&usr, None, &directories));
    let opensc_entry = report
        .iter()
        .find(|module| module.name == "opensc")
        .expect("opensc");
    assert_eq!(
        opensc_entry.registration,
        Some(registry.join("opensc.module"))
    );
    let dnie = report
        .iter()
        .find(|module| module.name == "dnie")
        .expect("dnie");
    assert_eq!(dnie.discard, Some(p11kit::DiscardReason::DisabledInRfirma));
    assert_eq!(dnie.library, Some(PathBuf::from("dnie.so")));
}
