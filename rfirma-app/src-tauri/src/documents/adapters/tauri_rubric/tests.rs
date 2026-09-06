use super::RubricChoiceView;
use crate::documents::adapters::rubric::{normalize, RubricError, Situation};

fn a_normalized_rubric() -> crate::documents::adapters::rubric::NormalizedRubric {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("el PNG de prueba deberia codificarse");
    normalize(&png).expect("el PNG de prueba deberia normalizarse")
}

#[test]
fn an_adopted_rubric_crosses_with_its_base64_and_size_and_no_failure() {
    let rubric = a_normalized_rubric();

    let choice = RubricChoiceView::adopted(&rubric);

    assert_eq!(
        choice.rubric.as_ref().map(|view| &view.base64),
        Some(&rubric.to_base64())
    );
    assert_eq!(
        choice.rubric.as_ref().map(|view| (view.width, view.height)),
        Some((4, 4))
    );
    assert!(choice.failure.is_none());
}

#[test]
fn a_refused_rubric_crosses_with_its_situation_and_no_image() {
    let error = RubricError::new(Situation::NotAnAcceptedImage, "no es PNG ni JPEG");

    let choice = RubricChoiceView::refused(&error);

    assert!(choice.rubric.is_none());
    assert_eq!(
        choice
            .failure
            .as_ref()
            .map(|failure| failure.situation.as_str()),
        Some("notAnAcceptedImage")
    );
    assert_eq!(
        choice
            .failure
            .as_ref()
            .map(|failure| failure.detail.as_str()),
        Some("no es PNG ni JPEG")
    );
}
