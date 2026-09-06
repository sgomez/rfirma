use super::{MediaBox, OutOfPage, Page, PageSet, Rotation, UserSpaceRect, ViewerRect};

const DRAG: ViewerRect = ViewerRect {
    x0: 60.0,
    y0: 80.0,
    x1: 260.0,
    y1: 160.0,
};

const A4: [f64; 4] = [0.0, 0.0, 595.0, 842.0];
const A5: [f64; 4] = [0.0, 0.0, 420.0, 595.0];
const LETTER: [f64; 4] = [0.0, 0.0, 612.0, 792.0];
const OFFSET: [f64; 4] = [20.0, 30.0, 615.0, 872.0];

struct Case {
    name: &'static str,
    page: u32,
    media_box: [f64; 4],
    rotate: i32,
    zoom: f64,
    widget: [i32; 4],
    params: [i32; 4],
}

#[rustfmt::skip]
const BANK: [Case; 16] = [
    Case { name: "a4",                 page: 1, media_box: A4,     rotate: 0,   zoom: 1.0,  widget: [60, 682, 260, 762],  params: [60, 682, 260, 762] },
    Case { name: "a4-rot90",           page: 1, media_box: A4,     rotate: 90,  zoom: 1.0,  widget: [80, 60, 160, 260],   params: [60, 435, 260, 515] },
    Case { name: "a4-rot180",          page: 1, media_box: A4,     rotate: 180, zoom: 1.0,  widget: [335, 80, 535, 160],  params: [60, 682, 260, 762] },
    Case { name: "a4-rot270",          page: 1, media_box: A4,     rotate: 270, zoom: 1.0,  widget: [435, 582, 515, 782], params: [60, 435, 260, 515] },
    Case { name: "a5",                 page: 1, media_box: A5,     rotate: 0,   zoom: 1.0,  widget: [60, 435, 260, 515],  params: [60, 435, 260, 515] },
    Case { name: "letter",             page: 1, media_box: LETTER, rotate: 0,   zoom: 1.0,  widget: [60, 632, 260, 712],  params: [60, 632, 260, 712] },
    Case { name: "offset",             page: 1, media_box: OFFSET, rotate: 0,   zoom: 1.0,  widget: [80, 712, 280, 792],  params: [80, 712, 280, 792] },
    Case { name: "offset-rot90",       page: 1, media_box: OFFSET, rotate: 90,  zoom: 1.0,  widget: [100, 90, 180, 290],  params: [90, 435, 290, 515] },
    Case { name: "offset-rot180",      page: 1, media_box: OFFSET, rotate: 180, zoom: 1.0,  widget: [355, 110, 555, 190], params: [60, 682, 260, 762] },
    Case { name: "offset-rot270",      page: 1, media_box: OFFSET, rotate: 270, zoom: 1.0,  widget: [455, 612, 535, 812], params: [60, 455, 260, 535] },
    Case { name: "mixto-p1",           page: 1, media_box: A4,     rotate: 0,   zoom: 1.0,  widget: [60, 682, 260, 762],  params: [60, 682, 260, 762] },
    Case { name: "mixto-p2",           page: 2, media_box: A5,     rotate: 90,  zoom: 1.0,  widget: [80, 60, 160, 260],   params: [60, 260, 260, 340] },
    Case { name: "mixto-p3",           page: 3, media_box: OFFSET, rotate: 180, zoom: 1.0,  widget: [355, 110, 555, 190], params: [60, 682, 260, 762] },
    Case { name: "a4-zoom175",         page: 1, media_box: A4,     rotate: 0,   zoom: 1.75, widget: [34, 751, 149, 796],  params: [34, 751, 149, 796] },
    Case { name: "a4rot90-zoom06",     page: 1, media_box: A4,     rotate: 90,  zoom: 0.6,  widget: [133, 100, 267, 433], params: [100, 328, 433, 462] },
    Case { name: "offrot270-zoom175",  page: 1, media_box: OFFSET, rotate: 270, zoom: 1.75, widget: [524, 723, 569, 838], params: [34, 524, 149, 569] },
];

fn page_of(case: &Case) -> Page {
    let [x0, y0, x1, y1] = case.media_box;
    Page {
        number: case.page,
        media_box: MediaBox::new(x0, y0, x1, y1),
        rotation: Rotation::from_degrees(case.rotate).expect("rotación del banco"),
    }
}

fn rect(values: [i32; 4]) -> UserSpaceRect {
    UserSpaceRect {
        lower_left_x: values[0],
        lower_left_y: values[1],
        upper_right_x: values[2],
        upper_right_y: values[3],
    }
}

