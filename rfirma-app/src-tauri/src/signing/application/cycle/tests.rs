use super::{presign, SigningRequest, ALGORITHM, NOTHING_FROM_A_SITE};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::identity::application::tests::a_certificate;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignBlock, PreSignRequest, PreSignature,
    SignatureOperation, XadesVariant,
};
use crate::signing::domain::{AdmissibleDocument, SessionSeal, SignatureConfig};
use crate::signing::ports::{Bridge, Signer};

const BORDER: &str = include_str!("../../adapters/ffi.rs");

fn identifiers(source: &str) -> BTreeSet<&str> {
    source
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect()
}

fn entry_points() -> BTreeSet<String> {
    BORDER
        .match_indices("autofirma_")
        .map(|(start, _)| {
            BORDER[start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

#[test]
fn java_has_no_entry_point_for_the_signing_phase() {
    let expected: BTreeSet<String> = [
        "autofirma_cades_postsign",
        "autofirma_cades_presign",
        "autofirma_expand_extra_params",
        "autofirma_filter_certificates",
        "autofirma_free_string",
        "autofirma_pades_postsign",
        "autofirma_pades_presign",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();

    assert_eq!(
        entry_points(),
        expected,
        "la frontera con Java ha cambiado de puntos de entrada: si uno de \
         ellos firma, la clave privada ha entrado en el isolate (ADR-0001)"
    );
}

#[test]
fn the_pin_has_no_way_across_the_border() {
    let words = identifiers(BORDER);
    for forbidden in ["pin", "private_key", "AuthPin", "cryptoki"] {
        assert!(
            !words.contains(forbidden),
            "«{forbidden}» aparece en la frontera FFI: la fase 2 se estaría \
             delegando a Java, contra el ADR-0001"
        );
    }
}

/// Un puente que resuelve lo mismo que el de `adapters/ffi.rs`, y apunta lo que le llega.
struct ABridgeLikeTheRealOne {
    calls: RefCell<Vec<String>>,
    blocks: usize,
}

impl Default for ABridgeLikeTheRealOne {
    fn default() -> Self {
        Self {
            calls: RefCell::default(),
            blocks: 1,
        }
    }
}

impl ABridgeLikeTheRealOne {
    fn presigning(blocks: usize) -> Self {
        Self {
            calls: RefCell::default(),
            blocks,
        }
    }
}

impl Bridge for ABridgeLikeTheRealOne {
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError> {
        request.format.bridged()?;
        self.calls.borrow_mut().push(format!(
            "presign(operation={}, document={}, algorithm={}, chain={}, extraParams={})",
            request.operation.name(),
            request.document_b64,
            request.algorithm,
            request.certificate_chain_b64,
            request.extra_params
        ));
        Ok(PreSignature {
            session: "<xml/>".to_owned(),
            blocks: (0..self.blocks)
                .map(|index| PreSignBlock {
                    id: format!("00{index}"),
                    pre: format!("12{index}").into_bytes(),
                })
                .collect(),
            stamp: SessionSeal::from_bridge("el sello de la prefirma"),
        })
    }

    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError> {
        request.format.bridged()?;
        let pkcs1: Vec<&str> = request
            .sealed
            .signed()
            .iter()
            .map(|block| block.pkcs1_b64())
            .collect();
        self.calls.borrow_mut().push(format!(
            "postsign(document={}, chain={}, session={}, pkcs1={}, stamp={})",
            request.document_b64,
            request.certificate_chain_b64,
            request.sealed.session(),
            pkcs1.join(","),
            request.sealed.stamp().as_bridge_payload()
        ));
        Ok(b"%PDF-1.7 firmado".to_vec())
    }
}

/// Un token que cuenta cuántas veces se le pide el secreto y cuántas firma.
#[derive(Default)]
struct ATokenThatCounts {
    secrets: Cell<usize>,
    signatures: Cell<usize>,
}

impl Signer for ATokenThatCounts {
    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        self.secrets.set(self.secrets.get() + 1);
        Ok(StoreSecret::NotNeeded)
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        _pin: &str,
        _algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        self.signatures.set(self.signatures.get() + 1);
        Ok(data.to_vec())
    }
}

fn a_request<'a>(
    format: Format,
    document: AdmissibleDocument<'a>,
    chain: &'a [Vec<u8>],
    config: &'a SignatureConfig,
    certificate: &'a crate::identity::domain::certificate::CertificateRef,
) -> SigningRequest<'a> {
    SigningRequest {
        format,
        operation: SignatureOperation::Sign,
        document,
        chain,
        config,
        from_the_site: &NOTHING_FROM_A_SITE,
        certificate,
    }
}

fn an_invisible_signature() -> SignatureConfig {
    SignatureConfig {
        placement: None,
        layer2_text: String::new(),
        rubric_image: None,
        sign_reason: None,
        allow_unregistered_signatures: false,
    }
}

#[test]
fn a_cades_cycle_reaches_the_bridge_instead_of_stopping_at_the_format() {
    let bridge = ABridgeLikeTheRealOne::default();
    let chosen = a_certificate("FIRMA", b"der");
    let config = an_invisible_signature();
    let document = AdmissibleDocument::check_for(Format::Cades, b"no soy un PDF")
        .expect("CAdES no mira el /SubFilter");

    let cycle = presign(
        &bridge,
        a_request(
            Format::Cades,
            document,
            std::slice::from_ref(&b"der".to_vec()),
            &config,
            chosen.reference(),
        ),
    )
    .expect("el puente ya atiende CAdES");
    let seal = cycle.seal_in_transit();
    cycle
        .postsign(&bridge, cycle.invented_signatures(), &seal)
        .expect("el sello volvio intacto");

    assert_eq!(
        bridge.calls.borrow().len(),
        2,
        "la prefirma y la postfirma cruzaron"
    );
}

#[test]
fn every_format_the_bridge_does_not_resolve_is_refused_by_its_name() {
    let bridge = ABridgeLikeTheRealOne::default();
    let chosen = a_certificate("FIRMA", b"der");
    let config = an_invisible_signature();

    for format in [
        Format::CadesAsicS,
        Format::Xades(XadesVariant::Enveloped),
        Format::FacturaE,
    ] {
        let document =
            AdmissibleDocument::check_for(format, b"lo que sea").expect("no se mira el PDF");

        let failed = presign(
            &bridge,
            a_request(
                format,
                document,
                std::slice::from_ref(&b"der".to_vec()),
                &config,
                chosen.reference(),
            ),
        )
        .expect_err("el puente no atiende ese formato");

        assert!(failed.to_string().contains(format.name()));
    }
}

#[test]
fn a_pades_cycle_sends_the_bridge_the_very_same_call_as_before_the_format() {
    let bridge = ABridgeLikeTheRealOne::default();
    let chosen = a_certificate("FIRMA", b"der");
    let config = an_invisible_signature();
    let document = AdmissibleDocument::check_for(Format::Pades, b"%PDF-1.7")
        .expect("es un PDF que se puede firmar");

    let cycle = presign(
        &bridge,
        a_request(
            Format::Pades,
            document,
            std::slice::from_ref(&b"der".to_vec()),
            &config,
            chosen.reference(),
        ),
    )
    .expect("PAdES cruza");
    let seal = cycle.seal_in_transit();
    cycle
        .postsign(&bridge, cycle.invented_signatures(), &seal)
        .expect("el sello volvio intacto");

    assert_eq!(
        *bridge.calls.borrow(),
        [
            "presign(operation=sign, document=JVBERi0xLjc=, algorithm=SHA256withRSA, \
             chain=ZGVy, extraParams=layer2FontSize=0\nlayer2Text=\n\
             signatureSubFilter=ETSI.CAdES.detached\n)"
                .to_owned(),
            format!(
                "postsign(document=JVBERi0xLjc=, chain=ZGVy, session=<xml/>, pkcs1={}, \
                 stamp=el sello de la prefirma)",
                crate::signing::domain::TokenSignature::invented().to_pkcs1_base64()
            ),
        ]
    );
}

#[test]
fn the_algorithm_matches_the_pkcs11_mechanism() {
    assert_eq!(ALGORITHM.name(), "SHA256withRSA");
    assert_eq!(
        ALGORITHM.mechanism().mechanism_type(),
        cryptoki::mechanism::MechanismType::SHA256_RSA_PKCS
    );
}

#[test]
fn a_countersignature_asks_the_secret_once_and_signs_every_block_it_got() {
    let bridge = ABridgeLikeTheRealOne::presigning(3);
    let token = ATokenThatCounts::default();
    let chosen = a_certificate("FIRMA", b"der");
    let config = an_invisible_signature();
    let document = AdmissibleDocument::check_for(Format::Cades, b"una firma CAdES")
        .expect("CAdES no mira el /SubFilter");

    let cycle = presign(
        &bridge,
        SigningRequest {
            format: Format::Cades,
            operation: SignatureOperation::Countersign,
            document,
            chain: std::slice::from_ref(&b"der".to_vec()),
            config: &config,
            from_the_site: &NOTHING_FROM_A_SITE,
            certificate: chosen.reference(),
        },
    )
    .expect("el puente contrafirma en CAdES");
    let secret = token.secret_of(cycle.certificate()).expect("no hace falta");
    let signatures = cycle
        .sign_on_token(&token, "1234")
        .expect("el token firma cada bloque");
    let seal = cycle.seal_in_transit();
    cycle
        .postsign(&bridge, signatures, &seal)
        .expect("el sello volvio intacto");

    assert_eq!(token.signatures.get(), 3, "una firma por bloque");
    assert_eq!(token.secrets.get(), 1, "el secreto se pide una sola vez");
    assert!(matches!(secret, StoreSecret::NotNeeded));
    assert!(
        bridge.calls.borrow()[0].contains("operation=countersign"),
        "{:?}",
        bridge.calls.borrow()
    );
}

#[test]
fn a_cosignature_names_its_operation_at_the_border() {
    let bridge = ABridgeLikeTheRealOne::default();
    let chosen = a_certificate("FIRMA", b"der");
    let config = an_invisible_signature();
    let document = AdmissibleDocument::check_for(Format::Cades, b"una firma CAdES")
        .expect("CAdES no mira el /SubFilter");

    presign(
        &bridge,
        SigningRequest {
            format: Format::Cades,
            operation: SignatureOperation::Cosign,
            document,
            chain: std::slice::from_ref(&b"der".to_vec()),
            config: &config,
            from_the_site: &NOTHING_FROM_A_SITE,
            certificate: chosen.reference(),
        },
    )
    .expect("el puente cofirma en CAdES");

    assert!(
        bridge.calls.borrow()[0].contains("operation=cosign"),
        "{:?}",
        bridge.calls.borrow()
    );
}
