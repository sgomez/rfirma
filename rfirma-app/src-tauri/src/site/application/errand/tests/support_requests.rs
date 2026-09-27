//! Constructores de peticiones y consentimientos compartidos por las pruebas del tramite.

use std::path::Path;
use std::sync::Arc;

use super::support::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::adapters::memory::Memory;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::application::tests::read_operation;
use crate::site::application::tests::InMemoryBatchServices;
use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage, SignRequest, SiteOperation};
use crate::site::domain::signing::SiteSignature;
use base64::Engine as _;

/// Un PDF mínimo, que es lo que la sede manda dentro de `dat`.
pub(crate) const A_PDF: &[u8] = b"%PDF-1.7\n";

pub(crate) const A_PDF_SIGNED_BY_SOMETHING_ELSE: &[u8] =
    b"%PDF-1.7\n9 0 obj\n<< /Type /Sig /SubFilter /adbe.pkcs7.whatever >>\nendobj\n";

/// La petición de firma ya leída, que es lo que recibe el caso de uso.
pub(crate) fn signature_requested(url: &AfirmaUrl) -> SignRequest {
    let SiteOperation::Sign(request) =
        read_operation(url).expect("es una operacion que se atiende")
    else {
        panic!("es una firma");
    };
    request
}

/// La operación de firma tal y como llega por el canal.
pub(crate) fn a_signature(verb: &str, extra: &str) -> AfirmaUrl {
    a_signature_over(A_PDF, verb, extra)
}