#[test]
fn converts_the_drag_to_user_space_like_pdfjs_does() {
    for case in &BANK {
        assert_eq!(
            page_of(case).to_user_space(&DRAG, case.zoom),
            rect(case.widget),
            "paso 1 del caso «{}»",
            case.name
        );
    }
}

#[test]
fn matches_every_measured_case_of_the_bank() {
    for case in &BANK {
        let placed = page_of(case)
            .place(&DRAG, case.zoom)
            .unwrap_or_else(|error| panic!("caso «{}»: {error}", case.name));
        assert_eq!(
            [
                placed.lower_left_x,
                placed.lower_left_y,
                placed.upper_right_x,
                placed.upper_right_y
            ],
            case.params,
            "paso 2 del caso «{}»",
            case.name
        );
    }
}

#[test]
fn covers_the_four_rotations_with_a_displaced_media_box() {
    for degrees in [90, 180, 270] {
        assert!(
            BANK.iter().any(|case| case.media_box == OFFSET
                && case.rotate == degrees
                && case.media_box[0] != 0.0),
            "el banco se ha quedado sin el caso de MediaBox desplazada a {degrees}°"
        );
    }
}

#[test]
fn uses_the_upper_bounds_of_the_media_box_and_not_the_width() {
    let at_origin = Page {
        number: 1,
        media_box: MediaBox::new(0.0, 0.0, 595.0, 842.0),
        rotation: Rotation::Quarter,
    };
    let displaced = Page {
        media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
        ..at_origin
    };
    let same_rect = rect([100, 200, 300, 260]);

    assert_ne!(
        at_origin.pades_rect(&same_rect).expect("cabe"),
        displaced.pades_rect(&same_rect).expect("cabe"),
    );
}

#[test]
fn emits_integer_coordinates() {
    let page = Page {
        number: 1,
        media_box: MediaBox::new(0.0, 0.0, 595.0, 842.0),
        rotation: Rotation::None,
    };
    let placed = page
        .place(
            &ViewerRect {
                x0: 60.4,
                y0: 80.7,
                x1: 260.2,
                y1: 160.9,
            },
            1.0,
        )
        .expect("cabe");
    assert_eq!(
        [
            placed.lower_left_x,
            placed.lower_left_y,
            placed.upper_right_x,
            placed.upper_right_y
        ],
        [60, 681, 260, 761]
    );
}

#[test]
fn keeps_the_box_still_when_the_zoom_changes() {
    let page = Page {
        number: 1,
        media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
        rotation: Rotation::ThreeQuarters,
    };
    let at_one = page.to_user_space(&DRAG, 1.0);
    for zoom in [0.5, 1.75, 3.0] {
        let scaled = ViewerRect {
            x0: DRAG.x0 * zoom,
            y0: DRAG.y0 * zoom,
            x1: DRAG.x1 * zoom,
            y1: DRAG.y1 * zoom,
        };
        assert_eq!(
            page.to_user_space(&scaled, zoom),
            at_one,
            "el zoom {zoom} ha movido el recuadro"
        );
    }
}

#[test]
fn rejects_a_box_that_would_fall_off_the_page() {
    let page = Page {
        number: 4,
        media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
        rotation: Rotation::None,
    };
    let error = page
        .pades_rect(&rect([500, 700, 700, 780]))
        .expect_err("un recuadro que se sale no puede firmarse");
    assert_eq!(
        error,
        OutOfPage {
            page: 4,
            rect: [500, 700, 700, 780],
            media_box: [20, 30, 615, 872],
        }
    );
}

#[test]
fn says_which_limit_the_box_crossed() {
    let page = Page {
        number: 1,
        media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
        rotation: Rotation::None,
    };
    let message = page
        .pades_rect(&rect([500, 700, 700, 780]))
        .expect_err("se sale")
        .to_string();
    assert!(message.contains("615"), "no dice el límite: {message}");
    assert!(message.contains("872"), "no dice el límite: {message}");
}

#[test]
fn rejects_a_box_that_falls_short_of_a_displaced_origin() {
    let page = Page {
        number: 1,
        media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
        rotation: Rotation::None,
    };
    assert!(page.pades_rect(&rect([5, 10, 200, 100])).is_err());
}

#[test]
fn accepts_a_box_that_touches_the_edge() {
    let page = Page {
        number: 1,
        media_box: MediaBox::new(20.0, 30.0, 615.0, 872.0),
        rotation: Rotation::None,
    };
    assert!(page.pades_rect(&rect([20, 30, 615, 872])).is_ok());
}

