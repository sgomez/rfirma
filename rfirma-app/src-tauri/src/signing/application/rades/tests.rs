use base64::Engine;
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::cms::{CMSOptions, CmsContentInfo};
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer};
use openssl::x509::{X509Name, X509};

use super::Rades;
use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignRequest, PreSignature, SignatureOperation,
};
use crate::signing::ports::Bridge;

const A_DOCUMENT: &[u8] = b"un documento de la sede\x00\x01";
const AN_INSTANT: u64 = 1_700_000_000;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

struct Holder {
    key: PKey<Private>,
    certificate: X509,
}

impl Holder {
    fn der(&self) -> Vec<u8> {
        self.certificate.to_der().expect("DER del certificado")
    }
}

fn an_rsa_key() -> PKey<Private> {
    PKey::from_rsa(Rsa::generate(2048).expect("clave RSA")).expect("PKey RSA")
}

fn an_ec_key() -> PKey<Private> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).expect("P-256");
    PKey::from_ec_key(EcKey::generate(&group).expect("clave EC")).expect("PKey EC")
}

fn issued(common_name: &str, serial: u32, key: PKey<Private>, issuer: Option<&Holder>) -> Holder {
    let mut name = X509Name::builder().expect("nombre");
    name.append_entry_by_nid(Nid::COMMONNAME, common_name)
        .expect("CN");
    let name = name.build();
    let mut builder = X509::builder().expect("certificado");
    builder.set_version(2).expect("version");
    builder
        .set_serial_number(
            &BigNum::from_u32(serial)
                .and_then(|serial| serial.to_asn1_integer())
                .expect("serie"),
        )
        .expect("serie");
    builder.set_subject_name(&name).expect("titular");
    builder
        .set_issuer_name(issuer.map_or(&name, |ca| ca.certificate.subject_name()))
        .expect("emisor");
    builder.set_pubkey(&key).expect("clave publica");
    builder
        .set_not_before(&Asn1Time::days_from_now(0).expect("fecha"))
        .expect("inicio");
    builder
        .set_not_after(&Asn1Time::days_from_now(30).expect("fecha"))
        .expect("fin");
    builder
        .sign(issuer.map_or(&key, |ca| &ca.key), MessageDigest::sha256())
        .expect("firma del certificado");
    Holder {
        certificate: builder.build(),
        key,
    }
}

struct Chain {
    signer: Holder,
    authority: Holder,
}

fn a_chain_with(key: PKey<Private>) -> Chain {
    let authority = issued("AC DE PRUEBAS", 1, an_rsa_key(), None);
    let signer = issued("FIRMANTE DE PRUEBAS", 2, key, Some(&authority));
    Chain { signer, authority }
}

impl Chain {
    fn b64(&self) -> String {
        format!("{};{}", b64(&self.signer.der()), b64(&self.authority.der()))
    }
}

/// Lo que haría el token: hashear y firmar con el mecanismo del algoritmo, ECDSA en DER.
fn token_signs(key: &PKey<Private>, algorithm: &str, data: &[u8]) -> Vec<u8> {
    let digest = if algorithm.starts_with("SHA512") {
        MessageDigest::sha512()
    } else if algorithm.starts_with("SHA384") {
        MessageDigest::sha384()
    } else {
        MessageDigest::sha256()
    };
    let mut signer = Signer::new(digest, key).expect("firmador");
    if algorithm.ends_with("andMGF1") {
        signer
            .set_rsa_padding(Padding::PKCS1_PSS)
            .expect("relleno PSS");
        signer
            .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
            .expect("sal PSS");
        signer.set_rsa_mgf1_md(digest).expect("MGF1");
    }
    signer.update(data).expect("datos");
    signer.sign_to_vec().expect("firma")
}

struct Asked<'a> {
    operation: SignatureOperation,
    algorithm: &'a str,
    extra_params: &'a str,
}

const SIGN: Asked<'static> = Asked {
    operation: SignatureOperation::Sign,
    algorithm: "SHA256withRSA",
    extra_params: "",
};

fn presigned(rades: &Rades, chain: &Chain, asked: &Asked<'_>) -> Result<PreSignature, BridgeError> {
    rades.presign(PreSignRequest {
        format: Format::Cms,
        operation: asked.operation,
        document_b64: &b64(A_DOCUMENT),
        algorithm: asked.algorithm,
        certificate_chain_b64: &chain.b64(),
        extra_params: asked.extra_params,
    })
}

