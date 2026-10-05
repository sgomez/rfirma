use super::*;

fn parsed(words: &[&str]) -> Result<SignArguments, ArgumentsRefusal> {
    let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    parse_sign_arguments(&arguments, false)
}

#[test]
fn every_recognised_parameter_is_read() {
    let arguments = parsed(&[
        "-i",
        "a.pdf",
        "-o",
        "b.pdf",
        "-format",
        "PAdES",
        "-algorithm",
        "sha256",
        "-alias",
        "yo",
        "-config",
        "a=b",
        "-store",
        "pkcs11",
        "-password-fd",
        "3",
        "-xml",
    ])
    .expect("válidos");

    assert_eq!(arguments.input, "a.pdf");
    assert_eq!(arguments.output.as_deref(), Some("b.pdf"));
    assert_eq!(arguments.format, Format::Pades);
    assert_eq!(arguments.algorithm, Algorithm::Sha256);
    assert_eq!(arguments.selection, Some(Selection::Alias("yo".to_owned())));
    assert_eq!(arguments.store.as_deref(), Some("pkcs11"));
    assert_eq!(arguments.config.as_deref(), Some("a=b"));
    assert_eq!(arguments.password_fd, Some(3));
    assert!(arguments.xml);
}

#[test]
fn format_and_algorithm_default_to_auto_and_sha512() {
    let arguments = parsed(&["-i", "a", "-o", "b", "-alias", "yo"]).expect("válidos");

    assert_eq!(arguments.format, Format::Auto);
    assert_eq!(arguments.algorithm, Algorithm::Sha512);
}

#[test]
fn the_output_is_required_unless_xml() {
    assert_eq!(
        parsed(&["-i", "a", "-alias", "yo"]),
        Err(ArgumentsRefusal::MissingOutput)
    );
    assert!(parsed(&["-i", "a", "-alias", "yo", "-xml"]).is_ok());
}

#[test]
fn the_input_is_required() {
    assert_eq!(
        parsed(&["-o", "b", "-alias", "yo"]),
        Err(ArgumentsRefusal::MissingInput)
    );
}

#[test]
fn sha1_is_refused() {
    assert_eq!(
        parsed(&["-i", "a", "-o", "b", "-alias", "yo", "-algorithm", "SHA1"]),
        Err(ArgumentsRefusal::Sha1Refused)
    );
}

#[test]
fn sha1_is_read_when_the_preference_allows_it() {
    let arguments: Vec<String> = ["-i", "a", "-o", "b", "-alias", "yo", "-algorithm", "SHA1"]
        .iter()
        .map(|word| (*word).to_owned())
        .collect();

    let parsed = parse_sign_arguments(&arguments, true).expect("válidos");

    assert_eq!(parsed.algorithm, Algorithm::Sha1);
}

#[test]
fn facturae_ooxml_and_odf_are_unsupported_formats() {
    for format in ["facturae", "ooxml", "odf"] {
        assert_eq!(
            parsed(&["-i", "a", "-o", "b", "-alias", "yo", "-format", format]),
            Err(ArgumentsRefusal::UnsupportedFormat(format.to_owned()))
        );
    }
}

#[test]
fn exactly_one_way_of_choosing_the_certificate_is_required() {
    let base = ["-i", "a", "-o", "b"];
    for extra in [
        &[][..],
        &["-alias", "yo", "-certtui"][..],
        &["-alias", "yo", "-filter", "x"][..],
        &["-certgui", "-certtui"][..],
    ] {
        let words: Vec<&str> = base.iter().chain(extra).copied().collect();
        assert!(parsed(&words).is_err(), "{extra:?}");
    }
}

#[test]
fn a_filter_goes_alone_or_narrows_the_terminal_list() {
    let alone = parsed(&["-i", "a", "-o", "b", "-filter", "x"]).expect("válido");
    let narrowing = parsed(&["-i", "a", "-o", "b", "-certtui", "-filter", "x"]).expect("válido");

    assert_eq!(alone.selection, Some(Selection::Filter("x".to_owned())));
    assert_eq!(
        narrowing.selection,
        Some(Selection::Terminal {
            filter: Some("x".to_owned())
        })
    );
}

#[test]
fn certgui_chooses_in_the_window_and_a_filter_narrows_its_list() {
    let alone = parsed(&["-i", "a", "-o", "b", "-certgui"]).expect("válido");
    let narrowing = parsed(&["-i", "a", "-o", "b", "-certgui", "-filter", "x"]).expect("válido");

    assert_eq!(alone.selection, Some(Selection::Window { filter: None }));
    assert_eq!(
        narrowing.selection,
        Some(Selection::Window {
            filter: Some("x".to_owned())
        })
    );
}

#[test]
fn an_unknown_argument_a_missing_value_or_a_bad_descriptor_is_refused() {
    assert!(matches!(
        parsed(&["-i", "a", "-zzz"]),
        Err(ArgumentsRefusal::UnknownArgument(_))
    ));
    assert_eq!(
        parsed(&["-i", "a", "-o"]),
        Err(ArgumentsRefusal::MissingValue("-o"))
    );
    assert_eq!(
        parsed(&["-i", "a", "-o", "b", "-alias"])
            .unwrap_err()
            .to_string(),
        "el parámetro --alias necesita un valor"
    );
    assert!(matches!(
        parsed(&["-i", "a", "-o", "b", "-alias", "yo", "-password-fd", "x"]),
        Err(ArgumentsRefusal::InvalidDescriptor(_))
    ));
}

#[test]
fn an_unknown_argument_is_named_in_the_double_dash_form_only_when_it_is_a_known_option() {
    let refused = |words: &[&str]| parsed(words).unwrap_err().to_string();
    assert_eq!(
        refused(&["-i", "a", "-o", "b", "-alias", "yo", "-preurl", "x"]),
        "el argumento «--preurl» no se reconoce"
    );
    assert_eq!(
        refused(&["-i", "a", "-zzz"]),
        "el argumento «-zzz» no se reconoce"
    );
}

#[test]
fn gui_hands_the_file_to_the_window_without_the_other_requirements() {
    let arguments = parsed(&["-gui", "-i", "a.pdf"]).expect("válido");

    assert!(arguments.hand_to_window);
    assert_eq!(arguments.selection, None);
}
