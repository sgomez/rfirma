use super::*;

#[test]
fn each_situation_has_its_own_spanish_phrase() {
    let cases = [
        (Situation::HomeUnknown, "carpeta personal"),
        (Situation::ScratchFolderUnusable, "carpeta de paso"),
        (Situation::WindowUnavailable, "ventana"),
    ];

    for (situation, fragment) in cases {
        let failure = StartupFailure::new(situation, "detalle crudo");

        assert_eq!(failure.situation(), situation);
        assert!(
            failure.phrase().contains(fragment),
            "la frase de {situation:?} debería mencionar «{fragment}»: {}",
            failure.phrase()
        );
    }
}

#[test]
fn the_detail_is_kept_raw_and_never_translated() {
    let failure = StartupFailure::new(Situation::HomeUnknown, "falta la variable de entorno HOME");

    assert_eq!(failure.detail(), "falta la variable de entorno HOME");
    assert!(failure
        .to_string()
        .contains("falta la variable de entorno HOME"));
}