#[test]
fn normalises_the_rotation_the_pdf_declares() {
    assert_eq!(Rotation::from_degrees(0), Some(Rotation::None));
    assert_eq!(Rotation::from_degrees(360), Some(Rotation::None));
    assert_eq!(Rotation::from_degrees(-90), Some(Rotation::ThreeQuarters));
    assert_eq!(Rotation::from_degrees(450), Some(Rotation::Quarter));
    assert_eq!(Rotation::from_degrees(45), None);
    assert_eq!(Rotation::ThreeQuarters.degrees(), 270);
}

#[test]
fn orders_the_corners_of_the_media_box() {
    let media_box = MediaBox::new(615.0, 872.0, 20.0, 30.0);
    assert_eq!(media_box.lower_x(), 20.0);
    assert_eq!(media_box.lower_y(), 30.0);
    assert_eq!(media_box.upper_x(), 615.0);
    assert_eq!(media_box.upper_y(), 872.0);
}

#[test]
fn normalises_a_drag_made_in_any_direction() {
    let page = Page {
        number: 1,
        media_box: MediaBox::new(0.0, 0.0, 595.0, 842.0),
        rotation: Rotation::Half,
    };
    let backwards = ViewerRect {
        x0: DRAG.x1,
        y0: DRAG.y1,
        x1: DRAG.x0,
        y1: DRAG.y0,
    };
    assert_eq!(
        page.to_user_space(&backwards, 1.0),
        page.to_user_space(&DRAG, 1.0)
    );
}

#[test]
fn resolves_all_to_the_very_same_pages_the_full_list_names() {
    assert_eq!(
        PageSet::All.resolve(3),
        PageSet::only([1, 2, 3]).expect("no esta vacio").resolve(3)
    );
}

#[test]
fn orders_and_deduplicates_the_pages_it_is_given() {
    let pages = PageSet::only([9, 3, 9, 7]).expect("no esta vacio");
    assert_eq!(pages.literal(), "3,7,9");
}

#[test]
fn refuses_to_build_a_set_without_pages() {
    assert_eq!(PageSet::only([]), None);
}

#[test]
fn refuses_a_page_the_document_does_not_have() {
    let refusal = PageSet::only_page(99)
        .validate(3)
        .expect_err("un documento de tres paginas no tiene la 99");
    assert_eq!(refusal.missing, vec![99]);
    assert_eq!(refusal.page_count, 3);
}

#[test]
fn names_every_page_the_document_does_not_have() {
    let refusal = PageSet::only([1, 4, 9])
        .expect("no esta vacio")
        .validate(3)
        .expect_err("le faltan dos");
    assert_eq!(refusal.missing, vec![4, 9]);
}

#[test]
fn refuses_the_page_zero_instead_of_guessing_which_convention_it_meant() {
    assert!(PageSet::only_page(0).validate(3).is_err());
}

#[test]
fn accepts_a_set_that_fits_in_the_document() {
    assert!(PageSet::only([1, 3])
        .expect("no esta vacio")
        .validate(3)
        .is_ok());
    assert!(PageSet::All.validate(3).is_ok());
}

#[test]
fn refuses_all_when_the_document_has_no_pages() {
    assert!(PageSet::All.validate(0).is_err());
}

#[test]
fn crosses_as_the_word_all_or_as_the_list_it_names() {
    assert_eq!(
        serde_json::to_value(PageSet::All).expect("deberia serializarse"),
        serde_json::json!("all")
    );
    assert_eq!(
        serde_json::to_value(PageSet::only([3, 1]).expect("no esta vacio"))
            .expect("deberia serializarse"),
        serde_json::json!({ "only": [1, 3] })
    );
}

#[test]
fn reads_back_what_it_writes() {
    for pages in [
        PageSet::All,
        PageSet::only_page(3),
        PageSet::only([1, 2, 3]).expect("no esta vacio"),
    ] {
        let written = serde_json::to_value(&pages).expect("deberia serializarse");
        assert_eq!(
            serde_json::from_value::<PageSet>(written).expect("deberia leerse"),
            pages
        );
    }
}

#[test]
fn refuses_a_page_set_written_in_a_grammar_it_does_not_speak() {
    for written in [
        serde_json::json!("append"),
        serde_json::json!("1-3"),
        serde_json::json!(0),
        serde_json::json!([1, 3]),
    ] {
        assert!(
            serde_json::from_value::<PageSet>(written.clone()).is_err(),
            "«{written}» no es un conjunto de paginas"
        );
    }
}

#[test]
fn refuses_an_empty_set_when_the_destination_is_validated() {
    let empty: PageSet =
        serde_json::from_value(serde_json::json!({ "only": [] })).expect("es json valido");
    assert!(empty.validate(3).is_err());
}
