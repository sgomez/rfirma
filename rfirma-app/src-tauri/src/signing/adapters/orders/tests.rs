use super::PlacementOrder;
use crate::signing::domain::PageSet;

#[test]
fn accepts_a_rect_with_the_fractional_coordinates_the_viewer_sends() {
    let placement: PlacementOrder = serde_json::from_value(sent_from_the_viewer())
        .expect("el recuadro del visor tiene decimales");

    let placed = placement.placement().expect("cabe en la pagina");
    assert_eq!(
        (
            placed.rect.lower_left_x,
            placed.rect.lower_left_y,
            placed.rect.upper_right_x,
            placed.rect.upper_right_y
        ),
        (48, 179, 250, 260)
    );
}

fn sent_from_the_viewer() -> serde_json::Value {
    serde_json::json!({
        "page": 1,
        "pages": { "only": [1] },
        "pageCount": 3,
        "mediaBox": [0.0, 0.0, 595.276, 841.89],
        "rotation": 0,
        "rect": [47.7218, 179.1376722440945, 250.1, 259.9],
    })
}

fn order_placed_on(pages: serde_json::Value, page_count: u32) -> PlacementOrder {
    let mut sent = sent_from_the_viewer();
    sent["pages"] = pages;
    sent["pageCount"] = serde_json::json!(page_count);
    serde_json::from_value(sent).expect("la orden del visor")
}

#[test]
fn refuses_a_destination_the_document_does_not_have_before_calling_the_bridge() {
    let failure = order_placed_on(serde_json::json!({ "only": [99] }), 3)
        .placement()
        .expect_err("un documento de tres paginas no tiene la 99");

    let failure = crate::commands::Failure::from(failure);
    assert_eq!(failure.situation, "pageOutOfDocument");
    assert!(failure.detail.contains("99"), "{}", failure.detail);
}

#[test]
fn refuses_a_drag_page_the_document_does_not_have() {
    let mut sent = sent_from_the_viewer();
    sent["page"] = serde_json::json!(9);
    sent["pages"] = serde_json::json!("all");
    let order: PlacementOrder = serde_json::from_value(sent).expect("la orden del visor");

    assert_eq!(
        crate::commands::Failure::from(order.placement().expect_err("la 9 no existe")).situation,
        "pageOutOfDocument"
    );
}

#[test]
fn carries_the_page_set_through_to_the_placement() {
    let placed = order_placed_on(serde_json::json!("all"), 3)
        .placement()
        .expect("cabe y existe");
    assert_eq!(placed.pages, PageSet::All);

    let placed = order_placed_on(serde_json::json!({ "only": [3, 1] }), 3)
        .placement()
        .expect("cabe y existe");
    assert_eq!(placed.pages, PageSet::only([1, 3]).expect("no esta vacio"));
}
