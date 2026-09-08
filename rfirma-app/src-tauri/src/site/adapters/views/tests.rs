use super::{Moment, SignatureRound, SignatureRoundView};
use super::{
    NoCertificateView, NoChannelView, RefusalSituation, RefusalSituationView, SiteErrandView,
};
use crate::site::domain::batch_error::Situation as BatchSituation;

#[test]
fn the_dead_ends_cross_named_and_never_written_out() {
    assert_eq!(
        serde_json::to_value(SiteErrandView::no_channel(NoChannelView::ChannelNotOpened))
            .expect("el callejon cruza"),
        serde_json::json!({
            "origin": null,
            "stage": { "kind": "noChannel", "reason": "channelNotOpened" },
        })
    );
    assert_eq!(
        serde_json::to_value(SiteErrandView::no_channel(NoChannelView::LocalCaMissing))
            .expect("el callejon cruza"),
        serde_json::json!({
            "origin": null,
            "stage": { "kind": "noChannel", "reason": "localCaMissing" },
        })
    );
    assert_eq!(
        serde_json::to_value(SiteErrandView::without_certificates(
            NoCertificateView::None,
            0
        ))
        .expect("el callejon cruza"),
        serde_json::json!({
            "origin": null,
            "stage": { "kind": "noCertificate", "reason": "none", "owned": 0 },
        })
    );
}

#[test]
fn saving_and_loading_cross_with_the_name_and_never_a_path() {
    assert_eq!(
        serde_json::to_value(SiteErrandView::from(&Moment::Saving {
            filename: Some("firma.pdf".to_owned())
        }))
        .expect("cruza"),
        serde_json::json!({
            "origin": null,
            "stage": { "kind": "saving", "filename": "firma.pdf" },
        })
    );
    assert_eq!(
        serde_json::to_value(SiteErrandView::from(&Moment::Loading { multiple: true }))
            .expect("cruza"),
        serde_json::json!({
            "origin": null,
            "stage": { "kind": "loading", "multiple": true },
        })
    );
}

#[test]
fn a_refusal_without_a_channel_crosses_with_its_situation_and_its_detail() {
    let refusal = crate::site::domain::protocol::Refusal::new(
        crate::site::domain::protocol::SafCode::UnsupportedProcedure,
        "la sede declara la version de protocolo 3",
    )
    .because(RefusalSituation::UnsupportedProtocolVersion);

    assert_eq!(
        serde_json::to_value(SiteErrandView::refused(&refusal)).expect("el rechazo cruza"),
        serde_json::json!({
            "origin": null,
            "stage": {
                "kind": "outcome",
                "outcome": {
                    "kind": "refused",
                    "situation": "unsupportedProtocolVersion",
                    "detail": "la sede declara la version de protocolo 3",
                },
            },
        })
    );
}

#[test]
fn the_round_crosses_named_as_the_site_asked_for_it() {
    let view = SiteErrandView::from(&Moment::AskingToSign {
        document: "doc-1".to_owned(),
        round: SignatureRound::Again,
        certificates: Vec::new(),
        unregistered_signatures: false,
    });

    assert_eq!(
        serde_json::to_value(&view).expect("serializa")["stage"]["round"],
        serde_json::to_value(SignatureRoundView::Cosign).expect("serializa")
    );
}

#[test]
fn each_batch_situation_crosses_as_its_own_view() {
    for (situation, expected) in [
        (
            BatchSituation::PresignerUnreachable,
            RefusalSituationView::BatchPresignerUnreachable,
        ),
        (
            BatchSituation::PostsignerUnreachable,
            RefusalSituationView::BatchPostsignerUnreachable,
        ),
        (
            BatchSituation::InvalidPresignResponse,
            RefusalSituationView::BatchInvalidPresignResponse,
        ),
        (
            BatchSituation::InvalidPostsignResponse,
            RefusalSituationView::BatchInvalidPostsignResponse,
        ),
    ] {
        assert_eq!(RefusalSituationView::from(situation), expected);
    }
}

#[test]
fn the_batch_consent_crosses_with_how_many_signs_it_has_and_who_is_already_chosen() {
    assert_eq!(
        serde_json::to_value(SiteErrandView::from(&Moment::AskingToSignTheBatch {
            signs: 3,
            certificates: Vec::new(),
            already_chosen: Some("una-asa".to_owned()),
        }))
        .expect("el consentimiento del lote cruza"),
        serde_json::json!({
            "origin": null,
            "stage": {
                "kind": "askingToSignTheBatch",
                "signs": 3,
                "certificates": [],
                "alreadyChosen": "una-asa",
            },
        })
    );
    assert_eq!(
        serde_json::to_value(SiteErrandView::from(&Moment::AskingToSignTheBatch {
            signs: 1,
            certificates: Vec::new(),
            already_chosen: None,
        }))
        .expect("el consentimiento del lote cruza"),
        serde_json::json!({
            "origin": null,
            "stage": {
                "kind": "askingToSignTheBatch",
                "signs": 1,
                "certificates": [],
                "alreadyChosen": null,
            },
        })
    );
}
