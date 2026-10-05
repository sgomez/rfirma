use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use openssl::hash::MessageDigest;
use openssl::rsa::Padding as RsaPadding;
use openssl::sign::{RsaPssSaltlen, Verifier};
use openssl::x509::X509;

use super::cng::{cancelled_by_the_person, hash_name, padding_of, situation_of, Padding};
use super::{candidate_modules, is_the_user_store, user_store, WindowsToken};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::Situation;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;
use crate::signing::adapters::ffi::{locate, NativeBridge};
use crate::signing::application::cycle::{self, SigningRequest};
use crate::signing::domain::bridge::{Format, SignatureOperation, XadesVariant};
use crate::signing::domain::{AdmissibleDocument, SignatureConfig, Waivers};

#[test]
fn the_user_store_is_told_apart_from_a_pkcs11_module() {
    assert!(is_the_user_store(&user_store()));
    assert!(!is_the_user_store(&Store::module(
        r"C:\Windows\System32\opensc-pkcs11.dll"
    )));
}

#[test]
fn the_pkcs11_candidates_live_under_program_files_and_system32() {
    let candidates = candidate_modules(Path::new("P"), Path::new("S"));

    assert!(candidates.contains(&PathBuf::from(
        "P/OpenSC Project/OpenSC/pkcs11/opensc-pkcs11.dll"
    )));
    assert!(candidates.contains(&Path::new("S").join("DNIe_P11_priv.dll")));
}

#[test]
fn the_user_store_asks_windows_for_the_pin_and_not_rfirma() {
    let reference = CertificateRef::new(user_store(), "CurrentUser\\MY", "x", Some(vec![0; 20]));

    assert_eq!(
        WindowsToken.secret_of(&reference),
        Ok(StoreSecret::NotNeeded)
    );
    assert_eq!(
        WindowsToken.accepts_the_secret(&reference, &ProtectedSecret::new(b"")),
        Ok(())
    );
}

#[test]
fn each_algorithm_gets_the_padding_cng_expects() {
    assert_eq!(padding_of(SignatureAlgorithm::Sha256Rsa), Padding::Pkcs1);
    assert_eq!(padding_of(SignatureAlgorithm::Sha512RsaPss), Padding::Pss);
    assert_eq!(padding_of(SignatureAlgorithm::Sha384Ecdsa), Padding::None);
}

fn wide_text(name: *const u16) -> String {
    let length = (0..).take_while(|&i| unsafe { *name.add(i) } != 0).count();
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(name, length) })
}

#[test]
fn each_algorithm_gets_its_own_cng_hash_name() {
    let expected = [
        (SignatureAlgorithm::Sha1Rsa, "SHA1"),
        (SignatureAlgorithm::Sha1Ecdsa, "SHA1"),
        (SignatureAlgorithm::Sha256Rsa, "SHA256"),
        (SignatureAlgorithm::Sha256RsaPss, "SHA256"),
        (SignatureAlgorithm::Sha256Ecdsa, "SHA256"),
        (SignatureAlgorithm::Sha384Rsa, "SHA384"),
        (SignatureAlgorithm::Sha512Ecdsa, "SHA512"),
    ];
    for (algorithm, name) in expected {
        assert_eq!(wide_text(hash_name(algorithm)), name);
    }
}

#[test]
fn smart_card_codes_map_to_their_situations() {
    assert_eq!(situation_of(0x8010_006B), Situation::IncorrectPin);
    assert_eq!(situation_of(0x8010_006C), Situation::PinLocked);
    assert_eq!(situation_of(0x8010_000C), Situation::TokenAbsent);
    assert_eq!(situation_of(0x8009_0016), Situation::CertificateNotFound);
    assert_eq!(situation_of(0x1234_5678), Situation::Unknown);
}

#[test]
fn cancelling_the_windows_pin_window_is_told_apart() {
    assert!(cancelled_by_the_person(0x8010_006E));
    assert!(cancelled_by_the_person(0x8007_04C7));
    assert!(!cancelled_by_the_person(0x8010_006B));
}

#[test]
fn a_missing_thumbprint_is_a_certificate_not_found() {
    let reference = CertificateRef::new(user_store(), "CurrentUser\\MY", "x", Some(vec![0; 20]));

    let refused = WindowsToken
        .sign_with_secret(
            &reference,
            &ProtectedSecret::new(b""),
            SignatureAlgorithm::Sha256Rsa,
            b"hola",
        )
        .expect_err("no hay ningun certificado con esa huella");

    assert_eq!(refused.situation(), Situation::CertificateNotFound);
}

