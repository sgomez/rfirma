use super::*;
use crate::commands::Failure;
use crate::fixtures::{a_certificate, listed_from};
use crate::signing::adapters::ffi::BridgeError;
use crate::site::domain::protocol::site_filter;
use std::cell::RefCell;

struct AnEngine {
    answer: Vec<usize>,
    asked: RefCell<Vec<(String, String)>>,
}

impl AnEngine {
    fn answering(answer: &[usize]) -> Self {
        Self {
            answer: answer.to_vec(),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl FilterEngine for AnEngine {
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError> {
        self.asked
            .borrow_mut()
            .push((filter_properties.to_owned(), certificates_b64.to_owned()));
        Ok(self.answer.clone())
    }
}

fn a_filter(expression: &str) -> SiteFilter {
    site_filter(&[("filters".to_owned(), expression.to_owned())]).expect("es aceptable")
}

#[test]
fn the_expression_and_the_listing_reach_the_engine_untouched() {
    let engine = AnEngine::answering(&[0]);
    let certificates = vec![a_certificate("UNO", &[0x01]), a_certificate("DOS", &[0x02])];

    keep_what_the_site_accepts(&engine, &a_filter("subject.contains:PEREZ"), certificates)
        .expect("el motor contesta");

    let asked = engine.asked.borrow();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].0, "filters=subject.contains:PEREZ\n");
    assert_eq!(asked[0].1, "AQ==;Ag==");
}

#[test]
fn only_the_certificates_the_engine_picked_come_back() {
    let engine = AnEngine::answering(&[1]);
    let certificates = vec![a_certificate("UNO", &[0x01]), a_certificate("DOS", &[0x02])];

    let kept = keep_what_the_site_accepts(&engine, &a_filter("ssl:true"), certificates)
        .expect("el motor contesta");

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].reference().label(), "DOS");
}

#[test]
fn a_site_that_excludes_them_all_gives_an_empty_listing_and_not_a_failure() {
    let engine = AnEngine::answering(&[]);
    let certificates = vec![a_certificate("UNO", &[0x01])];

    let kept = keep_what_the_site_accepts(&engine, &a_filter("dnie:true"), certificates)
        .expect("excluirlos todos no es un fallo");

    assert!(kept.is_empty());
}

#[test]
fn a_site_that_declares_nothing_still_reaches_the_engine() {
    let engine = AnEngine::answering(&[0]);
    let certificates = vec![a_certificate("UNO", &[0x01])];

    keep_what_the_site_accepts(&engine, &SiteFilter::default(), certificates)
        .expect("el motor contesta");

    assert_eq!(engine.asked.borrow()[0].0, "");
}

#[test]
fn a_certificate_the_site_no_longer_accepts_is_refused_before_the_pin() {
    let engine = AnEngine::answering(&[]);
    let certificates = [a_certificate("FIRMA", &[])];
    let (listed, handles) = listed_from(&certificates);

    let failure = usable_certificate_for_the_site(
        &engine,
        &a_filter("subject.contains:OTRO"),
        &certificates,
        &handles[0],
        &listed,
    )
    .expect_err("la sede lo excluye");

    let failure = Failure::from(failure);
    assert_eq!(failure.situation, "certificateNotFound");
    assert!(failure.detail.contains("FIRMA"), "{}", failure.detail);
}

#[test]
fn an_unusable_certificate_never_reaches_the_engine() {
    let engine = AnEngine::answering(&[0]);
    let certificates = [a_certificate("FIRMA", &[0x00, 0x01, 0x02])];
    let (listed, handles) = listed_from(&certificates);

    let failure = usable_certificate_for_the_site(
        &engine,
        &a_filter("ssl:true"),
        &certificates,
        &handles[0],
        &listed,
    )
    .expect_err("no es legible");

    let failure = Failure::from(failure);
    assert!(failure.detail.contains("Unreadable"), "{}", failure.detail);
    assert!(
        engine.asked.borrow().is_empty(),
        "un certificado que ya no sirve no tiene por que cruzar la frontera"
    );
}

#[test]
fn an_index_outside_the_listing_is_a_failure_and_not_a_silent_shorter_list() {
    let engine = AnEngine::answering(&[7]);
    let certificates = vec![a_certificate("UNO", &[0x01])];

    let failure = keep_what_the_site_accepts(&engine, &a_filter("ssl:true"), certificates)
        .expect_err("7 no es una fila");

    let failure = Failure::from(failure);
    assert!(failure.detail.contains('7'), "{}", failure.detail);
}

#[test]
fn the_rfirma_criteria_run_before_the_expression_of_the_site() {
    let source = include_str!("../filtering.rs");
    let body = source
        .split_once("pub fn listing_the_site_accepts")
        .expect("el caso de uso sigue aqui")
        .1;
    let ours = body
        .find("token.list_across")
        .expect("los criterios de rFirma");
    let theirs = body
        .find("keep_what_the_site_accepts")
        .expect("y despues los de la sede");

    assert!(
        ours < theirs,
        "la expresion de la sede se estaria aplicando antes que los criterios de rFirma"
    );
}
