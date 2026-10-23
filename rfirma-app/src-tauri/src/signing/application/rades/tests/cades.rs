use ::rades::ObjectIdentifier;
use openssl::sha::sha256;

use super::{
    a_chain_with, an_ec_key, an_rsa_key, b64, contains, openssl_verifies, token_signs, Chain,
    Rades, AN_INSTANT, A_DOCUMENT,
};
use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignRequest, SignatureOperation,
};
use crate::signing::ports::Bridge;

const ALGORITHM: &str = "SHA256withRSA";
const CONTENT_HINTS: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.4");
const MIME_TYPE: ObjectIdentifier = ObjectIdentifier::new_unwrap("0.4.0.1733.2.1");

fn cades_with(chain: &Chain, document: &[u8], algorithm: &str, extra_params: &str) -> Vec<u8> {
    let rades = Rades::at(|| AN_INSTANT);
    let presigned = rades
        .presign(PreSignRequest {
            format: Format::Cades,
            operation: SignatureOperation::Sign,
            document_b64: &b64(document),
            algorithm,
            certificate_chain_b64: &chain.b64(),
            extra_params,
        })
        .expect("la prefirma sale");
    let signatures = presigned
        .signed_one_by_one(|pre| Ok::<_, ()>(token_signs(&chain.signer.key, algorithm, pre)))
        .expect("el token firma");
    let sealed = presigned
        .sealed_with(signatures, presigned.stamp())
        .expect("el sello vuelve intacto");
    rades
        .postsign(PostSignRequest {
            format: Format::Cades,
            document_b64: &b64(document),
            certificate_chain_b64: &chain.b64(),
            sealed: &sealed,
        })
        .expect("la postfirma sale")
}

fn a_cades(chain: &Chain, extra_params: &str) -> Vec<u8> {
    cades_with(chain, A_DOCUMENT, ALGORITHM, extra_params)
}

fn refused_with(operation: SignatureOperation, algorithm: &str, extra_params: &str) -> BridgeError {
    let chain = a_chain_with(an_rsa_key());
    Rades::at(|| AN_INSTANT)
        .presign(PreSignRequest {
            format: Format::Cades,
            operation,
            document_b64: &b64(A_DOCUMENT),
            algorithm,
            certificate_chain_b64: &chain.b64(),
            extra_params,
        })
        .expect_err("rades lo rechaza")
}

fn signed_attribute(signature: &[u8], oid: ObjectIdentifier) -> Option<Vec<Vec<u8>>> {
    let decoded = ::rades::cades::decode(signature).expect("rades lee su CAdES");
    decoded.signers()[0]
        .signed_attribute(oid)
        .map(|attribute| attribute.values().to_vec())
}

#[test]
fn rades_signs_cades_but_not_its_asic_s_container() {
    assert!(Rades::signs(Format::Cades));
    assert!(!Rades::signs(Format::CadesAsicS));
}

#[test]
fn an_implicit_cades_is_the_default_and_openssl_verifies_it() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(&chain, "");

    assert_eq!(openssl_verifies(&signature, None), A_DOCUMENT);
    let decoded = ::rades::cades::decode(&signature).expect("rades lee su CAdES");
    assert!(decoded.signers()[0].signing_certificate().is_some());
}

#[test]
fn an_explicit_cades_leaves_the_document_out_and_verifies_against_it() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(&chain, "mode=explicit\n");

    let decoded = ::rades::cades::decode(&signature).expect("rades lee su CAdES");
    assert_eq!(decoded.content(), None);
    openssl_verifies(&signature, Some(A_DOCUMENT));
}

#[test]
fn a_precalculated_hash_is_signed_as_a_detached_signature_over_the_data_it_digests() {
    let chain = a_chain_with(an_rsa_key());

    let signature = cades_with(
        &chain,
        &sha256(A_DOCUMENT),
        ALGORITHM,
        "precalculatedHashAlgorithm=SHA-256\n",
    );

    let decoded = ::rades::cades::decode(&signature).expect("rades lee su CAdES");
    assert_eq!(decoded.content(), None);
    openssl_verifies(&signature, Some(A_DOCUMENT));
}

#[test]
fn a_precalculated_hash_algorithm_rades_does_not_know_is_refused_with_its_name() {
    let refused = refused_with(
        SignatureOperation::Sign,
        ALGORITHM,
        "precalculatedHashAlgorithm=SHA-1\n",
    );

    assert!(
        matches!(&refused, BridgeError::NotSupported(what) if what.contains("SHA-1")),
        "{refused}"
    );
}

#[test]
fn the_whole_chain_goes_in_unless_the_site_asks_for_the_signer_only() {
    let chain = a_chain_with(an_rsa_key());

    let by_default = a_cades(&chain, "");
    let signer_only = a_cades(&chain, "includeOnlySignningCertificate=true\n");

    assert!(contains(&by_default, &chain.authority.der()));
    assert!(contains(&signer_only, &chain.signer.der()));
    assert!(!contains(&signer_only, &chain.authority.der()));
}