#[test]
fn lists_a_certificate_of_the_user_store_with_its_private_key() {
    let temporary = TemporaryCertificate::create(RSA_IN_CNG);

    let listed = temporary.listed();

    assert_eq!(listed.reference().store(), user_store());
    assert!(listed
        .subject()
        .is_some_and(|subject| subject.contains(&temporary.subject)));
}

#[test]
fn signs_with_rsa_through_cng() {
    let temporary = TemporaryCertificate::create(RSA_IN_CNG);

    let signature = temporary.signed(SignatureAlgorithm::Sha256Rsa);

    assert!(temporary.verifies(SignatureAlgorithm::Sha256Rsa, &signature));
}

#[test]
fn signs_with_an_rsa_key_of_a_legacy_csp_through_cng() {
    let temporary = TemporaryCertificate::create(RSA_IN_A_LEGACY_CSP);

    let signature = temporary.signed(SignatureAlgorithm::Sha256Rsa);

    assert!(temporary.verifies(SignatureAlgorithm::Sha256Rsa, &signature));
}

#[test]
fn signs_with_rsa_pss_through_cng() {
    let temporary = TemporaryCertificate::create(RSA_IN_CNG);

    let signature = temporary.signed(SignatureAlgorithm::Sha384RsaPss);

    assert!(temporary.verifies(SignatureAlgorithm::Sha384RsaPss, &signature));
}

#[test]
fn signs_with_ecdsa_through_cng_in_der() {
    let temporary = TemporaryCertificate::create(ECDSA_IN_CNG);

    let signature = temporary.signed(SignatureAlgorithm::Sha256Ecdsa);

    assert_eq!(signature[0], 0x30, "ECDSA viaja en DER, no en r||s");
    assert!(temporary.verifies(SignatureAlgorithm::Sha256Ecdsa, &signature));
}

#[test]
fn an_rsa_key_does_not_offer_ecdsa() {
    let temporary = TemporaryCertificate::create(RSA_IN_CNG);
    let reference = temporary.listed().reference().clone();

    assert_eq!(
        WindowsToken.offers(&reference, SignatureAlgorithm::Sha256Rsa),
        Ok(())
    );
    assert_eq!(
        WindowsToken
            .offers(&reference, SignatureAlgorithm::Sha256Ecdsa)
            .map_err(|error| error.situation()),
        Err(Situation::MechanismNotOffered)
    );
}

const RSA_IN_CNG: &str =
    "-Provider 'Microsoft Software Key Storage Provider' -KeyAlgorithm RSA -KeyLength 2048";
const ECDSA_IN_CNG: &str =
    "-Provider 'Microsoft Software Key Storage Provider' -KeyAlgorithm ECDSA_nistP256";
const RSA_IN_A_LEGACY_CSP: &str =
    "-Provider 'Microsoft Enhanced RSA and AES Cryptographic Provider' -KeyAlgorithm RSA -KeyLength 2048";

const DATA: &[u8] = b"lo que Java manda firmar";

/// Un certificado autofirmado en `Cert:\CurrentUser\My` que se borra, con su clave, al soltarse.
struct TemporaryCertificate {
    subject: String,
    thumbprint: String,
}

