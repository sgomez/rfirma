//! Pruebas de eleccion de documento, guardado y carga por orden de la sede.

use std::sync::Arc;

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::session::SigningSession;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::application::session::SiteRefusal;
use crate::site::application::tests::{InMemoryBatchServices, InMemoryTokenSigning, NotAsked};
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelMessage, NegotiatedCredential, SafCode, SignatureRound, WireAnswer,
};
use crate::site::domain::signing::SiteSignature;
use base64::Engine as _;

#[test]
fn document_chosen_reads_the_scratch_path_lists_certificates_and_continues_the_errand() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step =
        attend(&desk, a_sign_and_save_without_dat(""), the_wire().0, &live).expect("hay codec");
    assert!(matches!(step, ErrandStep::Loading(_)));

    let chosen_path = home.path().join("elegido.pdf");
    std::fs::write(&chosen_path, A_PDF).expect("se escribe el fichero elegido");
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let moved = crate::site::application::errand::document_chosen(
        &desk,
        &[("elegido.pdf".to_owned(), chosen_path)],
        &live,
    );

    // Sin almacen, la mesa contesta en el acto: prueba que el documento se ha leido (si no,
    // hubiera salido SAF_25 antes de llegar a mirar certificados) y que se ha llegado a
    // mirarlos, no que se firme.
    assert!(
        matches!(moved, LoadCompletion::Delivered(_)),
        "ya ha contestado a la sede"
    );
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some(
            WireAnswer::refused(SafCode::CannotFindKeystore)
                .on_the_wire()
                .as_str()
        )
    );
}

#[test]
fn signing_and_saving_reaches_asking_to_sign_with_the_saving_hints_the_site_declared() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let url = a_sign_and_save("");

    let step = consent_to_sign_and_save(
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
        &sign_and_save_requested(&url),
        ours,
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(consent.round, SignatureRound::First);
    let saving = consent.saving.expect("signandsave trae pistas de guardado");
    assert_eq!(saving.filename, "firma.pdf");
    assert_eq!(saving.extensions, ["pdf", "p7s"]);
    assert_eq!(saving.description.as_deref(), Some("Documentos"));
    assert_eq!(saving.starting_folder.as_deref(), Some("/home/persona"));
}

#[test]
fn signing_and_saving_ends_in_the_saving_moment_with_the_der_to_answer_with() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    // La firma ya esta hecha (grada A no abre ningun ciclo real): lo que se prueba es la rama
    // de `finish` que compone el guardado, no el ciclo de firma en si.
    let desk = ErrandDesk {
        engine: &engine,
        policies: &policies,
        validation: &NotAsked,
        neighbours: ASignerThatSucceeds {
            neighbours: TheNeighbours {
                stores: Vec::new(),
                home: home.path(),
                listed: &listed,
                opened: &opened,
                memory: &memory,
                token: InMemoryTokenSigning::default(),
                signer: ATokenThatSigns::default(),
                ours: Vec::new(),
                bridge: TheBridge::default(),
                session: SigningSession::default(),
            },
            listed: ours.clone(),
            signature: SiteSignature {
                signature: b"%PDF-1.7 firmado".to_vec(),
                signer_der: ours[0].der().to_vec(),
            },
        },
        scratch_dir: scratch.clone(),
        scratch: Arc::new(crate::site::adapters::scratch::RealScratch),
        batch: Arc::new(InMemoryBatchServices::default()),
    };

    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, mut wire) = the_wire();
    let url = a_sign_and_save("");
    // Por `attend`, como llega en produccion: `dispatch` reparte a `attend_operation` y es
    // `remembered` quien rellena la memoria del tramite, no la prueba.
    let step = attend(&desk, url, handle, &live).expect("hay codec negociado");
    let ErrandStep::AskingToSign(_) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el consentimiento de firma no escribe nada en el cable"
    );

    let moved = finish(&desk, &live).expect("la postfirma no falla");
    let Some(ErrandStep::Saving(saving)) = moved else {
        panic!("signandsave pasa al guardado en vez de contestar: {moved:?}");
    };
    assert_eq!(saving.filename, Some("firma.pdf".to_owned()));
    assert_eq!(saving.extensions, ["pdf", "p7s"]);
    assert!(
        saving.signer_der.is_some(),
        "con que contestar cuando se guarde"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "signandsave no contesta hasta que la persona guarda"
    );
    assert_eq!(
        live.moment(),
        Some(Moment::Saving {
            filename: Some("firma.pdf".to_owned())
        })
    );
    assert!(
        live.current().is_some(),
        "el tramite sigue vivo, a la espera del guardado"
    );
}

#[test]
fn saved_with_a_signer_der_answers_the_same_line_as_a_plain_signature() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let der = vec![0xfb, 0xff, 0xbf];
    let destination = home.path().join("firma.pdf");

    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &destination,
        A_PDF,
        Some(&der),
        &live,
    );

    assert!(
        matches!(outcome, SiteOutcome::Signature { .. }),
        "{outcome:?}"
    );
    assert_eq!(std::fs::read(&destination).expect("se ha escrito"), A_PDF);
    let encode = base64::engine::general_purpose::URL_SAFE;
    assert_eq!(
        what_the_site_received(&mut wire).expect("la sede recibe la firma"),
        format!("{}|{}", encode.encode(&der), encode.encode(A_PDF)),
        "la misma linea que escribe signature_handed_over para el mismo par"
    );
}

