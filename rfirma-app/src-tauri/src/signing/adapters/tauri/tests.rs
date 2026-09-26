use super::{
    pades_lower_left, remember_the_visible_signature_ordered, Memory, PlacementOrder, SigningOrder,
    VisibleContent,
};
use crate::desktop::adapters::paths::Paths;
use crate::signing::adapters::orders::{DatumOrder, PhrasePartOrder, VisibleContentOrder};
use crate::signing::application::tests::an_order;

/// El cuerpo de la prefirma, para las guardas que leen su propio codigo fuente.
const THE_ORDERS: &str = include_str!("../tauri.rs");

#[test]
fn only_begin_signing_calls_remember_the_visible_signature_ordered() {
    assert_eq!(
        THE_ORDERS
            .matches("remember_the_visible_signature_ordered(&order")
            .count(),
        1,
        "se llama desde un solo sitio"
    );
    let prefirma = THE_ORDERS
        .split_once("pub fn begin_signing(")
        .expect("la prefirma sigue aqui")
        .1
        .split_once("\n}")
        .expect("y tiene cuerpo")
        .0;
    assert!(
        prefirma.contains("remember_the_visible_signature_ordered(&order"),
        "y esa llamada esta en la prefirma"
    );
}

#[test]
fn the_model_ordered_in_the_prefirma_comes_back_in_the_next_session() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = Memory::at(&Paths::under(directory.path()));
    let content = VisibleContentOrder::Custom {
        phrase: vec![
            PhrasePartOrder::Text {
                text: "Conforme, ".to_owned(),
            },
            PhrasePartOrder::Datum {
                datum: DatumOrder::Signer,
            },
        ],
    };
    let order = SigningOrder {
        content: Some(content.clone()),
        with_rubric: true,
        ..an_order()
    };

    remember_the_visible_signature_ordered(&order, &memory);

    let next_session = Memory::at(&Paths::under(directory.path()));
    let remembered = next_session.remembered_visible_signature();
    assert_eq!(remembered.content, Some(VisibleContent::from(&content)));
    assert!(remembered.rubric);
}

#[test]
fn an_order_without_content_does_not_touch_the_memory() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = Memory::at(&Paths::under(directory.path()));

    remember_the_visible_signature_ordered(&an_order(), &memory);

    assert_eq!(memory.remembered_visible_signature().content, None);
}

#[test]
fn matches_user_space_when_the_page_is_not_rotated() {
    let placement: PlacementOrder = serde_json::from_value(serde_json::json!({
        "page": 1,
        "pages": { "only": [1] },
        "pageCount": 1,
        "mediaBox": [0.0, 0.0, 595.0, 842.0],
        "rotation": 0,
        "rect": [250.0, 50.0, 450.0, 100.0],
    }))
    .expect("la orden del visor");

    assert_eq!(
        pades_lower_left(placement).expect("cabe en la pagina"),
        [250, 50]
    );
}

#[test]
fn diverges_from_user_space_when_the_page_is_rotated() {
    let placement: PlacementOrder = serde_json::from_value(serde_json::json!({
        "page": 1,
        "pages": { "only": [1] },
        "pageCount": 1,
        "mediaBox": [0.0, 0.0, 595.0, 842.0],
        "rotation": 90,
        "rect": [250.0, 50.0, 450.0, 100.0],
    }))
    .expect("la orden del visor");

    assert_eq!(
        pades_lower_left(placement).expect("cabe en la pagina"),
        [50, 145]
    );
}

#[test]
fn no_order_of_the_window_can_ask_for_a_format() {
    for source in [
        include_str!("../tauri.rs"),
        include_str!("../orders.rs"),
        include_str!("../views.rs"),
    ] {
        assert!(
            !source.contains("Format"),
            "la ventana principal solo firma PAdES: el formato lo fija la raiz, no la orden"
        );
    }
}