impl TemporaryCertificate {
    fn create(key: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.subsec_nanos())
            .unwrap_or_default();
        let subject = format!("rfirma-cng-test-{}-{nanos}", std::process::id());
        let script = format!(
            "$ErrorActionPreference = 'Stop'; \
             (New-SelfSignedCertificate -Subject 'CN={subject}' \
             -CertStoreLocation Cert:\\CurrentUser\\My \
             {key} \
             -KeyUsage DigitalSignature -NotAfter (Get-Date).AddDays(1)).Thumbprint"
        );
        // Varias pruebas crean su certificado a la vez y New-SelfSignedCertificate falla a veces
        // sin decir nada: se crean de uno en uno y se reintenta una vez.
        let _one_at_a_time = CREATING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (mut thumbprint, mut errors) = powershell_with_errors(&script);
        if thumbprint.len() != 40 {
            (thumbprint, errors) = powershell_with_errors(&script);
        }
        assert_eq!(
            thumbprint.len(),
            40,
            "New-SelfSignedCertificate: {thumbprint} {errors}"
        );
        Self {
            subject,
            thumbprint,
        }
    }

    fn listed(&self) -> TokenCertificate {
        WindowsToken
            .list(&user_store())
            .expect("el almacen del usuario deberia listarse")
            .into_iter()
            .find(|certificate| {
                certificate
                    .reference()
                    .cka_id()
                    .is_some_and(|id| hex(id) == self.thumbprint)
            })
            .expect("el certificado temporal deberia estar con su clave privada")
    }

    fn signed(&self, algorithm: SignatureAlgorithm) -> Vec<u8> {
        WindowsToken
            .sign_with_secret(
                self.listed().reference(),
                &ProtectedSecret::new(b""),
                algorithm,
                DATA,
            )
            .expect("CNG deberia firmar")
    }

    fn verifies(&self, algorithm: SignatureAlgorithm, signature: &[u8]) -> bool {
        let key = X509::from_der(self.listed().der())
            .and_then(|certificate| certificate.public_key())
            .expect("el certificado deberia traer su clave publica");
        let digest = match algorithm {
            SignatureAlgorithm::Sha384RsaPss => MessageDigest::sha384(),
            _ => MessageDigest::sha256(),
        };
        let mut verifier = Verifier::new(digest, &key).expect("verificador");
        if algorithm == SignatureAlgorithm::Sha384RsaPss {
            verifier.set_rsa_padding(RsaPadding::PKCS1_PSS).unwrap();
            verifier
                .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
                .unwrap();
            verifier.set_rsa_mgf1_md(digest).unwrap();
        }
        verifier.verify_oneshot(signature, DATA).unwrap_or(false)
    }
}

impl Drop for TemporaryCertificate {
    fn drop(&mut self) {
        powershell(&format!(
            "Remove-Item -Path Cert:\\CurrentUser\\My\\{} -DeleteKey",
            self.thumbprint
        ));
    }
}

/// Las creaciones de certificados temporales, de una en una.
static CREATING: Mutex<()> = Mutex::new(());

fn powershell(script: &str) -> String {
    powershell_with_errors(script).0
}

/// La salida de PowerShell y, aparte, lo que dijo por la de errores.
fn powershell_with_errors(script: &str) -> (String, String) {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .expect("powershell deberia estar en Windows");
    (
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

#[test]
#[ignore = "grada C: necesita rfirma_crypto.dll en RFIRMA_LIB_DIR"]
fn signs_xades_cades_and_pades_with_the_user_store_on_the_native_bridge() {
    let temporary = TemporaryCertificate::create(RSA_IN_CNG);
    let certificate = temporary.listed();
    let bridge = native_bridge();
    let xml = include_bytes!("../../../../../../testdata/reference/document.xml");
    let pdf = include_bytes!("../../../../../../testdata/site-driver/test/samples/pades-rsa.pdf");

    for (format, document) in [
        (Format::Xades(XadesVariant::Enveloping), &xml[..]),
        (Format::Cades, &xml[..]),
        (Format::Pades, &pdf[..]),
    ] {
        let signed = signed_on_the_bridge(&bridge, &certificate, format, document);
        assert!(signed.len() > document.len(), "{format:?}");
    }
}

fn native_bridge() -> NativeBridge {
    let directory = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let library = locate(&|name| std::env::var_os(name), &directory)
        .expect("RFIRMA_LIB_DIR deberia apuntar a rfirma_crypto.dll");
    NativeBridge::open_at(&library).expect("la libreria deberia cargarse")
}

fn signed_on_the_bridge(
    bridge: &NativeBridge,
    certificate: &TokenCertificate,
    format: Format,
    document: &[u8],
) -> Vec<u8> {
    let chain = certificate.chain();
    let config = SignatureConfig {
        placement: None,
        layer2_text: Some(String::new()),
        rubric_image: None,
        allow_unregistered_signatures: false,
    };
    let cycle = cycle::presign(
        bridge,
        SigningRequest {
            format,
            algorithm: SignatureAlgorithm::Sha256Rsa,
            operation: SignatureOperation::Sign,
            document: AdmissibleDocument::check_for(format, document, Waivers::NONE)
                .expect("el documento deberia ser firmable"),
            chain: &chain,
            config: &config,
            from_the_site: &BTreeMap::new(),
            certificate: certificate.reference(),
        },
    )
    .unwrap_or_else(|error| panic!("la prefirma {format:?} deberia salir: {error}"));
    let signatures = cycle
        .sign_on_token(&WindowsToken, &ProtectedSecret::new(b""))
        .expect("CNG deberia firmar los atributos");
    cycle
        .postsign(bridge, signatures, &cycle.seal_in_transit())
        .unwrap_or_else(|error| panic!("la postfirma {format:?} deberia ensamblar: {error}"))
        .signed_document()
        .to_vec()
}
