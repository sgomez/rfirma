//! Pruebas de contrafirma, gzip y firma sin dat.


use crate::site::application::errand::*;
use crate::documents::application::documents::{self, OpenedDocuments};
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::{
    a_memory, A_CADES_SIGNATURE,
};
use crate::signing::domain::bridge::{
    Format, SignatureOperation, XadesVariant,
};
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelMessage, NegotiatedCredential, Parameter,
    SafCode, WireAnswer,
};
use base64::Engine as _;
use super::support::*;
use super::support_requests::*;

/// La contrafirma que pide una sede, con el formato y el `target` que se le digan.
fn a_countersignature_asking_for(format: &str, target: &str, document: &[u8]) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(document);
    let properties = base64::engine::general_purpose::URL_SAFE.encode(format!("target={target}\n"));
    let text = format!(
        "afirma://countersign?op=countersign&idsession={CREDENTIAL}&format={format}&\
         algorithm=SHA256withRSA&dat={document}&properties={properties}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn a_cades_countersignature_reaches_the_bridge_as_a_countersignature_over_its_target() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("target=tree\n");
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
    desk.neighbours.ours = ours.clone();
    desk.neighbours.bridge = TheBridge::answering();

    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, _wire) = the_wire();

    let step = attend(
        &desk,
        a_countersignature_asking_for("CAdES", "tree", A_CADES_SIGNATURE),
        handle,
        &live,
    )
    .expect("hay codec negociado");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("una contrafirma CAdES llega al consentimiento: {step:?}");
    };

    let chosen = asking.certificates[0].id.clone();
    let Consented::SigningWith(_) = consent(&desk, &chosen, &live).expect("el certificado vale")
    else {
        panic!("una firma se consiente firmando");
    };

    assert_eq!(
        desk.neighbours.bridge.operation_of_the_presign(),
        SignatureOperation::Countersign
    );
    assert_eq!(
        desk.neighbours.bridge.format_of_the_presign(),
        Format::Cades
    );
    assert!(
        policies
            .asked
            .borrow()
            .iter()
            .any(|asked| asked.contains("target=tree")),
        "el objetivo de la contrafirma cruza al puente sin traducir"
    );
}

#[test]
fn a_xades_countersignature_reaches_the_bridge_as_a_countersignature_over_its_target() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("target=leafs\n");
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
    desk.neighbours.ours = ours.clone();
    desk.neighbours.bridge = TheBridge::answering();

    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, _wire) = the_wire();

    let step = attend(
        &desk,
        a_countersignature_asking_for("XAdES", "leafs", AN_XML_CHALLENGE),
        handle,
        &live,
    )
    .expect("hay codec negociado");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("una contrafirma XAdES llega al consentimiento: {step:?}");
    };

    let chosen = asking.certificates[0].id.clone();
    let Consented::SigningWith(_) = consent(&desk, &chosen, &live).expect("el certificado vale")
    else {
        panic!("una firma se consiente firmando");
    };

    assert_eq!(
        desk.neighbours.bridge.operation_of_the_presign(),
        SignatureOperation::Countersign
    );
    assert_eq!(
        desk.neighbours.bridge.format_of_the_presign(),
        Format::Xades(XadesVariant::Enveloping)
    );
    assert!(
        policies
            .asked
            .borrow()
            .iter()
            .any(|asked| asked.contains("target=leafs")),
        "el objetivo de la contrafirma cruza al puente sin traducir"
    );
}

fn gzipped(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).expect("comprime en memoria");
    encoder.finish().expect("termina el gzip")
}