#[test]
fn rsa_and_ecdsa_cades_verify_in_openssl() {
    let rsa = a_chain_with(an_rsa_key());
    let ec = a_chain_with(an_ec_key());

    for (chain, algorithm) in [
        (&rsa, "SHA512withRSA"),
        (&rsa, "SHA256withRSAandMGF1"),
        (&ec, "SHA384withECDSA"),
    ] {
        let signature = cades_with(chain, A_DOCUMENT, algorithm, "");

        assert_eq!(
            openssl_verifies(&signature, None),
            A_DOCUMENT,
            "{algorithm}"
        );
    }
}

#[test]
fn content_hints_carry_the_oid_and_the_description_the_site_gives() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(
        &chain,
        "contentTypeOid=1.2.840.113549.1.7.1\ncontentDescription=Un contrato\n",
    );

    let values = signed_attribute(&signature, CONTENT_HINTS).expect("content-hints");
    assert!(contains(&values[0], b"Un contrato"));
    assert!(contains(
        &values[0],
        &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01]
    ));
}

#[test]
fn content_hints_and_mime_type_are_left_out_unless_the_site_gives_the_value() {
    let chain = a_chain_with(an_rsa_key());

    for declared in [
        "",
        "contentDescription=Sin tipo\n",
        "includeMimeTypeAttribute=true\n",
        "mimeType=application/pdf\n",
        "includeMimeTypeAttribute=false\nmimeType=application/pdf\n",
    ] {
        let signature = a_cades(&chain, declared);

        assert_eq!(
            signed_attribute(&signature, CONTENT_HINTS),
            None,
            "{declared}"
        );
        assert_eq!(signed_attribute(&signature, MIME_TYPE), None, "{declared}");
    }
}

#[test]
fn the_mime_type_attribute_carries_the_type_when_the_site_asks_for_it() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(
        &chain,
        "includeMimeTypeAttribute=true\nmimeType=application/pdf\n",
    );

    let values = signed_attribute(&signature, MIME_TYPE).expect("mime-type");
    assert!(contains(&values[0], b"application/pdf"));
}

#[test]
fn what_rades_does_not_do_yet_is_refused_before_the_token_with_the_key_that_asked_for_it() {
    for declared in [
        "tsaURL=https\\://tsa.example\n",
        "commitmentTypeIndication1Identifier=1.2.3\n",
        "signerClaimedRoles=Director\n",
        "signatureProductionCity=Sevilla\n",
        "signatureProductionCountry=ES\n",
    ] {
        let refused = refused_with(SignatureOperation::Sign, ALGORITHM, declared);

        let key = declared.split('=').next().expect("una clave");
        assert!(
            matches!(&refused, BridgeError::NotSupported(_)) && refused.to_string().contains(key),
            "{declared}: {refused}"
        );
    }
}

#[test]
fn a_value_that_forces_what_rades_does_not_do_is_refused_with_the_key_and_the_value() {
    for declared in [
        "signingCertificateV2=false",
        "includeSigningTimeAttribute=false",
    ] {
        let refused = refused_with(
            SignatureOperation::Sign,
            ALGORITHM,
            &format!("{declared}\n"),
        );

        assert!(
            refused.to_string().contains(declared),
            "{declared}: {refused}"
        );
    }
}

#[test]
fn the_values_rades_already_does_are_admitted() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(
        &chain,
        "signingCertificateV2=true\nincludeSigningTimeAttribute=true\n",
    );

    assert_eq!(openssl_verifies(&signature, None), A_DOCUMENT);
}

#[test]
fn the_protocol_keys_that_do_not_change_a_cades_are_admitted_without_effect() {
    let chain = a_chain_with(an_rsa_key());

    let plain = a_cades(&chain, "");
    let with_protocol_keys = a_cades(
        &chain,
        "format=CAdES\nheadless=true\nmandatoryCertSelection=true\n",
    );

    assert_eq!(plain, with_protocol_keys);
}

#[test]
fn a_cades_is_neither_cosigned_nor_countersigned_by_rades() {
    for operation in [SignatureOperation::Cosign, SignatureOperation::Countersign] {
        let refused = refused_with(operation, ALGORITHM, "");

        assert!(
            matches!(
                refused,
                BridgeError::UnsupportedOperation(Format::Cades, asked) if asked == operation
            ),
            "{operation:?}: {refused}"
        );
    }
}

#[test]
fn sha1_is_refused_in_a_cades_with_the_algorithm_name() {
    let refused = refused_with(SignatureOperation::Sign, "SHA1withRSA", "");

    assert!(matches!(&refused, BridgeError::NotSupported(what) if what == "SHA1withRSA"));
}

