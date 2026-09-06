use super::*;
use crate::signing::domain::SignatureConfig;
use std::cell::RefCell;

struct AnEngine {
    asked: RefCell<Vec<(String, String)>>,
    answer: Result<String, ()>,
}

impl AnEngine {
    fn answering(block: &str) -> Self {
        Self {
            asked: RefCell::new(Vec::new()),
            answer: Ok(block.to_owned()),
        }
    }

    fn that_refuses_the_policy() -> Self {
        Self {
            asked: RefCell::new(Vec::new()),
            answer: Err(()),
        }
    }
}

impl PolicyEngine for AnEngine {
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError> {
        self.asked
            .borrow_mut()
            .push((extra_params.to_owned(), format.to_owned()));
        self.answer.clone().map_err(|()| {
            BridgeError::IncompatiblePolicy("politica que no se puede aplicar".to_owned())
        })
    }
}

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn declared(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn the_declared_block_reaches_the_engine_as_a_pades_expansion() {
    let engine = AnEngine::answering("policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n");

    let expanded =
        expanded_for_the_site(&engine, &declared(&[("expPolicy", "FirmaAGE")])).expect("ok");

    assert_eq!(
        engine.asked.borrow().as_slice(),
        [("expPolicy=FirmaAGE\n".to_owned(), "pades".to_owned())]
    );
    assert_eq!(
        expanded,
        params(&[("policyIdentifier", "urn:oid:2.16.724.1.3.1.1.2.1.9")])
    );
}

#[test]
fn a_policy_that_cannot_be_applied_is_not_signed_around() {
    let engine = AnEngine::that_refuses_the_policy();

    let refused = expanded_for_the_site(&engine, &declared(&[("expPolicy", "Inventada")]));

    assert!(refused.is_err());
}

#[test]
fn what_rfirma_decides_is_written_over_what_the_site_declared() {
    let merged = merged_with(
        params(&[
            ("layer2Text", "lo que la sede quisiera"),
            ("policyIdentifier", "urn:oid:1"),
        ]),
        params(&[("layer2Text", "Firmado por: Ada Lovelace Byron")]),
    );

    assert_eq!(
        merged,
        params(&[
            ("layer2Text", "Firmado por: Ada Lovelace Byron"),
            ("policyIdentifier", "urn:oid:1"),
        ])
    );
}

#[test]
fn the_box_the_site_placed_reaches_the_bridge_exactly_as_it_came() {
    let hers = params(&[
        ("signaturePositionOnPageLowerLeftX", "100"),
        ("signaturePositionOnPageLowerLeftY", "100"),
        ("signaturePositionOnPageUpperRightX", "300"),
        ("signaturePositionOnPageUpperRightY", "180"),
        ("signaturePages", "1-3,-3--1"),
    ]);
    let ours = SignatureConfig {
        placement: None,
        layer2_text: "Firmado por: Ada Lovelace Byron".to_owned(),
        rubric_image: None,
        sign_reason: None,
        allow_unregistered_signatures: false,
    };

    let merged = merged_with(hers.clone(), ours.extra_params());

    for (key, value) in &hers {
        assert_eq!(
            merged.get(key),
            Some(value),
            "'{key}' lo ajusto la sede contra AutoFirma y cruza sin tocar"
        );
    }
}
