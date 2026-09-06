use super::*;

#[test]
fn a_site_that_demands_nothing_is_served() {
    assert!(check_minimum_client_version(None).is_ok());
    assert!(check_minimum_client_version(Some("")).is_ok());
}

#[test]
fn the_versions_the_sites_actually_demand_are_served() {
    for requested in ["1.6", "1.7", "1.8", "1.9", "1.9.2"] {
        assert!(
            check_minimum_client_version(Some(requested)).is_ok(),
            "una sede que exige {requested} tiene que poder firmar"
        );
    }
}

#[test]
fn a_version_newer_than_the_one_implemented_is_refused_with_its_own_code() {
    let refusal =
        check_minimum_client_version(Some("1.9.3")).expect_err("no se implementa la 1.9.3");

    assert_eq!(refusal.code(), SafCode::MinimumVersionNonSatisfied);
}

#[test]
fn the_comparison_is_against_autofirma_and_not_against_the_version_of_rfirma() {
    assert_eq!(IMPLEMENTED_AUTOFIRMA_VERSION, "1.9.2");
    assert!(
        check_minimum_client_version(Some("1.9")).is_ok(),
        "con la version de rFirma —0.x— esta exigencia daria SAF_41 y no se firmaria nunca"
    );
}

#[test]
fn a_minimum_version_that_does_not_parse_is_a_parameter_error() {
    for requested in ["ultima", "1.a", "1..9"] {
        let refusal = check_minimum_client_version(Some(requested)).expect_err("no es una version");

        assert_eq!(refusal.code(), SafCode::Params, "con {requested}");
    }
}

#[test]
fn data_that_asks_for_a_local_file_is_refused() {
    for data in [
        "file:/etc/passwd",
        "file:///etc/passwd",
        "FILE:/etc/passwd",
        "  file:/x",
    ] {
        let refusal = check_local_access_is_not_requested(data)
            .expect_err("la sede no elige que ficheros se leen");

        assert_eq!(refusal.code(), SafCode::Params, "con {data}");
    }
}

#[test]
fn base64_data_goes_through() {
    assert!(check_local_access_is_not_requested("JVBERi0xLjcKJeLjz9M").is_ok());
    assert!(check_local_access_is_not_requested("").is_ok());
}