fn postsigned(
    rades: &Rades,
    chain: &Chain,
    presigned: &PreSignature,
    algorithm: &str,
    document: &[u8],
) -> Result<Vec<u8>, BridgeError> {
    let signatures = presigned
        .signed_one_by_one(|pre| Ok::<_, ()>(token_signs(&chain.signer.key, algorithm, pre)))
        .expect("el token firma");
    let sealed = presigned
        .sealed_with(signatures, presigned.stamp())
        .expect("el sello vuelve intacto");
    rades.postsign(PostSignRequest {
        format: Format::Cms,
        document_b64: &b64(document),
        certificate_chain_b64: &chain.b64(),
        sealed: &sealed,
    })
}

fn a_cms(chain: &Chain, asked: &Asked<'_>) -> Vec<u8> {
    let rades = Rades::at(|| AN_INSTANT);
    let presigned = presigned(&rades, chain, asked).expect("la prefirma sale");
    postsigned(&rades, chain, &presigned, asked.algorithm, A_DOCUMENT).expect("la postfirma sale")
}

fn openssl_verifies(signature: &[u8], detached: Option<&[u8]>) -> Vec<u8> {
    let mut cms = CmsContentInfo::from_der(signature).expect("openssl lee el CMS");
    let mut content = Vec::new();
    cms.verify(
        None,
        None,
        detached,
        Some(&mut content),
        CMSOptions::NO_SIGNER_CERT_VERIFY | CMSOptions::BINARY,
    )
    .expect("openssl verifica la firma");
    content
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

mod cades;

#[test]
fn rades_signs_cms_and_cades_and_nothing_else_yet() {
    for format in Format::ALL {
        assert_eq!(
            Rades::signs(format),
            matches!(format, Format::Cms | Format::Cades),
            "{format}"
        );
    }
}

#[test]
fn without_a_mode_the_cms_carries_the_document_and_openssl_verifies_it() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cms(&chain, &SIGN);

    assert_eq!(openssl_verifies(&signature, None), A_DOCUMENT);
}

#[test]
fn an_explicit_cms_leaves_the_document_out_and_verifies_against_it() {
    let chain = a_chain_with(an_rsa_key());

    let signature = a_cms(
        &chain,
        &Asked {
            extra_params: "mode=explicit\n",
            ..SIGN
        },
    );

    let decoded = ::rades::cades::decode(&signature).expect("rades lee su CMS");
    assert_eq!(decoded.content(), None);
    openssl_verifies(&signature, Some(A_DOCUMENT));
}

#[test]
fn the_whole_chain_goes_in_unless_the_site_asks_for_the_signer_only() {
    let chain = a_chain_with(an_rsa_key());

    let by_default = a_cms(&chain, &SIGN);
    let signer_only = a_cms(
        &chain,
        &Asked {
            extra_params: "includeOnlySignningCertificate=true\n",
            ..SIGN
        },
    );

    assert!(contains(&by_default, &chain.authority.der()));
    assert!(contains(&signer_only, &chain.signer.der()));
    assert!(!contains(&signer_only, &chain.authority.der()));
}

#[test]
fn rsa_pkcs1_rsa_pss_and_ecdsa_sign_in_sha256_sha384_and_sha512() {
    let rsa = a_chain_with(an_rsa_key());
    let ec = a_chain_with(an_ec_key());
    let cases = [
        (&rsa, "SHA256withRSA"),
        (&rsa, "SHA384withRSA"),
        (&rsa, "SHA512withRSA"),
        (&rsa, "SHA256withRSAandMGF1"),
        (&rsa, "SHA384withRSAandMGF1"),
        (&rsa, "SHA512withRSAandMGF1"),
        (&ec, "SHA256withECDSA"),
        (&ec, "SHA384withECDSA"),
        (&ec, "SHA512withECDSA"),
    ];

    for (chain, algorithm) in cases {
        let signature = a_cms(chain, &Asked { algorithm, ..SIGN });

        assert_eq!(
            openssl_verifies(&signature, None),
            A_DOCUMENT,
            "{algorithm}"
        );
    }
}

