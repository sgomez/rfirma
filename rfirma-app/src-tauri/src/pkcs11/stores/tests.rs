use super::*;

#[test]
fn keeps_only_the_candidates_that_are_there() {
    let stores = present_among(&["/hay/uno.so", "/no/hay.so", "/hay/otro.so"], |path| {
        path.starts_with("/hay")
    });

    assert_eq!(
        stores,
        vec![PathBuf::from("/hay/uno.so"), PathBuf::from("/hay/otro.so")]
    );
}

#[test]
fn a_store_under_the_installed_directory_is_its_own_class() {
    let home = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let installed = home.path().join("certificates");
    let mine = installed.join("2a01");
    std::fs::create_dir_all(&mine).expect("deberia poder crearse el almacen");
    std::fs::write(mine.join("cert9.db"), b"").expect("deberia poder escribirse cert9.db");
    let elsewhere = home.path().join(".pki/nssdb");
    std::fs::create_dir_all(&elsewhere).expect("deberia poder crearse el perfil");

    assert_eq!(
        Store::nss("/usr/lib/libsoftokn3.so", &mine).class_under(&installed),
        StoreClass::Installed
    );
    assert_eq!(
        Store::nss("/usr/lib/libsoftokn3.so", &elsewhere).class_under(&installed),
        StoreClass::Chrome
    );
}

#[test]
fn has_no_stores_when_no_candidate_is_installed() {
    assert!(present_among(CANDIDATE_MODULES, |_| false).is_empty());
}

#[test]
fn lists_the_same_module_once_even_under_two_names() {
    let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let module = directory.path().join("modulo.so");
    std::fs::write(&module, b"").expect("deberia poder escribirse el modulo");
    let link = directory.path().join("enlace.so");
    std::os::unix::fs::symlink(&module, &link).expect("deberia poder enlazarse");

    let candidates = [
        module.to_str().expect("ruta valida"),
        link.to_str().expect("ruta valida"),
    ];
    let stores = present_among(&candidates, |path| path.is_file());

    assert_eq!(stores, vec![module]);
}

#[test]
fn a_plain_module_has_nothing_to_configure() {
    assert_eq!(Store::module("/usr/lib/x.so").init_args(), None);
}

#[test]
fn a_plain_module_is_a_card() {
    assert_eq!(Store::module("/usr/lib/x.so").class(), StoreClass::Card);
}

#[test]
fn an_nss_store_is_classified_by_whose_profile_it_opens() {
    let firefox = Store::nss(
        "/usr/lib/libsoftokn3.so",
        Path::new("/casa/ada/.mozilla/firefox/aaaaaaaa.default-release"),
    );
    let chrome = Store::nss("/usr/lib/libsoftokn3.so", Path::new("/casa/ada/.pki/nssdb"));

    assert_eq!(firefox.class(), StoreClass::Firefox);
    assert_eq!(chrome.class(), StoreClass::Chrome);
}

#[test]
fn an_nss_store_is_classified_the_same_under_the_xdg_paths() {
    let firefox = Store::nss(
        "/usr/lib/libsoftokn3.so",
        Path::new("/casa/ada/.local/share/mozilla/firefox/cccccccc.default-release"),
    );
    let chrome = Store::nss(
        "/usr/lib/libsoftokn3.so",
        Path::new("/casa/ada/.local/share/pki/nssdb"),
    );

    assert_eq!(firefox.class(), StoreClass::Firefox);
    assert_eq!(chrome.class(), StoreClass::Chrome);
}

#[test]
fn an_nss_store_somewhere_else_claims_no_owner() {
    let store = Store::nss(
        "/usr/lib/libsoftokn3.so",
        Path::new("/tmp/perfil-de-pruebas"),
    );

    assert_eq!(store.class(), StoreClass::Nssdb);
}

#[test]
fn an_nss_store_opens_the_profile_read_only_and_in_sql_format() {
    let store = Store::nss("/usr/lib/libsoftokn3.so", Path::new("/casa/perfil"));
    let args = store.init_args().expect("un almacen NSS lleva init args");

    assert!(args.contains("configdir='sql:/casa/perfil'"), "{args}");
    assert!(args.contains("flags=readOnly"), "{args}");
}