/// La misma operación, sobre el documento que se le diga.
pub(crate) fn a_signature_over(pdf: &[u8], verb: &str, extra: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(pdf);
    let text = format!(
        "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&dat={document}{extra}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// La misma operación, con el formato y el documento que se le digan.
pub(crate) fn a_signature_asking_for(format: &str, document: &[u8]) -> AfirmaUrl {
    let encoded = base64::engine::general_purpose::URL_SAFE.encode(document);
    let text = format!(
        "afirma://sign?op=sign&idsession={CREDENTIAL}&format={format}&\
         algorithm=SHA256withRSA&dat={encoded}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// La misma operación, con `mode=explicit` declarado entre las propiedades.
pub(crate) fn an_explicit_mode_signature(format: &str, document: &[u8]) -> AfirmaUrl {
    let encoded = base64::engine::general_purpose::URL_SAFE.encode(document);
    let properties = base64::engine::general_purpose::URL_SAFE.encode("mode=explicit\n");
    let text = format!(
        "afirma://sign?op=sign&idsession={CREDENTIAL}&format={format}&\
         algorithm=SHA256withRSA&dat={encoded}&properties={properties}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Operación de firma entrando por el canal.
pub(crate) fn a_signature_arriving_over_the_channel(verb: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    arriving_over_the_channel(&format!(
        "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&dat={document}"
    ))
}

/// La petición de `signandsave` ya leída, que es lo que recibe el caso de uso.
pub(crate) fn sign_and_save_requested(
    url: &AfirmaUrl,
) -> crate::site::domain::protocol::SignAndSaveRequest {
    let SiteOperation::SignAndSave(request) =
        read_operation(url).expect("es una operacion que se atiende")
    else {
        panic!("es un firmar y guardar");
    };
    request
}

/// La operación de `signandsave`, con las tres pistas `filenameSave*` declaradas.
pub(crate) fn a_sign_and_save(extra: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    let properties = base64::engine::general_purpose::URL_SAFE.encode(
        "filenameSaveExts=pdf,p7s\nfilenameSaveDescription=Documentos\n\
         filenameSaveCurrentDir=/home/persona\n"
            .as_bytes(),
    );
    let text = format!(
        "afirma://signandsave?op=signandsave&cop=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&filename=firma.pdf&properties={properties}&dat={document}{extra}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Consentimiento de una firma con política expandida.
pub(crate) fn a_consent_to_sign(expanded: &str) -> ErrandStep {
    a_consent_to_sign_over(A_PDF, expanded)
}

/// El mismo consentimiento, sobre el documento que se le diga.
pub(crate) fn a_consent_to_sign_over(pdf: &[u8], expanded: &str) -> ErrandStep {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering(expanded);
    let scratch = home.path().join("errand");

    consent_to_sign(
        &a_desk(
            &engine,
            &policies,
            &a_neighbourhood(home.path(), &listed, &opened, &memory),
            &scratch,
        ),
        &signature_requested(&a_signature_over(pdf, "sign", "")),
        ours,
        &live,
    )
}

/// La operación de `sign` sin `dat`, con las pistas del selector.
pub(crate) fn a_signature_without_dat(extra: &str) -> AfirmaUrl {
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

/// La operación de `signandsave` sin `dat`, con las pistas del selector y las de guardado.
pub(crate) fn a_sign_and_save_without_dat(extra: &str) -> AfirmaUrl {
    let properties = base64::engine::general_purpose::URL_SAFE.encode(
        "filenameExts=pdf\nfilenameDescription=PDF\nfilenameCurrentDir=/home/persona\n\
         filenameSaveExts=pdf,p7s\nfilenameSaveDescription=Documentos\n\
         filenameSaveCurrentDir=/home/persona\n"
            .as_bytes(),
    );
    let text = format!(
        "afirma://signandsave?op=signandsave&cop=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&filename=firma.pdf&properties={properties}{extra}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Un vecino que lista los certificados dados y da la firma de sede por buena.
pub(crate) fn a_signer_for_the_batch<'a>(
    home: &'a Path,
    listed: &'a ListedCertificates,
    memory: &'a Memory,
    ours: &[TokenCertificate],
) -> ASignerThatSucceeds<'a> {
    ASignerThatSucceeds {
        neighbours: a_neighbourhood(home, listed, opened_for_nobody(), memory),
        listed: ours.to_vec(),
        signature: SiteSignature {
            signature: Vec::new(),
            signer_der: Vec::new(),
        },
    }
}

/// Una mesa sobre el vecino dado que habla con los servlets del lote dados.
pub(crate) fn a_desk_for_the_batch<'a>(
    engine: &'a AnEngine,
    policies: &'a APolicyEngine,
    neighbours: &'a dyn Neighbours,
    home: &Path,
    services: Arc<InMemoryBatchServices>,
) -> ErrandDesk<'a, AnEngine, APolicyEngine> {
    ErrandDesk {
        batch: services,
        ..a_desk(engine, policies, neighbours, &home.join("errand"))
    }
}

pub(crate) const A_LOCAL_PDF: &[u8] = b"%PDF-1.4\n";
pub(crate) const A_LOCAL_BINARY: &[u8] = b"\x00\x01\x02\x03";
pub(crate) const A_LOCAL_XML: &[u8] = b"<?xml version=\"1.0\"?><a/>";

/// El Base64 con el que la sede mete un documento dentro del JSON del lote.
pub(crate) fn in_the_batch(document: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(document)
}

/// Un lote local de tres elementos con `format=auto`: un PDF, un binario que se cofirma y un XML.
pub(crate) fn a_local_batch(extra: &str) -> AfirmaUrl {
    let lote = format!(
        "{{\"algorithm\":\"SHA256\",\"format\":\"auto\",\"stoponerror\":false,\"singlesigns\":[\
         {{\"id\":\"001\",\"datareference\":\"{}\"}},\
         {{\"id\":\"002\",\"datareference\":\"{}\",\"suboperation\":\"cosign\"}},\
         {{\"id\":\"003\",\"datareference\":\"{}\"}}]}}",
        in_the_batch(A_LOCAL_PDF),
        in_the_batch(A_LOCAL_BINARY),
        in_the_batch(A_LOCAL_XML),
    );
    let text = format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&jsonbatch=true&\
         localBatchProcess=true&dat={}{extra}",
        base64::engine::general_purpose::URL_SAFE.encode(&lote)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Un vecindario que firma de verdad por el ciclo de sede, con el puente doblado atendiendo.
pub(crate) fn neighbours_for_the_local_batch<'a>(
    home: &'a Path,
    listed: &'a ListedCertificates,
    opened: &'a OpenedDocuments,
    memory: &'a Memory,
    ours: &[TokenCertificate],
) -> TheNeighbours<'a> {
    let mut neighbours = a_neighbourhood(home, listed, opened, memory);
    neighbours.ours = ours.to_vec();
    neighbours.bridge = TheBridge::answering();
    neighbours
}

/// El resultado del lote que la sede acaba de recibir, ya descodificado.
pub(crate) fn the_batch_result(wire: &mut tokio::sync::oneshot::Receiver<String>) -> String {
    let answered = what_the_site_received(wire).expect("la sede recibe el resultado del lote");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(answered)
        .expect("el resultado del lote viaja en Base64");
    String::from_utf8(decoded).expect("el resultado del lote es JSON")
}

/// El documento sobre el que se piden las firmas XAdES del cable: cualquier XML vale.
pub(crate) const AN_XML_CHALLENGE: &[u8] = b"<?xml version=\"1.0\"?><documento/>";
