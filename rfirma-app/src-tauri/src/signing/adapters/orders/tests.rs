use super::{PlacementOrder, SigningOrder};
use crate::signing::domain::{Datum, PageSet, PhrasePart, VisibleContent};

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

    let failure = crate::crossing::Failure::from(failure);
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
        crate::crossing::Failure::from(order.placement().expect_err("la 9 no existe")).situation,
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

fn a_signing_order() -> serde_json::Value {
    serde_json::json!({
        "document": "/run/user/1000/doc/1e8b83b9/contrato.pdf",
        "certificate": "FIRMA",
        "placement": null,
        "content": { "model": "complete" },
        "signedAt": "31/08/26, 12:00:00",
        "rubric": null,
        "language": "es",
    })
}

#[test]
fn an_order_composes_the_choice_from_its_model() {
    let mut sent = a_signing_order();
    sent["content"] = serde_json::json!({ "model": "custom", "phrase": [{ "datum": "issuer" }, { "text": "." }] });
    sent["withRubric"] = serde_json::json!(true);

    let order: SigningOrder = serde_json::from_value(sent).expect("la orden por modelo se acepta");

    assert_eq!(
        order.choice().expect("sin recuadro").content,
        VisibleContent::Custom(vec![
            PhrasePart::Datum(Datum::Issuer),
            PhrasePart::Text(".".to_owned()),
        ])
    );
    assert!(order.with_rubric);
}

#[test]
fn a_model_the_window_does_not_know_is_refused() {
    let mut sent = a_signing_order();
    sent["content"] = serde_json::json!({ "model": "stamp" });

    assert!(serde_json::from_value::<SigningOrder>(sent).is_err());
}
