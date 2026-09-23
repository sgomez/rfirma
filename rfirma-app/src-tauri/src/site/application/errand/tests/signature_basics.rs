//! Pruebas de la firma de sede basica: recuadro, rubrica y paginas anadidas.

use std::cell::RefCell;

use super::support::*;
use super::support_requests::*;
use crate::crossing::Failure;
use crate::documents::application::documents::{self, OpenedDocuments};
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::adapters::failures::told_of_cycle;
use crate::signing::application::cycle::CycleError;
use crate::signing::application::session::CycleFailure;
use crate::signing::application::tests::a_memory;
use crate::signing::domain::bridge::BridgeError;
use crate::site::adapters::desk::signing_refusal_of;
use crate::site::application::errand::*;
use crate::site::application::session::SiteRefusal;
use crate::site::application::site::{attend_launch, Attendance};
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    NegotiatedCredential, SafCode, SignatureRound, SiteVisibleSignature, WireAnswer,
    THE_PORT_OF_THE_THIRD_PROTOCOL,
};
use crate::site::domain::signing::SiteSignature;
use base64::Engine as _;

/// Trámite completo de firma con el canal abierto, sobre la forma de arranque que se le pase.
#[expect(clippy::too_many_lines)]
fn the_whole_signature_errand_over(
    launch: &str,
    expected_port: u16,
    verb: &str,
    round: SignatureRound,
) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n");
    let scratch = home.path().join("errand");

    let attendance = attend_launch(launch, &a_codec_table(), &a_transport(&asked), &live);
    let Attendance::Serving { channel, .. } = &attendance else {
        panic!("la invocacion es buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        expected_port,
        "el tramite escucha en el puerto que declara el protocolo"
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = a_signature_arriving_over_the_channel(verb);
    let step = consent_to_sign(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            &scratch,
        ),
        &signature_requested(&url),
        ours.clone(),
        &live,
    );
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(consent.round, round, "la ronda es la que pidio la sede");
    assert_eq!(consent.certificates.len(), 1);
    assert_eq!(
        consent
            .from_the_site
            .get("policyIdentifier")
            .map(String::as_str),
        Some("urn:oid:2.16.724.1.3.1.1.2.1.9"),
        "la politica la expandio el motor del original"
    );
    assert_eq!(
        std::fs::read(
            documents::opened_document(&opened, &consent.document)
                .expect("el documento esta en la mano")
                .reading_path()
        )
        .expect("el fichero de paso existe"),
        A_PDF,
        "lo que se firma es lo que la sede mando"
    );
    assert!(
        live.current().is_some(),
        "consintiendo, el tramite sigue vivo"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el momento del consentimiento no escribe nada en el cable"
    );

    let scratch_file = live
        .scratch_path()
        .expect("el fichero de paso queda apuntado en el tramite");
    let reply = signature_handed_over(
        &live,
        &SiteSignature {
            signature: b"%PDF-1.7 firmado".to_vec(),
            signer_der: ours[0].der().to_vec(),
        },
    );
    assert!(
        matches!(reply, SiteOutcome::Signature { .. }),
        "la firma ha terminado: {reply:?}"
    );

    let encode = base64::engine::general_purpose::URL_SAFE;
    let line = what_the_site_received(&mut wire)
        .expect("la sede recibe la firma en el acto, por el asa del tramite");
    assert_eq!(
        line,
        format!(
            "{}|{}",
            encode.encode(ours[0].der()),
            encode.encode(b"%PDF-1.7 firmado")
        ),
        "el texto exacto del cable es el certificado y la firma, en Base64 URL-safe"
    );
    assert_eq!(
        line.split('|').count(),
        2,
        "el cliente publicado parte por `|` y no espera ningun tercer campo"
    );
    assert!(
        live.the_signature_consented().is_none(),
        "contestada la sede, no queda firma pendiente"
    );
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra al contestar"
    );
}
#[test]
fn a_signature_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand_over(
        &a_launch("54001,54002,54003"),
        54001,
        "sign",
        SignatureRound::First,
    );
}
#[test]
fn a_cosignature_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand_over(
        &a_launch("54001,54002,54003"),
        54001,
        "cosign",
        SignatureRound::Again,
    );
}
#[test]
fn a_signature_over_the_third_protocol_goes_all_the_way_from_the_launch_to_the_wire() {
    the_whole_signature_errand_over(
        &a_v3_launch(),
        THE_PORT_OF_THE_THIRD_PROTOCOL,
        "sign",
        SignatureRound::First,
    );
}
#[test]
fn a_signature_that_is_declined_ends_in_a_cancel_and_leaves_no_scratch_behind() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let attendance = attend_launch(
        &a_launch("54001,54002,54003"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    assert!(
        matches!(attendance, Attendance::Serving { .. }),
        "la invocacion es buena: {attendance:?}"
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = a_signature_arriving_over_the_channel("sign");
    let step = consent_to_sign(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            &scratch,
        ),
        &signature_requested(&url),
        ours,
        &live,
    );
    assert!(
        matches!(step, ErrandStep::AskingToSign(_)),
        "hay algo que consentir: {step:?}"
    );
    assert_eq!(what_the_site_received(&mut wire), None);
    let scratch_file = live
        .scratch_path()
        .expect("el fichero de paso queda apuntado en el tramite");

    let reply = declined(&live);

    assert!(matches!(reply, SiteOutcome::Cancelled), "{reply:?}");
    assert_eq!(
        what_the_site_received(&mut wire),
        Some("CANCEL".to_owned()),
        "cancelar sale al cable en el acto, sin esperar a que nadie cierre nada"
    );
    assert!(live.the_signature_consented().is_none());
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra tambien al cancelar"
    );

    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), None);
}
#[test]
fn a_signature_that_never_came_out_is_answered_with_the_code_of_a_failed_signature() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));

    let reply = the_signature_did_not_come_out(
        &live,
        SiteRefusal::Signing(signing_refusal_of(told_of_cycle(&CycleFailure::Cycle(
            CycleError::Bridge(BridgeError::Failed("la prefirma no ha salido".to_owned())),
        )))),
    );

    assert_eq!(
        what_the_site_received(&mut wire),
        Some("SAF_09: No se ha podido completar la firma electronica".to_owned()),
        "la sede recibe el codigo del catalogo, sin una palabra del detalle"
    );
    assert_eq!(
        reply
            .refusal()
            .map(|refusal| Failure::from(refusal).situation),
        Some("bridgeFailed".to_owned()),
        "y la ventana se queda con la situacion entera"
    );
    assert!(
        live.current().is_none(),
        "la firma que no sale cierra el tramite igual que la que sale"
    );
}
#[test]
fn a_broken_session_seal_is_answered_with_its_own_code() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));

    the_signature_did_not_come_out(
        &live,
        SiteRefusal::Signing(signing_refusal_of(told_of_cycle(&CycleFailure::Cycle(
            CycleError::Seal(crate::signing::domain::SealMismatch),
        )))),
    );

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::PostprocessingData).on_the_wire()),
        "el sello roto sale con SAF_42, que es el que el catalogo tiene para el"
    );
}
#[test]
fn the_document_a_site_sends_leaves_no_trace_at_all() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let step = consent_to_sign(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            &scratch,
        ),
        &signature_requested(&a_signature("cosign", "")),
        ours.clone(),
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(consent.round, SignatureRound::Again);
    assert!(
        !documents::opened_document(&opened, &consent.document)
            .expect("el documento esta en la mano")
            .is_remembered(),
        "el documento de la sede entra por la puerta que no recuerda"
    );
    assert!(
        crate::documents::application::recents::listed_rows(
            &memory,
            &crate::documents::adapters::files::RealFiles,
            &opened
        )
        .is_empty(),
        "no deja fila en Recientes"
    );
    assert_eq!(
        memory
            .state()
            .map(crate::signing::adapters::store::Loaded::into_value)
            .ok()
            .and_then(|state| state.visible_signature),
        None,
        "ni colocacion del recuadro"
    );

    let scratch_file = live.scratch_path().expect("hay fichero de paso");
    assert!(scratch_file.exists());
    declined(&live);
    assert!(
        !scratch_file.exists(),
        "el fichero de paso se borra al contestar"
    );
}
#[test]
fn a_box_the_site_placed_is_honoured_and_the_signature_goes_on() {
    let asked = a_consent_to_sign(
        "signaturePositionOnPageLowerLeftX=100\n\
         signaturePositionOnPageLowerLeftY=100\n\
         signaturePositionOnPageUpperRightX=300\n\
         signaturePositionOnPageUpperRightY=180\n\
         signaturePages=-1\n\
         visibleSignature=want\n",
    );

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("hay recuadro y hay certificado: {asked:?}");
    };
    assert_eq!(consent.visible, SiteVisibleSignature::PlacedByTheSite);
    assert_eq!(
        consent
            .from_the_site
            .get("signaturePages")
            .map(String::as_str),
        Some("-1"),
        "la pagina contada desde el final la resuelve el puente, no rFirma"
    );
}
#[test]
fn an_optional_box_the_site_never_placed_is_signed_invisible() {
    let asked = a_consent_to_sign("visibleSignature=optional\nvisibleAppearance=custom\n");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("se firma igual, sin recuadro: {asked:?}");
    };
    assert_eq!(consent.visible, SiteVisibleSignature::Declined);
}
#[test]
fn a_mandatory_box_the_site_never_placed_cancels_before_anyone_is_asked() {
    let asked = a_consent_to_sign("visibleSignature=want\n");

    let ErrandStep::Answering(reply) = asked else {
        panic!("no hay donde colocar el recuadro: {asked:?}");
    };
    assert!(
        on_the_wire(&reply).starts_with("SAF_43"),
        "lo que sale es el codigo de la firma visible: {}",
        on_the_wire(&reply)
    );
}
#[test]
fn a_page_appended_to_the_document_is_refused_before_anyone_is_asked() {
    let asked = a_consent_to_sign(
        "signaturePositionOnPageLowerLeftX=100\n\
         signaturePositionOnPageLowerLeftY=100\n\
         signaturePositionOnPageUpperRightX=300\n\
         signaturePositionOnPageUpperRightY=180\n\
         signaturePages=append\n",
    );

    let ErrandStep::Answering(reply) = asked else {
        panic!("no se anaden paginas: {asked:?}");
    };
    assert!(
        on_the_wire(&reply).starts_with("SAF_03"),
        "lo que sale es el rechazo del parametro: {}",
        on_the_wire(&reply)
    );
}
#[test]
fn an_appended_page_without_a_box_never_happens_and_the_errand_goes_on() {
    let asked = a_consent_to_sign("signaturePages=append\n");

    assert!(
        matches!(asked, ErrandStep::AskingToSign(_)),
        "sin esquinas no hay pagina que anadir: {asked:?}"
    );
}