/// Operación de guardado tal y como llega por el canal.
fn a_save(extra: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    let text = format!("afirma://save?op=save&idsession={CREDENTIAL}&dat={document}{extra}");
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Operación de carga tal y como llega por el canal.
fn a_load(extra: &str) -> AfirmaUrl {
    let text = format!("afirma://load?op=load&idsession={CREDENTIAL}{extra}");
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn saving_by_order_of_a_site_never_looks_at_certificates() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend_operation(&desk, &a_save(""), decoded(&a_save("")), &live);

    let ErrandStep::Saving(consent) = step else {
        panic!("«save» no mira certificados y espera al dialogo: {step:?}");
    };
    assert_eq!(consent.data, A_PDF);
}

#[test]
fn loading_by_order_of_a_site_never_looks_at_certificates() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend_operation(
        &desk,
        &a_load("&multiload=true"),
        decoded(&a_load("&multiload=true")),
        &live,
    );

    let ErrandStep::Loading(consent) = step else {
        panic!("«load» no mira certificados y espera al dialogo: {step:?}");
    };
    assert!(consent.multiple);
}

#[test]
fn the_save_moment_carries_only_the_name_the_site_proposed() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend(&desk, a_save("&filename=firma.pdf"), the_wire().0, &live)
        .expect("hay codec negociado");
    assert_eq!(
        step.moment(),
        Some(Moment::Saving {
            filename: Some("firma.pdf".to_owned())
        })
    );
    assert_eq!(
        live.the_saving_pending()
            .expect("hay guardado pendiente")
            .filename,
        Some("firma.pdf".to_owned())
    );
}

#[test]
fn the_loading_moment_never_carries_the_starting_folder() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();

    let step = attend(
        &desk,
        a_load("&filePath=/home/persona"),
        the_wire().0,
        &live,
    )
    .expect("hay codec negociado");
    assert_eq!(step.moment(), Some(Moment::Loading { multiple: false }));
    assert_eq!(
        live.the_loading_pending()
            .expect("hay carga pendiente")
            .starting_folder,
        Some("/home/persona".to_owned())
    );
}

#[test]
fn a_file_is_written_where_the_person_chose_and_the_site_gets_save_ok() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_save(""),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let destination = home.path().join("firma.pdf");
    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &destination,
        A_PDF,
        None,
        &live,
    );

    assert!(matches!(outcome, SiteOutcome::Saved));
    assert_eq!(std::fs::read(&destination).expect("se ha escrito"), A_PDF);
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some("SAVE_OK")
    );
}

#[test]
fn a_save_that_cannot_be_written_is_answered_with_saf_05() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_save(""),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let unwritable = home.path().join("no-existe").join("firma.pdf");
    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &unwritable,
        A_PDF,
        None,
        &live,
    );

    assert!(matches!(
        outcome,
        SiteOutcome::Refused(SiteRefusal::CannotSaveData(_))
    ));
    assert_eq!(
        on_the_wire(&outcome),
        WireAnswer::refused(SafCode::CannotSaveData).on_the_wire()
    );
    assert!(
        what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_05")),
        "sale el codigo del catalogo"
    );
}

#[test]
fn saved_writes_the_data_it_is_given_never_a_pending_consent_it_does_not_read() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let destination = home.path().join("firma.pdf");
    std::fs::write(&destination, "lo que ya habia").expect("se escribe el previo");

    let outcome = crate::site::application::errand::saved(
        &crate::site::adapters::scratch::RealScratch,
        &destination,
        A_PDF,
        None,
        &live,
    );

    assert!(matches!(outcome, SiteOutcome::Saved));
    assert_eq!(
        std::fs::read(&destination).expect("se ha escrito"),
        A_PDF,
        "escribe lo que se le pasa, nunca cero bytes por falta de consentimiento pendiente"
    );
}

#[test]
fn the_files_the_person_chose_go_out_named_and_apart_with_a_bar() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let first = home.path().join("uno.pdf");
    let second = home.path().join("dos.pdf");
    std::fs::write(&first, A_PDF).expect("se escribe la primera");
    std::fs::write(&second, A_PDF).expect("se escribe la segunda");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_load("&multiload=true"),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let chosen = vec![
        ("uno.pdf".to_owned(), first),
        ("dos.pdf".to_owned(), second),
    ];
    let outcome = crate::site::application::errand::loaded(
        &crate::site::adapters::scratch::RealScratch,
        &chosen,
        &live,
    );

    let SiteOutcome::Loaded(files) = &outcome else {
        panic!("se han cargado los ficheros: {outcome:?}");
    };
    assert_eq!(files.len(), 2);
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some("uno.pdf:JVBERi0xLjcK|dos.pdf:JVBERi0xLjcK")
    );
}

#[test]
fn a_file_that_disappeared_before_it_could_be_read_is_answered_with_saf_25() {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let live = a_live();
    let _ = attend(
        &a_desk_without_any_store(
            &AnEngine::answering(&[]),
            &APolicyEngine::answering(""),
            home.path(),
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &a_memory(home.path()),
            &home.path().join("errand"),
        ),
        a_load(""),
        the_wire().0,
        &live,
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let missing = home.path().join("no-existe.pdf");
    let outcome = crate::site::application::errand::loaded(
        &crate::site::adapters::scratch::RealScratch,
        &[("no-existe.pdf".to_owned(), missing)],
        &live,
    );

    assert!(matches!(
        outcome,
        SiteOutcome::Refused(SiteRefusal::CannotLoadData(_))
    ));
    assert!(
        what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_25")),
        "sale el codigo del catalogo"
    );
}
