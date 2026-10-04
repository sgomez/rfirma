use super::*;

fn scope_of_words(words: &[&str]) -> Result<StoreScope, StoreRefusal> {
    scope_of_words_on(Platform::Linux, words)
}

fn scope_of_words_on(platform: Platform, words: &[&str]) -> Result<StoreScope, StoreRefusal> {
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    scope_named_by(&arguments, platform)
}

#[test]
fn on_windows_windows_and_auto_narrow_to_the_windows_store_in_any_case() {
    for name in ["windows", "Windows", "auto", "AUTO"] {
        assert_eq!(
            scope_of_words_on(Platform::Windows, &["listaliases", "-store", name]),
            Ok(StoreScope::Windows),
            "{name}"
        );
    }
}

#[test]
fn on_windows_mozilla_and_the_stores_it_does_not_open_are_not_supported() {
    for value in ["mozilla", "pkcs12:/a.p12", "dni", "dnie", "mac"] {
        assert_eq!(
            scope_of_words_on(Platform::Windows, &["listaliases", "-store", value]),
            Err(StoreRefusal::NotSupported(value.to_owned())),
            "{value}"
        );
    }
}

#[test]
fn on_windows_pkcs11_and_unknown_names_behave_as_on_linux() {
    assert_eq!(
        scope_of_words_on(
            Platform::Windows,
            &["listaliases", "-store", "pkcs11:/m.so"]
        ),
        Ok(StoreScope::Module("/m.so".to_owned()))
    );
    assert_eq!(
        scope_of_words_on(Platform::Windows, &["listaliases", "-store", "pkcs11"]),
        Err(StoreRefusal::ModuleWithoutPath)
    );
    assert_eq!(
        scope_of_words_on(Platform::Windows, &["listaliases", "-store", "inventado"]),
        Err(StoreRefusal::Unknown("inventado".to_owned()))
    );
}

#[test]
fn without_store_the_search_is_everywhere() {
    assert_eq!(scope_of_words(&["listaliases"]), Ok(StoreScope::Everywhere));
}

#[test]
fn auto_and_mozilla_narrow_to_the_nss_family_in_any_case() {
    for name in ["auto", "mozilla", "Mozilla", "AUTO"] {
        assert_eq!(
            scope_of_words(&["listaliases", "-store", name]),
            Ok(StoreScope::Nss),
            "{name}"
        );
    }
}

#[test]
fn pkcs11_with_a_path_narrows_to_that_module() {
    assert_eq!(
        scope_of_words(&["listaliases", "-store", "pkcs11:/usr/lib/a.so"]),
        Ok(StoreScope::Module("/usr/lib/a.so".to_owned()))
    );
}

#[test]
fn pkcs11_without_a_path_is_refused() {
    for value in ["pkcs11", "pkcs11:", "pkcs11:  "] {
        assert_eq!(
            scope_of_words(&["listaliases", "-store", value]),
            Err(StoreRefusal::ModuleWithoutPath),
            "{value}"
        );
    }
}

#[test]
fn the_stores_of_the_original_rfirma_does_not_open_are_not_supported() {
    for value in [
        "pkcs12:/a.p12",
        "dni",
        "dnie",
        "windows",
        "mac",
        "auto:/x.so",
    ] {
        assert_eq!(
            scope_of_words(&["listaliases", "-store", value]),
            Err(StoreRefusal::NotSupported(value.to_owned())),
            "{value}"
        );
    }
}

#[test]
fn an_unknown_name_is_refused() {
    assert_eq!(
        scope_of_words(&["listaliases", "-store", "inventado"]),
        Err(StoreRefusal::Unknown("inventado".to_owned()))
    );
}

#[test]
fn store_without_a_value_is_refused() {
    assert_eq!(
        scope_of_words(&["listaliases", "-store"]),
        Err(StoreRefusal::Missing)
    );
}