const A_POLICY: &str = "policyIdentifier=urn:oid:1.2.3.4.5\n\
    policyIdentifierHash=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8\\=\n\
    policyIdentifierHashAlgorithm=http\\://www.w3.org/2001/04/xmlenc#sha256\n\
    policyQualifier=https\\://policy.example/doc.pdf\n";

fn policy_of(signature: &[u8]) -> ::rades::SignaturePolicy {
    let decoded = ::rades::cades::decode(signature).expect("rades lee su CAdES");
    decoded.signers()[0]
        .signature_policy()
        .cloned()
        .expect("la firma lleva política")
}

#[test]
fn the_four_policy_keys_are_the_signature_policy_identifier_of_the_signature() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(&chain, A_POLICY);

    let expected = ::rades::ExplicitPolicy::new(
        ObjectIdentifier::new_unwrap("1.2.3.4.5"),
        ::rades::PolicyHashAlgorithm::Digest(::rades::DigestAlgorithm::Sha256),
        (0..32).collect(),
    )
    .qualifier(::rades::PolicyQualifier::Spuri(
        "https://policy.example/doc.pdf".to_owned(),
    ));
    assert_eq!(policy_of(&signature), expected.into());
    assert_eq!(openssl_verifies(&signature, None), A_DOCUMENT);
}

#[test]
fn the_qualifier_is_optional() {
    let chain = a_chain_with(an_rsa_key());
    let without_qualifier: String = A_POLICY
        .lines()
        .filter(|line| !line.trim_start().starts_with("policyQualifier"))
        .map(|line| format!("{line}\n"))
        .collect();

    let signature = a_cades(&chain, &without_qualifier);

    let ::rades::SignaturePolicy::Explicit(policy) = policy_of(&signature) else {
        panic!("la política es explícita");
    };
    assert!(policy.qualifiers().is_empty());
}

#[test]
fn the_age_policy_the_site_expands_is_the_age_v1_9_in_an_implicit_and_an_explicit_cades() {
    let chain = a_chain_with(an_rsa_key());
    let expanded = "policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9\n\
        policyIdentifierHash=G7roucf600+f03r/o0bAOQ6WAs0\\=\n\
        policyIdentifierHashAlgorithm=http\\://www.w3.org/2000/09/xmldsig#sha1\n\
        policyQualifier=https\\://sede.administracion.gob.es/politica_de_firma_anexo_1.pdf\n";

    for mode in ["implicit", "explicit"] {
        let signature = a_cades(&chain, &format!("{expanded}mode={mode}\n"));

        assert_eq!(
            policy_of(&signature),
            ::rades::SignaturePolicy::age_v1_9(),
            "{mode}"
        );
    }
}

#[test]
fn an_unexpanded_age_policy_gives_the_age_v1_9_too() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cades(&chain, "expPolicy=FirmaAGE\n");

    assert_eq!(policy_of(&signature), ::rades::SignaturePolicy::age_v1_9());
}

#[test]
fn a_policy_that_is_not_the_age_one_is_refused_with_its_name() {
    let refused = refused_with(SignatureOperation::Sign, ALGORITHM, "expPolicy=Otra\n");

    assert!(
        matches!(&refused, BridgeError::NotSupported(what) if what.contains("expPolicy=Otra")),
        "{refused}"
    );
}

#[test]
fn an_incomplete_policy_is_refused_before_the_token_and_says_what_is_missing() {
    for (declared, missing) in [
        ("policyIdentifier=urn:oid:1.2.3\n", "policyIdentifierHash"),
        (
            "policyIdentifier=urn:oid:1.2.3\npolicyIdentifierHash=AAAA\n",
            "policyIdentifierHashAlgorithm",
        ),
        ("policyQualifier=https\\://a.example\n", "policyIdentifier"),
    ] {
        let refused = refused_with(SignatureOperation::Sign, ALGORITHM, declared);

        assert!(
            matches!(&refused, BridgeError::NotSupported(what) if what.contains(missing)),
            "{declared}: {refused}"
        );
    }
}

#[test]
fn a_policy_hash_that_does_not_fit_its_algorithm_is_refused_before_the_token() {
    let refused = refused_with(
        SignatureOperation::Sign,
        ALGORITHM,
        "policyIdentifier=urn:oid:1.2.3\npolicyIdentifierHash=AAAA\n\
         policyIdentifierHashAlgorithm=http\\://www.w3.org/2001/04/xmlenc#sha256\n",
    );

    assert!(matches!(refused, BridgeError::Failed(_)), "{refused}");
}

#[test]
fn a_policy_identifier_that_is_not_an_oid_is_refused_with_its_value() {
    let refused = refused_with(
        SignatureOperation::Sign,
        ALGORITHM,
        "policyIdentifier=https\\://x.example/p\npolicyIdentifierHash=AAAA\n\
         policyIdentifierHashAlgorithm=SHA1\n",
    );

    assert!(
        matches!(&refused, BridgeError::NotSupported(what) if what.contains("x.example")),
        "{refused}"
    );
}