fn a_home_with(profiles: &[(&str, bool)], ini: Option<&str>) -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
    let firefox = home.path().join(".mozilla/firefox");
    std::fs::create_dir_all(&firefox).expect("deberia poder crearse .mozilla/firefox");
    for (name, with_database) in profiles {
        let directory = home.path().join(name);
        std::fs::create_dir_all(&directory).expect("deberia poder crearse el perfil");
        if *with_database {
            std::fs::write(directory.join("cert9.db"), b"").expect("deberia poder escribirse");
        }
    }
    if let Some(ini) = ini {
        std::fs::write(firefox.join("profiles.ini"), ini).expect("deberia poder escribirse");
    }
    home
}

#[test]
fn reads_every_firefox_profile_declared_in_profiles_ini() {
    let home = a_home_with(
        &[
            (".mozilla/firefox/aaaaaaaa.default-release", true),
            (".mozilla/firefox/bbbbbbbb.trabajo", true),
        ],
        Some(
            "[Install4F96D1932A9F858E]\n\
             Default=aaaaaaaa.default-release\n\
             \n\
             [Profile0]\n\
             Name=default-release\n\
             IsRelative=1\n\
             Path=aaaaaaaa.default-release\n\
             \n\
             [Profile1]\n\
             Name=trabajo\n\
             IsRelative=1\n\
             Path=bbbbbbbb.trabajo\n",
        ),
    );

    assert_eq!(
        nss_profiles(home.path()),
        vec![
            home.path()
                .join(".mozilla/firefox/aaaaaaaa.default-release"),
            home.path().join(".mozilla/firefox/bbbbbbbb.trabajo"),
        ]
    );
}

#[test]
fn skips_a_declared_profile_without_a_certificate_database() {
    let home = a_home_with(
        &[
            (".mozilla/firefox/aaaaaaaa.vacio", false),
            (".mozilla/firefox/bbbbbbbb.lleno", true),
        ],
        Some("[Profile0]\nPath=aaaaaaaa.vacio\n\n[Profile1]\nPath=bbbbbbbb.lleno\n"),
    );

    assert_eq!(
        nss_profiles(home.path()),
        vec![home.path().join(".mozilla/firefox/bbbbbbbb.lleno")]
    );
}

#[test]
fn reads_the_shared_nssdb_too() {
    let home = a_home_with(&[(".pki/nssdb", true)], None);

    assert_eq!(
        nss_profiles(home.path()),
        vec![home.path().join(".pki/nssdb")]
    );
}

#[test]
fn has_no_profiles_when_firefox_is_not_installed() {
    let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");

    assert!(nss_profiles(home.path()).is_empty());
}

#[test]
fn reads_a_firefox_profile_from_the_paired_xdg_config_and_data_dirs() {
    let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
    let config = home.path().join(".config/mozilla/firefox");
    let data = home.path().join(".local/share/mozilla/firefox");
    let profile = data.join("cccccccc.default-release");
    std::fs::create_dir_all(&config).expect("deberia poder crearse .config/mozilla/firefox");
    std::fs::create_dir_all(&profile).expect("deberia poder crearse el perfil");
    std::fs::write(profile.join("cert9.db"), b"").expect("deberia poder escribirse");
    std::fs::write(
        config.join("profiles.ini"),
        "[Profile0]\nPath=cccccccc.default-release\n",
    )
    .expect("deberia poder escribirse");

    assert_eq!(nss_profiles(home.path()), vec![profile]);
}

#[test]
fn reads_the_xdg_shared_nssdb_too() {
    let home = a_home_with(&[(".local/share/pki/nssdb", true)], None);

    assert_eq!(
        nss_profiles(home.path()),
        vec![home.path().join(".local/share/pki/nssdb")]
    );
}

#[test]
fn resolves_an_absolute_profile_path_as_it_comes() {
    let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
    let firefox = home.path().join(".mozilla/firefox");
    let elsewhere = home.path().join("otro-sitio");
    std::fs::create_dir_all(&firefox).expect("deberia poder crearse .mozilla/firefox");
    std::fs::create_dir_all(&elsewhere).expect("deberia poder crearse el otro sitio");
    std::fs::write(elsewhere.join("cert9.db"), b"").expect("deberia poder escribirse");
    std::fs::write(
        firefox.join("profiles.ini"),
        format!("[Profile0]\nIsRelative=0\nPath={}\n", elsewhere.display()),
    )
    .expect("deberia poder escribirse");

    assert_eq!(nss_profiles(home.path()), vec![elsewhere]);
}
