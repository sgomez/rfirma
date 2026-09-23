//! Pruebas de algoritmo, resumen y formato de la firma de sede.


use crate::site::application::errand::*;
use crate::documents::application::documents::{self, OpenedDocuments};
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::signing::application::session::{self};
use crate::signing::application::tests::{
    a_memory, A_CADES_SIGNATURE, A_FACTURAE_SIGNATURE,
    A_XADES_SIGNATURE,
};
use crate::signing::domain::bridge::{
    Format, XadesVariant,
};
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    AfirmaUrl, AskedAlgorithm, ChannelMessage, NegotiatedCredential,
    SafCode,
    SiteVisibleSignature,
};
use base64::Engine as _;
use super::support::*;
use super::support_requests::*;

/// El reto de 64 bytes del banco de referencia, lo que una sede manda en `dat` para un CAdES.
const A_CHALLENGE: &[u8] = include_bytes!("../../../../../../../testdata/reference/challenge.bin");

/// Las claves de recuadro y rúbrica que una sede puede declarar y que solo lee un firmador PDF.
const THE_BOX_A_SITE_DECLARES: [&str; 7] = [
    "signaturePositionOnPageLowerLeftX",
    "signaturePositionOnPageLowerLeftY",
    "signaturePositionOnPageUpperRightX",
    "signaturePositionOnPageUpperRightY",
    "signaturePage",
    "visibleSignature",
    "signatureRubricImage",
];

/// Lo que expande el motor de políticas de una petición con política, modo y recuadro.
const EXPANDED_WITH_A_BOX: &str = "mode=explicit\n\
     policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n\
     signaturePositionOnPageLowerLeftX=100\n\
     signaturePositionOnPageLowerLeftY=100\n\
     signaturePositionOnPageUpperRightX=200\n\
     signaturePositionOnPageUpperRightY=200\n\
     signaturePage=1\n\
     visibleSignature=want\n\
     signatureRubricImage=cnVicmljYQ==\n";

#[test]
fn an_algorithm_the_token_does_not_offer_is_refused_without_asking_for_the_secret() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
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
    desk.neighbours.signer = ATokenThatSigns::offering(&[SignatureAlgorithm::Sha256Ecdsa]);

    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, _wire) = the_wire();

    let step = attend(
        &desk,
        a_signature_with_the_algorithm("SHA512withRSA"),
        handle,
        &live,
    )
    .expect("hay codec negociado");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("el algoritmo se casa con el token tras el consentimiento: {step:?}");
    };

    let chosen = asking.certificates[0].id.clone();
    let ConsentError::Refused(refusal) =
        consent(&desk, &chosen, &live).expect_err("el token no ofrece Sha512RsaPkcs")
    else {
        panic!("el trámite se rechaza, no se queda sin nada pendiente");
    };

    let (told, code) = crate::site::adapters::frontier::told(&refusal);
    assert_eq!(code, SafCode::SignatureFailed);
    assert_eq!(told.situation, "mechanismNotOffered");
    assert!(told.detail.contains("SHA512withECDSA"), "{}", told.detail);
    assert_eq!(
        desk.neighbours.signer.secrets_asked(),
        0,
        "el listado de mecanismos ya lo sabia: el PIN no se pide"
    );
}

#[test]
fn the_digest_the_site_asks_for_reaches_the_bridge_composed_with_the_key() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
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
        a_signature_with_the_algorithm("SHA384"),
        handle,
        &live,
    )
    .expect("hay codec negociado");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("una firma llega al consentimiento: {step:?}");
    };
    assert_eq!(asking.algorithm, AskedAlgorithm::Sha384);

    let chosen = asking.certificates[0].id.clone();
    consent(&desk, &chosen, &live).expect("el token ofrece Sha384RsaPkcs");

    assert_eq!(
        desk.neighbours.bridge.algorithm_of_the_presign(),
        "SHA384withECDSA",
        "el certificado de pruebas lleva clave EC: la sede pidio SHA384 y sale compuesto con ella"
    );
}

