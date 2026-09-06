use super::{begin_for_the_site, SiteSigning};
use crate::documents::application::opened::OpenedDocuments;
use crate::fixtures::{a_certificate, an_order};
use crate::identity::application::listed::ListedCertificates;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::adapters::orders::SigningOrder;
use crate::signing::application::session::{config_for, SigningSession};
use crate::signing::ports::FilterEngine;
use crate::site::domain::protocol::{SafCode, SiteFilter};
use std::collections::BTreeMap;

const SOURCE: &str = include_str!("../session.rs");

fn production_half() -> &'static str {
    SOURCE
}

#[test]
fn the_postsign_of_a_site_errand_writes_nothing_anywhere() {
    let site_postsign = production_half()
        .split_once("pub fn finish_for_the_site(")
        .expect("la postfirma de la sede sigue aqui")
        .1;

    for forbidden in [
        "documents::deliver",
        "recents::",
        "session.delivered",
        "remember_the_certificate",
    ] {
        assert!(
            !site_postsign.contains(forbidden),
            "la postfirma de la sede llama a «{forbidden}»: el documento que manda una sede no \
             deja rastro y rFirma no guarda ficheros por orden suya"
        );
    }
}

#[test]
fn the_presign_of_a_site_errand_checks_the_filter_again_before_the_pin() {
    let site_presign = production_half()
        .split_once("pub fn begin_for_the_site<")
        .expect("la prefirma de la sede sigue aqui")
        .1
        .split_once("\n/// ")
        .expect("y termina donde empieza la siguiente")
        .0;

    assert!(
        site_presign.contains("filtering::usable_certificate_for_the_site("),
        "el filtro de la sede se vuelve a comprobar antes de pedir el secreto"
    );
    assert!(
        !site_presign.contains("plan_signature("),
        "y no por el camino local, que no sabe nada de la sede"
    );
}

#[test]
fn a_signature_the_site_placed_carries_no_geometry_of_our_own() {
    let order = SigningOrder {
        placement: None,
        ..an_order()
    };

    let config = config_for(&order, &a_certificate("FIRMA", &[])).expect("no hay que colocar");

    assert_eq!(config.placement, None);
    for key in crate::signing::domain::Setting::Geometry.keys() {
        assert!(!config.extra_params().contains_key(*key), "'{key}' es suya");
    }
}

#[test]
fn a_site_signature_cannot_begin_on_a_document_that_is_not_open() {
    let order = SigningOrder {
        document: "00000000000000000000000000000000".to_owned(),
        ..an_order()
    };
    let engine = NoEngine;

    let failure = begin_for_the_site(
        &SiteSigning {
            engine: &engine,
            filter: &SiteFilter::default(),
            from_the_site: &BTreeMap::new(),
        },
        &order,
        &[],
        &ListedCertificates::new(),
        &OpenedDocuments::new(),
        &Isolate::start(),
        &SigningSession::default(),
    )
    .expect_err("ese documento no esta abierto");

    let (told, code) = crate::site::adapters::frontier::told(&failure);
    assert_eq!(told.situation, "documentUnreadable");
    assert_eq!(
        code,
        SafCode::CannotReadData,
        "y la sede recibe el codigo de lo que ha pasado, no uno para todo"
    );
}

struct NoEngine;

impl FilterEngine for NoEngine {
    fn select(
        &self,
        _properties: &str,
        _certificates: &str,
    ) -> Result<Vec<usize>, crate::signing::adapters::ffi::BridgeError> {
        unreachable!("no se llega a filtrar nada")
    }
}
