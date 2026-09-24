//! Pruebas del nombre del documento elegido en disco, que vuelve a la sede en la respuesta.

use std::path::Path;
use std::sync::Arc;

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::adapters::memory::Memory;
use crate::signing::application::session::SigningSession;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::application::tests::{InMemoryBatchServices, InMemoryTokenSigning, NotAsked};
use crate::site::domain::protocol::AfirmaUrl;
use crate::site::domain::signing::SiteSignature;
use base64::Engine as _;

const THE_CHOSEN: &str = "documento.txt";

#[expect(clippy::too_many_arguments)]
fn a_desk_that_signs<'a>(
    engine: &'a AnEngine,
    policies: &'a APolicyEngine,
    home: &'a Path,
    listed: &'a ListedCertificates,
    opened: &'a OpenedDocuments,
    memory: &'a Memory,
    ours: &[TokenCertificate],
    scratch: &Path,
) -> ErrandDesk<'a, AnEngine, APolicyEngine, ASignerThatSucceeds<'a>> {
    ErrandDesk {
        engine,
        policies,
        validation: &NotAsked,
        neighbours: ASignerThatSucceeds {
            neighbours: TheNeighbours {
                stores: Vec::new(),
                home,
                listed,
                opened,
                memory,
                token: InMemoryTokenSigning::default(),
                signer: ATokenThatSigns::default(),
                ours: Vec::new(),
                bridge: TheBridge::default(),
                session: SigningSession::default(),
            },
            listed: ours.to_vec(),
            signature: SiteSignature {
                signature: b"%PDF-1.7 firmado".to_vec(),
                signer_der: ours[0].der().to_vec(),
            },
        },
        scratch_dir: scratch.to_path_buf(),
        scratch: Arc::new(crate::site::adapters::scratch::RealScratch),
        batch: Arc::new(InMemoryBatchServices::default()),
        triphase: Arc::new(crate::site::application::tests::InMemoryTriphaseServer::default()),
    }
}

fn the_extra_data(line: &str) -> Option<String> {
    let third = line.split('|').nth(2)?;
    let json = base64::engine::general_purpose::URL_SAFE
        .decode(third)
        .expect("el tercer componente va en Base64 URL-safe");
    Some(String::from_utf8(json).expect("el JSON va en UTF-8"))
}

/// Firma lo que pide `url`, eligiendo en disco `THE_CHOSEN` cuando la petición no trae `dat`.
fn what_the_site_receives_after_signing(url: AfirmaUrl) -> String {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_that_signs(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &ours,
        &scratch,
    );
    let live = a_live();
    let (handle, mut wire) = the_wire();

    let mut step = attend(&desk, url, handle, &live).expect("hay codec");
    if let ErrandStep::Loading(_) = step {
        let chosen_path = home.path().join(THE_CHOSEN);
        std::fs::write(&chosen_path, A_PDF).expect("se escribe el fichero elegido");
        let LoadCompletion::Continues(continued) =
            document_chosen(&desk, &[(THE_CHOSEN.to_owned(), chosen_path)], &live)
        else {
            panic!("el documento elegido se lee y el tramite sigue");
        };
        step = continued;
    }
    assert!(matches!(step, ErrandStep::AskingToSign(_)), "{step:?}");

    if let Some(ErrandStep::Saving(saving)) = finish(&desk, &live).expect("la postfirma no falla") {
        let _ = saved(
            &crate::site::adapters::scratch::RealScratch,
            &home.path().join("firma.pdf"),
            b"%PDF-1.7 firmado",
            saving.signer_der.as_deref(),
            &live,
        );
    }
    what_the_site_received(&mut wire).expect("la sede recibe la firma")
}

#[test]
fn a_signature_over_a_document_chosen_on_disk_returns_its_name_in_a_third_component() {
    let line = what_the_site_receives_after_signing(a_signature_without_dat(""));

    assert_eq!(
        the_extra_data(&line).as_deref(),
        Some(r#"{"filename": "documento.txt"}"#)
    );
}

#[test]
fn a_sign_and_save_over_a_document_chosen_on_disk_returns_its_name_in_a_third_component() {
    let line = what_the_site_receives_after_signing(a_sign_and_save_without_dat(""));

    assert_eq!(
        the_extra_data(&line).as_deref(),
        Some(r#"{"filename": "documento.txt"}"#)
    );
}

#[test]
fn a_signature_over_the_document_the_site_sent_stays_a_pair() {
    let line = what_the_site_receives_after_signing(a_signature("sign", ""));

    assert_eq!(line.split('|').count(), 2, "{line}");
}