/// La misma firma de sede, con el `algorithm` que se le diga.
fn a_signature_with_the_algorithm(algorithm: &str) -> AfirmaUrl {
    let document = base64::engine::general_purpose::URL_SAFE.encode(A_PDF);
    let text = format!(
        "afirma://sign?op=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm={algorithm}&dat={document}"
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// El trámite entero de una firma de sede sobre un binario, del canal al cable, con el puente doblado.
fn the_whole_errand_asking_for(asked: &str, expected: Format) {
    the_whole_errand_asking_for_over(asked, A_CHALLENGE, expected, A_CADES_SIGNATURE);
}

/// Lo mismo, sobre el documento y con la firma de referencia que se le digan.
#[expect(clippy::too_many_lines)]
fn the_whole_errand_asking_for_over(
    asked: &str,
    document: &[u8],
    expected: Format,
    expected_signature: &[u8],
) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(EXPANDED_WITH_A_BOX);
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
    let (handle, mut wire) = the_wire();

    let step = attend(
        &desk,
        a_signature_asking_for(asked, document),
        handle,
        &live,
    )
    .expect("hay codec negociado");
    let ErrandStep::AskingToSign(asking) = step else {
        panic!("'{asked}' llega al consentimiento como PAdES: {step:?}");
    };
    assert_eq!(asking.format, expected, "format={asked}");
    assert_eq!(
        asking.visible,
        SiteVisibleSignature::Declined,
        "el recuadro que la sede coloco no se lee fuera de PAdES"
    );

    let chosen = asking.certificates[0].id.clone();
    let Consented::SigningWith(_) = consent(&desk, &chosen, &live).expect("el certificado vale")
    else {
        panic!("una firma se consiente firmando");
    };
    session::sign_on_token(&desk.neighbours.signer, &desk.neighbours.session, "1234")
        .expect("el token de pruebas firma el PRE");
    assert!(
        finish(&desk, &live).expect("la postfirma sale").is_none(),
        "un `sign` contesta en el acto, sin momento de guardado"
    );

    let encode = base64::engine::general_purpose::URL_SAFE;
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(format!(
            "{}|{}",
            encode.encode(ours[0].der()),
            encode.encode(expected_signature)
        )),
        "la misma linea que lleva un PDF firmado, con el CMS o el XML dentro"
    );
    assert!(live.current().is_none(), "contestada la sede, se acabo");

    let extra_params = desk.neighbours.bridge.extra_params_of_the_presign();
    assert_eq!(desk.neighbours.bridge.format_of_the_presign(), expected);
    assert!(
        extra_params.contains("mode=explicit"),
        "el modo llega al puente como propiedad: {extra_params}"
    );
    assert!(
        extra_params.contains("policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9"),
        "y la politica expandida tambien: {extra_params}"
    );
    for key in THE_BOX_A_SITE_DECLARES {
        assert!(
            !extra_params.contains(key),
            "'{key}' es del recuadro y solo lo lee un firmador PDF: {extra_params}"
        );
    }
    assert!(
        !extra_params.contains("layer2") && !extra_params.contains("signatureSubFilter"),
        "ni lo que rFirma anade de su cosecha para el recuadro: {extra_params}"
    );
}

#[test]
fn a_cades_signature_goes_all_the_way_from_the_operation_to_the_wire() {
    the_whole_errand_asking_for("CAdES", Format::Cades);
}

#[test]
fn a_cms_signature_goes_all_the_way_from_the_operation_to_the_wire() {
    the_whole_errand_asking_for("CMS/PKCS#7", Format::Cms);
}

#[test]
fn a_binary_under_format_auto_goes_out_as_a_cades_signature() {
    the_whole_errand_asking_for("auto", Format::Cades);
}

#[test]
fn a_xades_signature_goes_all_the_way_from_the_operation_to_the_wire() {
    for (format, expected) in [
        ("XAdES", XadesVariant::Enveloping),
        ("XAdES Detached", XadesVariant::Detached),
        ("XAdES Enveloped", XadesVariant::Enveloped),
        ("XAdES Enveloping", XadesVariant::Enveloping),
        ("XAdES-ASiC-S", XadesVariant::AsicS),
    ] {
        the_whole_errand_asking_for_over(
            format,
            AN_XML_CHALLENGE,
            Format::Xades(expected),
            A_XADES_SIGNATURE,
        );
    }
}

#[test]
fn an_xml_under_format_auto_goes_out_as_a_xades_signature() {
    the_whole_errand_asking_for_over(
        "auto",
        AN_XML_CHALLENGE,
        Format::Xades(XadesVariant::Enveloping),
        A_XADES_SIGNATURE,
    );
}

/// La factura sobre la que se pide la firma FacturaE del cable.
const AN_INVOICE_CHALLENGE: &[u8] = b"<Facturae><FileHeader/><Parties/><Invoices/></Facturae>";

#[test]
fn an_invoice_signature_goes_all_the_way_from_the_operation_to_the_wire() {
    for asked in ["FacturaE", "Factura-e"] {
        the_whole_errand_asking_for_over(
            asked,
            AN_INVOICE_CHALLENGE,
            Format::FacturaE,
            A_FACTURAE_SIGNATURE,
        );
    }
}

#[test]
fn an_invoice_under_format_auto_goes_out_as_a_facturae_signature() {
    the_whole_errand_asking_for_over(
        "auto",
        AN_INVOICE_CHALLENGE,
        Format::FacturaE,
        A_FACTURAE_SIGNATURE,
    );
}

#[test]
fn the_document_of_a_cades_errand_never_passes_through_as_a_pdf() {
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
        &signature_requested(&a_signature_asking_for("CAdES", A_CHALLENGE)),
        ours.clone(),
        &live,
    );

    let ErrandStep::AskingToSign(asking) = step else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    let path = documents::opened_document(&opened, &asking.document)
        .expect("el documento esta en la mano")
        .reading_path()
        .to_path_buf();
    assert_eq!(
        path.extension().and_then(std::ffi::OsStr::to_str),
        Some("bin"),
        "el reto de la sede no es un PDF y el fichero de paso no lo aparenta"
    );
    assert_eq!(
        std::fs::read(&path).expect("el fichero de paso existe"),
        A_CHALLENGE,
        "lo que se firma es lo que la sede mando"
    );
}