#[test]
fn sha1_is_refused_before_the_token_with_an_error_that_names_it() {
    let chain = a_chain_with(an_rsa_key());

    for algorithm in ["SHA1withRSA", "SHA1withECDSA"] {
        let refused = presigned(
            &Rades::at(|| AN_INSTANT),
            &chain,
            &Asked { algorithm, ..SIGN },
        )
        .expect_err("rades no firma con SHA-1");

        assert!(
            matches!(&refused, BridgeError::NotSupported(what) if what == algorithm),
            "{refused}"
        );
    }
}

#[test]
fn a_key_rades_does_not_know_is_refused_before_the_token_with_its_name() {
    let chain = a_chain_with(an_rsa_key());

    let refused = presigned(
        &Rades::at(|| AN_INSTANT),
        &chain,
        &Asked {
            extra_params: "mode=implicit\ntsaURL=https\\://tsa.example\n",
            ..SIGN
        },
    )
    .expect_err("rades no sella");

    assert!(
        matches!(&refused, BridgeError::NotSupported(what) if what == "tsaURL"),
        "{refused}"
    );
    assert!(refused.to_string().contains("tsaURL"), "{refused}");
}

#[test]
fn a_mode_that_is_neither_implicit_nor_explicit_is_refused_with_its_value() {
    let chain = a_chain_with(an_rsa_key());

    let refused = presigned(
        &Rades::at(|| AN_INSTANT),
        &chain,
        &Asked {
            extra_params: "mode=enveloped\n",
            ..SIGN
        },
    )
    .expect_err("no hay tal modo");

    assert!(refused.to_string().contains("mode=enveloped"), "{refused}");
}

#[test]
fn a_cms_is_neither_cosigned_nor_countersigned_by_rades() {
    let chain = a_chain_with(an_rsa_key());

    for operation in [SignatureOperation::Cosign, SignatureOperation::Countersign] {
        let refused = presigned(
            &Rades::at(|| AN_INSTANT),
            &chain,
            &Asked { operation, ..SIGN },
        )
        .expect_err("rades todavía no multifirma");

        assert!(
            matches!(
                refused,
                BridgeError::UnsupportedOperation(Format::Cms, asked) if asked == operation
            ),
            "{operation:?}: {refused}"
        );
    }
}

#[test]
fn the_signing_time_is_the_one_taken_at_the_presign_and_not_at_the_postsign() {
    let chain = a_chain_with(an_rsa_key());
    let presigned = presigned(&Rades::at(|| AN_INSTANT), &chain, &SIGN).expect("prefirma");

    let signature = postsigned(
        &Rades::at(|| AN_INSTANT + 3_600),
        &chain,
        &presigned,
        SIGN.algorithm,
        A_DOCUMENT,
    )
    .expect("la postfirma rehace la fase 1 desde la sesión");

    let decoded = ::rades::cades::decode(&signature).expect("rades lee su CMS");
    assert_eq!(
        decoded.signers()[0].signing_time(),
        Some(::rades::SigningTime::from_unix_seconds(AN_INSTANT).expect("hora"))
    );
}

#[test]
fn a_session_altered_after_the_presign_is_refused_by_the_postsign() {
    let chain = a_chain_with(an_rsa_key());
    let rades = Rades::at(|| AN_INSTANT);
    let honest = presigned(&rades, &chain, &SIGN).expect("prefirma");
    let altered = PreSignature {
        session: format!("{} ", honest.session()),
        ..honest.clone()
    };
    assert_ne!(altered.session(), honest.session());

    let refused = postsigned(&rades, &chain, &altered, SIGN.algorithm, A_DOCUMENT)
        .expect_err("el sello no cubre la sesión alterada");

    assert!(matches!(refused, BridgeError::Failed(_)), "{refused}");
}

#[test]
fn a_postsign_whose_data_to_sign_is_not_what_the_token_signed_is_refused() {
    let chain = a_chain_with(an_rsa_key());
    let rades = Rades::at(|| AN_INSTANT);
    let presigned = presigned(&rades, &chain, &SIGN).expect("prefirma");

    let refused = postsigned(
        &rades,
        &chain,
        &presigned,
        SIGN.algorithm,
        b"otro documento",
    )
    .expect_err("el token firmó otros datos");

    assert!(
        refused
            .to_string()
            .contains("no son los que firmó el token"),
        "{refused}"
    );
}
