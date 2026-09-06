use std::collections::BTreeSet;

use crate::signing::{PadesRect, PageSet, Placement, SignatureConfig};

fn production_half() -> &'static str {
    include_str!("../preview.rs")
}

#[test]
fn a_page_set_of_twenty_travels_as_a_single_presign_request() {
    let pages = PageSet::only(1..=20).expect("veinte paginas no es vacio");
    let config = SignatureConfig {
        placement: Some(Placement {
            rect: PadesRect {
                lower_left_x: 48,
                lower_left_y: 179,
                upper_right_x: 250,
                upper_right_y: 260,
            },
            pages,
        }),
        layer2_text: String::new(),
        rubric_image: None,
        sign_reason: None,
        allow_unregistered_signatures: false,
    };

    let params = config.extra_params();

    assert_eq!(
        params.get("signaturePages").map(String::as_str),
        Some("1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20"),
        "el conjunto entero viaja en un solo extraParams"
    );
}

#[test]
fn the_bridge_is_crossed_twice_and_never_once_per_page() {
    let source = production_half();

    assert_eq!(
        source.matches("cycle::presign(").count(),
        1,
        "la prefirma se pide una sola vez, sea cual sea el conjunto de paginas"
    );
    assert_eq!(
        source.matches(".postsign(").count(),
        1,
        "y la postfirma ensambla una sola vez"
    );
    for loop_keyword in ["for ", "while ", ".iter()", ".map("] {
        assert!(
            !source.contains(loop_keyword),
            "«{loop_keyword}» en la vista previa: una prefirma por pagina es incorrecto"
        );
    }
}

#[test]
fn the_dry_run_neither_asks_for_the_pin_nor_writes_anything() {
    let source = production_half();

    let words: BTreeSet<&str> = source
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect();
    for forbidden in ["pin", "sign_on_token", "deliver", "note_signed", "Signed"] {
        assert!(
            !words.contains(forbidden),
            "«{forbidden}» en la vista previa: deja de ser en seco"
        );
    }
}

#[test]
fn the_pkcs1_of_the_dry_run_is_the_invented_one() {
    assert!(production_half().contains("TokenSignature::invented()"));

    let invented = super::TokenSignature::invented();
    assert_eq!(invented.raw().len(), 256, "una firma RSA de 2048 bits");
    assert!(
        invented.raw().iter().all(|byte| *byte == 0),
        "no lo ha calculado ningun token, y se nota"
    );
}

#[test]
fn only_the_dry_run_invents_a_pkcs1() {
    let signing = include_str!("../signing/mod.rs");

    assert!(
        !signing.contains("TokenSignature::invented"),
        "el recorrido de la firma se esta inventando el PK1"
    );
}
