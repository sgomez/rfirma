use super::{PadesRect, PageSet, Placement, Setting, SignatureConfig, SUB_FILTER};
use std::collections::HashSet;

fn a_rect() -> PadesRect {
    PadesRect {
        lower_left_x: 100,
        lower_left_y: 200,
        upper_right_x: 300,
        upper_right_y: 260,
    }
}

fn placed_on(pages: PageSet) -> Option<Placement> {
    Some(Placement {
        rect: a_rect(),
        pages,
    })
}

fn minimal() -> SignatureConfig {
    SignatureConfig {
        placement: placed_on(PageSet::only_page(3)),
        layer2_text: "Firmado por: Ada Lovelace Byron".to_owned(),
        rubric_image: None,
        sign_reason: None,
        allow_unregistered_signatures: false,
    }
}

fn complete() -> SignatureConfig {
    SignatureConfig {
        rubric_image: Some("/9j/4AAQSkZJRg==".to_owned()),
        sign_reason: Some("Conforme".to_owned()),
        allow_unregistered_signatures: true,
        ..minimal()
    }
}

#[test]
fn closes_the_configuration_and_this_is_how_many_settings_it_has() {
    assert_eq!(Setting::ALL.len(), 7);
}

#[test]
fn emits_no_key_outside_the_five_settings() {
    let owned: HashSet<&str> = Setting::ALL
        .iter()
        .flat_map(|setting| setting.keys().iter().copied())
        .collect();

    for config in [minimal(), complete()] {
        for key in config.extra_params().keys() {
            assert!(
                owned.contains(key.as_str()),
                "«{key}» no pertenece a ninguno de los seis ajustes"
            );
        }
    }
}

#[test]
fn emits_every_key_the_settings_declare() {
    // La dirección contraria a la de arriba: una clave declarada en
    // `Setting::keys()` que `extra_params` no llegue a emitir nunca sería
    // una promesa muerta. `complete()` tiene los siete ajustes puestos, así
    // que sobre ella la contención va en los dos sentidos.
    let declared: HashSet<&str> = Setting::ALL
        .iter()
        .flat_map(|setting| setting.keys().iter().copied())
        .collect();
    let emitted = complete().extra_params();

    for key in declared {
        assert!(
            emitted.contains_key(key),
            "«{key}» lo declara un ajuste y no lo emite nadie"
        );
    }
}

#[test]
fn gives_every_setting_its_own_keys() {
    let mut seen: HashSet<&str> = HashSet::new();
    for setting in Setting::ALL {
        for key in setting.keys() {
            assert!(seen.insert(key), "«{key}» lo emiten dos ajustes");
        }
    }
}

#[test]
fn sends_the_sub_filter_explicitly() {
    assert_eq!(
        minimal().extra_params().get("signatureSubFilter"),
        Some(&SUB_FILTER.to_owned())
    );
}

#[test]
fn sends_the_geometry_of_the_box() {
    let params = minimal().extra_params();
    assert_eq!(params.get("signaturePages"), Some(&"3".to_owned()));
    assert_eq!(
        params.get("signaturePositionOnPageLowerLeftX"),
        Some(&"100".to_owned())
    );
    assert_eq!(
        params.get("signaturePositionOnPageLowerLeftY"),
        Some(&"200".to_owned())
    );
    assert_eq!(
        params.get("signaturePositionOnPageUpperRightX"),
        Some(&"300".to_owned())
    );
    assert_eq!(
        params.get("signaturePositionOnPageUpperRightY"),
        Some(&"260".to_owned())
    );
}

#[test]
fn always_sends_the_layer2_text_even_when_it_is_empty() {
    let config = SignatureConfig {
        layer2_text: String::new(),
        ..minimal()
    };
    assert_eq!(
        config.extra_params().get("layer2Text"),
        Some(&String::new())
    );
}

#[test]
fn always_sends_the_font_size_as_zero() {
    for config in [minimal(), complete()] {
        assert_eq!(
            config.extra_params().get("layer2FontSize"),
            Some(&"0".to_owned())
        );
    }
}

#[test]
fn omits_the_rubric_and_the_reason_when_there_are_none() {
    let params = minimal().extra_params();
    assert!(!params.contains_key("signatureRubricImage"));
    assert!(!params.contains_key("signReason"));
}

#[test]
fn sends_the_rubric_and_the_reason_when_there_are() {
    let params = complete().extra_params();
    assert_eq!(
        params.get("signatureRubricImage"),
        Some(&"/9j/4AAQSkZJRg==".to_owned())
    );
    assert_eq!(params.get("signReason"), Some(&"Conforme".to_owned()));
}

#[test]
fn never_sends_what_the_spec_ruled_out() {
    let ruled_out = [
        "signReservedSize",
        "policyIdentifier",
        "policyIdentifierHash",
        "signatureProductionCity",
        "signerContact",
        "profile",
        "doNotUseCertChainOnPostSign",
        "includeOnlySignningCertificate",
    ];
    let params = complete().extra_params();
    for key in ruled_out {
        assert!(!params.contains_key(key), "«{key}» no debería enviarse");
    }
}

#[test]
fn never_sends_the_singular_page_key() {
    assert!(!complete().extra_params().contains_key("signaturePage"));
}

#[test]
fn says_nothing_to_the_bridge_about_unregistered_signatures_until_someone_consents() {
    assert!(!minimal()
        .extra_params()
        .contains_key("allowCosigningUnregisteredSignatures"));

    let consented = SignatureConfig {
        allow_unregistered_signatures: true,
        ..minimal()
    };

    assert_eq!(
        consented
            .extra_params()
            .get("allowCosigningUnregisteredSignatures"),
        Some(&"true".to_owned())
    );
}

#[test]
fn writes_the_page_set_as_the_bridge_reads_it() {
    for (pages, literal) in [
        (PageSet::only_page(3), "3"),
        (PageSet::only([7, 3, 3]).expect("no esta vacio"), "3,7"),
        (PageSet::All, "all"),
    ] {
        let config = SignatureConfig {
            placement: placed_on(pages),
            ..minimal()
        };
        assert_eq!(
            config.extra_params().get("signaturePages"),
            Some(&literal.to_owned())
        );
    }
}

#[test]
fn emits_no_geometry_at_all_when_the_box_is_not_placed_by_rfirma() {
    let config = SignatureConfig {
        placement: None,
        ..complete()
    };

    let params = config.extra_params();

    for key in Setting::Geometry.keys() {
        assert!(
            !params.contains_key(*key),
            "'{key}' la pone la sede, no rFirma"
        );
    }
    assert!(params.contains_key("signatureSubFilter"), "lo demas sigue");
}

#[test]
fn changes_nothing_but_the_page_set_between_all_and_the_full_list() {
    let all = SignatureConfig {
        placement: placed_on(PageSet::All),
        ..complete()
    };
    let listed = SignatureConfig {
        placement: placed_on(PageSet::only([1, 2, 3]).expect("no esta vacio")),
        ..complete()
    };

    let mut left = all.extra_params();
    let mut right = listed.extra_params();
    assert_ne!(
        left.remove("signaturePages"),
        right.remove("signaturePages")
    );
    assert_eq!(left, right);
}