#[test]
#[expect(clippy::too_many_lines)]
fn gzip_true_decompresses_the_document_before_consent_in_the_five_operations() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0], &[0], &[0], &[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );

    let live = a_live();
    let gzipped_pdf = gzipped(A_PDF);
    let url_sign = a_signature_over(&gzipped_pdf, "sign", "&gzip=true");
    let step = consent_to_sign(&desk, &signature_requested(&url_sign), ours.clone(), &live);
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("esperaba AskingToSign en sign");
    };
    let path = documents::opened_document(&opened, &consent.document)
        .expect("el documento esta en la mano")
        .reading_path()
        .to_path_buf();
    assert_eq!(std::fs::read(&path).expect("lee el fichero"), A_PDF);

    let live = a_live();
    let url_cosign = a_signature_over(&gzipped_pdf, "cosign", "&gzip=true");
    let step = consent_to_sign(
        &desk,
        &signature_requested(&url_cosign),
        ours.clone(),
        &live,
    );
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("esperaba AskingToSign en cosign");
    };
    let path = documents::opened_document(&opened, &consent.document)
        .expect("el documento esta en la mano")
        .reading_path()
        .to_path_buf();
    assert_eq!(std::fs::read(&path).expect("lee el fichero"), A_PDF);

    let live = a_live();
    let gzipped_cades = gzipped(A_CADES_SIGNATURE);
    let countersign_doc = base64::engine::general_purpose::URL_SAFE.encode(&gzipped_cades);
    let props = base64::engine::general_purpose::URL_SAFE.encode("target=leafs\n");
    let text = format!(
        "afirma://countersign?op=countersign&idsession={CREDENTIAL}&format=CAdES&\
         algorithm=SHA256withRSA&gzip=true&dat={countersign_doc}&properties={props}"
    );
    let ChannelMessage::Operation { url: url_counter } = ChannelMessage::read(&text) else {
        panic!("esperaba operacion");
    };
    let step = consent_to_sign(
        &desk,
        &signature_requested(&url_counter),
        ours.clone(),
        &live,
    );
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("esperaba AskingToSign en countersign");
    };
    let path = documents::opened_document(&opened, &consent.document)
        .expect("el documento esta en la mano")
        .reading_path()
        .to_path_buf();
    assert_eq!(
        std::fs::read(&path).expect("lee el fichero"),
        A_CADES_SIGNATURE
    );

    let live = a_live();
    let encoded_pdf = base64::engine::general_purpose::URL_SAFE.encode(&gzipped_pdf);
    let text_save = format!(
        "afirma://signandsave?op=signandsave&cop=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&filename=firma.pdf&gzip=true&dat={encoded_pdf}"
    );
    let ChannelMessage::Operation { url: url_save } = ChannelMessage::read(&text_save) else {
        panic!("esperaba operacion");
    };
    let step = consent_to_sign_and_save(
        &desk,
        &sign_and_save_requested(&url_save),
        ours.clone(),
        &live,
    );
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("esperaba AskingToSign en signandsave");
    };
    let path = documents::opened_document(&opened, &consent.document)
        .expect("el documento esta en la mano")
        .reading_path()
        .to_path_buf();
    assert_eq!(std::fs::read(&path).expect("lee el fichero"), A_PDF);

    let live = a_live();
    let lote_json = format!(
        "{{\"algorithm\":\"SHA256\",\"format\":\"auto\",\"stoponerror\":false,\"singlesigns\":[\
         {{\"id\":\"001\",\"datareference\":\"{}\"}}]}}",
        in_the_batch(A_LOCAL_PDF)
    );
    let lote_compressed = gzipped(lote_json.as_bytes());
    let text_batch = format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&localBatchProcess=true&jsonbatch=true&gzip=true&dat={}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&lote_compressed)
    );
    let ChannelMessage::Operation { url: url_batch } = ChannelMessage::read(&text_batch) else {
        panic!("esperaba operacion");
    };
    let desk_batch = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );
    let step = attend_operation(&desk_batch, &url_batch, decoded(&url_batch), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("esperaba AskingToSignTheLocalBatch");
    };
    assert_eq!(asked.items.len(), 1);
    assert_eq!(asked.items[0].id, "001");
}

#[test]
fn gzip_true_with_invalid_dat_is_refused_with_saf03() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );

    let not_gzipped = b"not-a-valid-gzip-stream";
    let url = a_signature_over(not_gzipped, "sign", "&gzip=true");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::ShowingTheRefusal(refusal) = step else {
        panic!("un gzip que no se descomprime se rechaza: {step:?}");
    };
    assert_eq!(
        refusal.answer().on_the_wire(),
        WireAnswer::refused_because_of(SafCode::Params, Parameter::Data).on_the_wire()
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "hasta que se cierre la ventana"
    );
}

#[test]
fn gzip_with_value_other_than_true_leaves_dat_uncompressed() {
    let compressed = gzipped(A_PDF);
    let url = a_signature_over(&compressed, "sign", "&gzip=false");
    let request = signature_requested(&url);
    assert_eq!(request.document(), compressed.as_slice());
}

fn a_signature_without_dat(extra: &str) -> AfirmaUrl {
    let properties = base64::engine::general_purpose::URL_SAFE.encode(
        "filenameExts=pdf\nfilenameDescription=PDF\nfilenameCurrentDir=/home/persona\n".as_bytes(),
    );
    let text = format!(
        "afirma://sign?op=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&properties={properties}{extra}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn signing_without_dat_opens_the_loading_moment_with_the_sites_hints() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let live = a_live();
    let url = a_signature_without_dat("");

    let step = attend_operation(
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
        &url,
        decoded(&url),
        &live,
    );

    let ErrandStep::Loading(consent) = step else {
        panic!("sin 'dat' se abre el selector de un solo fichero: {step:?}");
    };
    assert!(!consent.multiple, "una firma pide un documento, no varios");
    assert_eq!(consent.extensions, ["pdf"]);
    assert_eq!(consent.description.as_deref(), Some("PDF"));
    assert_eq!(consent.starting_folder.as_deref(), Some("/home/persona"));
    assert!(
        consent.to_sign.is_some(),
        "lo elegido continua el tramite, no vuelve a la sede"
    );
}

#[test]
fn a_selector_declined_for_a_signature_without_dat_answers_cancel() {
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
    let (handle, mut wire) = the_wire();

    let step = attend(&desk, a_signature_without_dat(""), handle, &live).expect("hay codec");
    assert!(matches!(step, ErrandStep::Loading(_)));

    let outcome = crate::site::application::errand::decline(&live);

    assert!(matches!(outcome, SiteOutcome::Cancelled));
    assert_eq!(what_the_site_received(&mut wire).as_deref(), Some("CANCEL"));
}

#[test]
fn a_document_chosen_for_a_signature_without_dat_continues_the_errand() {
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

    let step = attend(&desk, a_signature_without_dat(""), the_wire().0, &live).expect("hay codec");
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

    assert!(
        matches!(moved, LoadCompletion::Delivered(_)),
        "sin almacen la mesa contesta en el acto, ya con el documento leido"
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

