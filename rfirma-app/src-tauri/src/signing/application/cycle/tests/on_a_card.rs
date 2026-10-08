//! El ciclo sobre el módulo PKCS#11 falso: cuántos `C_Login` recibe la tarjeta por ciclo y por lote (ADR-0047).

#![cfg(not(windows))]

use fake_pkcs11::FakeCard;

use super::{a_request, an_invisible_signature, ABridgeLikeTheRealOne};
use crate::identity::adapters::pkcs11::{list_certificates_across, RealToken};
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::store::Store;
use crate::signing::application::cycle::{presign, SigningRequest};
use crate::signing::domain::bridge::{Format, SignatureOperation};
use crate::signing::domain::{AdmissibleDocument, Waivers};
use crate::signing::ports::in_one_login;

fn signing_certificate_of(card: &FakeCard) -> TokenCertificate {
    list_certificates_across(&[Store::module(card.module())])
        .expect("la tarjeta falsa se lista")
        .into_iter()
        .find(|certificate| certificate.reference().label() == "CertFirmaDigital")
        .expect("la tarjeta falsa lleva el certificado de firma")
}

fn a_cades_document(content: &[u8]) -> AdmissibleDocument<'_> {
    AdmissibleDocument::check_for(Format::Cades, content, Waivers::NONE)
        .expect("CAdES no mira el /SubFilter")
}

#[test]
fn a_countersignature_on_a_card_logs_in_once_for_every_block() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let chosen = signing_certificate_of(&card);
    let bridge = ABridgeLikeTheRealOne::presigning(3);
    let config = an_invisible_signature();
    let chain = vec![chosen.der().to_vec()];
    let cycle = presign(
        &bridge,
        SigningRequest {
            operation: SignatureOperation::Countersign,
            ..a_request(
                Format::Cades,
                a_cades_document(b"una firma CAdES"),
                &chain,
                &config,
                chosen.reference(),
            )
        },
    )
    .expect("el puente contrafirma en CAdES");

    cycle
        .sign_on_token(&RealToken, &ProtectedSecret::from_str(FakeCard::PIN))
        .expect("la tarjeta firma cada bloque");

    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
}

#[test]
fn the_cycles_of_one_batch_on_a_card_share_one_login() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let chosen = signing_certificate_of(&card);
    let bridge = ABridgeLikeTheRealOne::default();
    let config = an_invisible_signature();
    let chain = vec![chosen.der().to_vec()];
    let secret = ProtectedSecret::from_str(FakeCard::PIN);

    in_one_login(&RealToken, chosen.reference(), || {
        for content in [b"uno".as_slice(), b"dos", b"tres"] {
            presign(
                &bridge,
                a_request(
                    Format::Cades,
                    a_cades_document(content),
                    &chain,
                    &config,
                    chosen.reference(),
                ),
            )
            .expect("el puente prefirma")
            .sign_on_token(&RealToken, &secret)
            .expect("la tarjeta firma");
        }
    });

    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
}
