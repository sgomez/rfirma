use std::collections::BTreeSet;

use crate::crossing::Failure;
use crate::documents::domain::document::Document;
use crate::identity::application::tests::{a_certificate, a_usable_certificate};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::session::CycleFailure;
use crate::signing::application::tests::{
    an_order, ABridgeThatSigns, AnIsolateWith, DocumentsInMemory,
};
use crate::signing::domain::{PadesRect, PageSet, Placement, SignatureConfig};

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
        layer2_text: Some(String::new()),
        rubric_image: None,
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
    assert!(production_half().contains("invented_signatures()"));

    let invented = crate::signing::domain::TokenSignature::invented();
    assert_eq!(invented.raw().len(), 256, "una firma RSA de 2048 bits");
    assert!(
        invented.raw().iter().all(|byte| *byte == 0),
        "no lo ha calculado ningun token, y se nota"
    );
}

#[test]
fn only_the_dry_run_invents_a_pkcs1() {
    let signing = include_str!("../session.rs");

    assert!(
        !signing.contains("invented_signatures"),
        "el recorrido de la firma se esta inventando el PK1"
    );
}

fn compose_with(
    certificate: &TokenCertificate,
    bridge: &ABridgeThatSigns,
) -> Result<Vec<u8>, CycleFailure> {
    let order = an_order();
    let files = DocumentsInMemory::default().with(&order.document, b"%PDF-1.7 contenido");
    super::compose(
        &files,
        &Document::opened(order.document.clone()),
        certificate,
        &order.choice().expect("el recuadro cabe"),
        &AnIsolateWith(bridge),
    )
}

#[test]
fn the_preview_with_an_elliptic_curve_certificate_presigns_with_sha256_ecdsa() {
    let bridge = ABridgeThatSigns::default();

    compose_with(&a_usable_certificate("FIRMA"), &bridge).expect("la vista previa se compone");

    assert_eq!(bridge.calls()[0].algorithm, "SHA256withECDSA");
}

#[test]
fn the_preview_with_a_key_that_is_neither_rsa_nor_elliptic_is_refused_before_the_bridge() {
    let bridge = ABridgeThatSigns::default();

    let failure =
        compose_with(&a_certificate("FIRMA", &[]), &bridge).expect_err("esa clave no firma");

    assert_eq!(Failure::from(failure).situation, "keyKindUnsupported");
    assert!(bridge.calls().is_empty());
}
