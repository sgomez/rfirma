use super::*;

fn a_url(parameters: &str) -> AfirmaUrl {
    AfirmaUrl::parse(&format!("afirma://selectcert?op=selectcert{parameters}"))
        .expect("es del protocolo")
}

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

#[test]
fn the_two_sticky_flags_are_read_regardless_of_case_and_default_to_off() {
    assert_eq!(sticky_certificate(&a_url("")), StickyCertificate::default());
    assert!(!sticky_certificate(&a_url("")).is_sticky());
    assert!(!sticky_certificate(&a_url("")).resets());

    let asked = sticky_certificate(&a_url("&sticky=TRUE&resetsticky=True"));
    assert!(asked.is_sticky());
    assert!(asked.resets());
}

#[test]
fn a_sticky_value_that_is_not_true_is_not_a_refusal() {
    let asked = sticky_certificate(&a_url("&sticky=loquesea&resetsticky="));

    assert!(!asked.is_sticky());
    assert!(!asked.resets());
}

#[test]
fn the_sticky_flags_are_not_trimmed_like_boolean_parse_boolean_does_not_trim() {
    let sticky = sticky_certificate(&a_url("&sticky=%20true&resetsticky=true%20"));

    assert!(!sticky.is_sticky());
    assert!(!sticky.resets());
}

#[test]
fn a_servlet_url_over_http_is_accepted_like_the_original_accepts_it() {
    assert!(check_servlet_url("http://relay.example/store", Parameter::StoreServlet).is_ok());
    assert!(check_servlet_url("https://relay.example/store", Parameter::StoreServlet).is_ok());
}

#[test]
fn a_servlet_url_on_a_local_host_is_a_local_access_attempt() {
    for candidate in [
        "https://localhost/store",
        "http://127.0.0.1:8080/store",
        "https://LOCALHOST/store",
    ] {
        let refusal = check_servlet_url(candidate, Parameter::RetrieveServlet)
            .expect_err("el original lo rechaza con su propio codigo");

        assert_eq!(
            refusal.code(),
            SafCode::LocalAccessBlocked,
            "con {candidate}"
        );
    }
}

#[test]
fn a_servlet_url_that_is_not_absolute_is_a_parameter_error() {
    let refusal =
        check_servlet_url("/store", Parameter::StoreServlet).expect_err("una url relativa no vale");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::StoreServlet));
}
