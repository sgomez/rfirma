//! Pruebas del diálogo del área de la firma visible que pide `visibleSignature`.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::signing::domain::{PadesRect, PageSet, Placement};
use crate::site::application::errand::*;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::NegotiatedCredential;

const THE_SITE_AREA: &str = "signaturePositionOnPageLowerLeftX=100\n\
                             signaturePositionOnPageLowerLeftY=100\n\
                             signaturePositionOnPageUpperRightX=300\n\
                             signaturePositionOnPageUpperRightY=180\n\
                             signaturePage=1\n";

fn the_person_marks() -> Placement {
    Placement {
        rect: PadesRect {
            lower_left_x: 40,
            lower_left_y: 50,
            upper_right_x: 200,
            upper_right_y: 110,
        },
        pages: PageSet::only_page(2),
    }
}

/// Lo que se ve primero, lo que deja el diálogo del área, lo que se ve después y lo que sale.
struct Walked {
    first: Option<Moment>,
    after: Result<AfterTheArea, String>,
    then: Option<Moment>,
    wire: Option<String>,
    presign: Option<String>,
}

fn walked(expanded: &str, marked: Option<&Placement>) -> Walked {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(expanded);
    let scratch = home.path().join("errand");
    let mut desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    desk.neighbours.ours = ours;
    desk.neighbours.bridge = TheBridge::answering();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, mut wire) = the_wire();

    let step = attend(&desk, a_signature("sign", ""), handle, &live).expect("hay codec");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    let first = live.moment();
    let after = area_marked(&live, marked).map_err(|error| format!("{error:?}"));
    let then = live.moment();
    let consented = consent(&desk, &asking.certificates[0].id, &live);
    let presign = consented
        .is_ok()
        .then(|| desk.neighbours.bridge.extra_params_of_the_presign());
    Walked {
        first,
        after,
        then,
        wire: what_the_site_received(&mut wire),
        presign,
    }
}

fn is_the_area(moment: Option<&Moment>) -> bool {
    matches!(moment, Some(Moment::MarkingTheArea { .. }))
}

fn is_the_consent(moment: Option<&Moment>) -> bool {
    matches!(moment, Some(Moment::AskingToSign { .. }))
}

#[test]
fn a_wanted_visible_signature_opens_the_area_before_the_consent() {
    let walked = walked("visibleSignature=want\n", Some(&the_person_marks()));

    assert!(is_the_area(walked.first.as_ref()), "{:?}", walked.first);
    assert_eq!(walked.after, Ok(AfterTheArea::Consenting));
    assert!(is_the_consent(walked.then.as_ref()), "{:?}", walked.then);
}

#[test]
fn the_area_the_person_marks_reaches_the_bridge_in_place_of_the_request_area() {
    let expanded = format!("{THE_SITE_AREA}visibleSignature=want\n");
    let walked = walked(&expanded, Some(&the_person_marks()));

    let presign = walked.presign.expect("la firma sigue");
    for marked in [
        "signaturePages=2",
        "signaturePositionOnPageLowerLeftX=40",
        "signaturePositionOnPageLowerLeftY=50",
        "signaturePositionOnPageUpperRightX=200",
        "signaturePositionOnPageUpperRightY=110",
    ] {
        assert!(presign.contains(marked), "falta {marked}: {presign}");
    }
    assert!(
        !presign.contains("signaturePage=1"),
        "la pagina de la peticion no se queda: {presign}"
    );
}

#[test]
fn cancelling_a_wanted_area_without_one_in_the_request_answers_saf_43() {
    let walked = walked("visibleSignature=want\n", None);

    assert_eq!(walked.after, Ok(AfterTheArea::Answered));
    let wire = walked.wire.expect("la sede recibe su respuesta");
    assert!(wire.starts_with("SAF_43"), "{wire}");
    assert_eq!(walked.presign, None, "no se firma nada");
}

#[test]
fn cancelling_a_wanted_area_that_came_in_the_request_signs_there() {
    let expanded = format!("{THE_SITE_AREA}visibleSignature=want\n");
    let walked = walked(&expanded, None);

    assert_eq!(walked.after, Ok(AfterTheArea::Consenting));
    assert!(is_the_consent(walked.then.as_ref()), "{:?}", walked.then);
    let presign = walked.presign.expect("la firma sigue");
    assert!(
        presign.contains("signaturePositionOnPageLowerLeftX=100")
            && presign.contains("signaturePage=1"),
        "{presign}"
    );
}

#[test]
fn cancelling_an_optional_area_signs_invisible() {
    let walked = walked("visibleSignature=optional\n", None);

    assert_eq!(walked.after, Ok(AfterTheArea::Consenting));
    let presign = walked.presign.expect("la firma sigue");
    assert!(!presign.contains("signaturePosition"), "{presign}");
}

#[test]
fn a_request_without_the_flag_goes_straight_to_the_consent() {
    let walked = walked(THE_SITE_AREA, None);

    assert!(is_the_consent(walked.first.as_ref()), "{:?}", walked.first);
    assert!(walked.after.is_err(), "no hay area pendiente");
}

#[test]
fn nobody_consents_while_the_area_is_pending() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("visibleSignature=want\n");
    let scratch = home.path().join("errand");
    let mut desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    desk.neighbours.ours = ours;
    desk.neighbours.bridge = TheBridge::answering();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, _wire) = the_wire();
    let step = attend(&desk, a_signature("sign", ""), handle, &live).expect("hay codec");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };

    assert!(matches!(
        consent(&desk, &asking.certificates[0].id, &live),
        Err(ConsentError::NothingPending)
    ));
}
