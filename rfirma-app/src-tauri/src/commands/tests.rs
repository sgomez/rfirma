use super::{pades_lower_left, PlacementOrder};

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
