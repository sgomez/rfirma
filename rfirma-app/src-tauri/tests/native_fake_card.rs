//! Prueba de grada C: un PDF firmado con la tarjeta falsa, con el adaptador PKCS#11 de verdad y la postfirma (ADR-0014).

#![cfg(not(windows))]

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use fake_pkcs11::FakeCard;
use rfirma_lib::desktop::adapters::command_line_ports::NativeFilter;
use rfirma_lib::desktop::ports::CertificateFilter;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::adapters::pkcs11::RealToken;
use rfirma_lib::identity::application::certificates::certificates_with_their_chains;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
use rfirma_lib::signing::domain::document_signatures::Validity;
use rfirma_lib::site::domain::protocol::site_filter;

use support::{a_cycle_signed_by, a_one_page_pdf, bridge};

const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";
const AUTHENTICATION_CERTIFICATE: &str = "CertAutenticacion";

fn signing_certificate_of(card: &FakeCard) -> TokenCertificate {
    pkcs11::list_certificates_across(&[Store::module(card.module())])
        .expect("la tarjeta falsa debería listarse")
        .into_iter()
        .find(|certificate| certificate.reference().label() == SIGNING_CERTIFICATE)
        .expect("la tarjeta falsa lleva el certificado de firma")
}

/// Lo que el filtro del original deja de la tarjeta, el mismo motor para una sede y para la línea de órdenes.
fn offered_under(card: &FakeCard, expression: &str) -> Vec<String> {
    let candidates = certificates_with_their_chains(&RealToken, &[Store::module(card.module())])
        .expect("la tarjeta falsa debería listarse");
    let filter = site_filter(&[("filters".to_owned(), expression.to_owned())]);
    let mut labels: Vec<String> = NativeFilter
        .accepted(&filter, candidates)
        .expect("el motor de filtros debería contestar")
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    labels.sort();
    labels
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_site_filters_choose_between_the_two_certificates_of_the_dnie_as_autofirma_does() {
    let card = FakeCard::new().expect("la tarjeta falsa debería montarse");

    assert_eq!(offered_under(&card, "signingcert:"), [SIGNING_CERTIFICATE]);
    assert_eq!(
        offered_under(&card, "authcert:"),
        [AUTHENTICATION_CERTIFICATE]
    );
    assert_eq!(offered_under(&card, "dnie:"), [SIGNING_CERTIFICATE]);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_pdf_signed_with_the_fake_card_is_valid_and_the_card_saw_the_pin_once() {
    let card = FakeCard::new().expect("la tarjeta falsa debería montarse");
    let certificate = signing_certificate_of(&card);

    let signed = a_cycle_signed_by(
        &certificate,
        FakeCard::PIN,
        Format::Pades,
        ALGORITHM,
        &a_one_page_pdf(),
        SignatureOperation::Sign,
        &[],
    );

    let report = bridge()
        .previous_signatures(&base64::engine::general_purpose::STANDARD.encode(&signed))
        .expect("el puente debería leer las firmas");
    assert_eq!(report.count(), 1);
    assert_eq!(report.signatures()[0].validity, Validity::Valid);
    assert_eq!(
        card.calls_to("C_Login").len(),
        1,
        "{:?}",
        card.calls_to("C_Login")
    );
    assert!(!card.calls_to("C_Sign").is_empty(), "{:?}", card.calls());
}

/// La lista en caliente de la ventana de sede, con el motor de filtros de verdad (ADR-0048).
mod the_site_window {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use base64::Engine;
    use fake_pkcs11::FakeCard;
    use rfirma_lib::identity::adapters::pkcs11::RealToken;
    use rfirma_lib::identity::application::certificates::{
        certificates_with_their_chains, rows_keeping_handles, rows_of, usable_certificate,
        ListedCertificates,
    };
    use rfirma_lib::identity::domain::certificate::{
        CertificateRef, ListedCertificate, TokenCertificate,
    };
    use rfirma_lib::identity::domain::error::TokenError;
    use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
    use rfirma_lib::identity::domain::secret::StoreSecret;
    use rfirma_lib::identity::domain::store::Store;
    use rfirma_lib::identity::ports::CertificateMemory;
    use rfirma_lib::memory_error::MemoryError;
    use rfirma_lib::signing::adapters::ffi::NativeBridge;
    use rfirma_lib::site::adapters::batch_services::RelayBatchServices;
    use rfirma_lib::site::adapters::codec::V4Codec;
    use rfirma_lib::site::adapters::scratch::RealScratch;
    use rfirma_lib::site::adapters::triphase_server::HttpTriphaseServer;
    use rfirma_lib::site::application::errand::{
        after_the_readers, attend, Acknowledgement, AfterTheReaders, Errand, ErrandDesk,
        ErrandStep, LiveErrand, ReplyHandle,
    };
    use rfirma_lib::site::domain::channel::ArrivalMode;
    use rfirma_lib::site::domain::protocol::{ChannelMessage, NegotiatedCredential};
    use rfirma_lib::site::domain::signing::{SigningRefusal, SiteSignature};
    use rfirma_lib::site::ports::{Neighbours, SiteSigningRequest};

    use super::{bridge, AUTHENTICATION_CERTIFICATE};

    struct NoMemory;

    impl CertificateMemory for NoMemory {
        fn remembered_certificate(&self) -> Option<CertificateRef> {
            None
        }

        fn remember_the_certificate(&self, _reference: &CertificateRef) -> Result<(), MemoryError> {
            Ok(())
        }

        fn forget_the_certificate(&self) -> Result<(), MemoryError> {
            Ok(())
        }
    }

    /// Lo que la sede ve de la tarjeta falsa: el listado de sede, con su certificado de autenticación.
    struct OnTheFakeCard {
        store: Store,
        home: tempfile::TempDir,
        listed: ListedCertificates,
    }

    impl OnTheFakeCard {
        fn rows_after_the_readers(
            &self,
            accepted: Vec<TokenCertificate>,
        ) -> Vec<ListedCertificate> {
            rows_keeping_handles(
                accepted,
                self.home.path(),
                &self.listed,
                &ListedCertificates::new(),
                &NoMemory,
            )
        }
    }

    // La selección de certificado no firma: el resto del puerto único no lo usa esta prueba.
    impl Neighbours for OnTheFakeCard {
        fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
            certificates_with_their_chains(&RealToken, std::slice::from_ref(&self.store))
        }

        fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
            rows_of(
                found,
                self.home.path(),
                &self.listed,
                &ListedCertificates::new(),
                &NoMemory,
            )
        }

        fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
            None
        }

        fn usable<'a>(
            &self,
            found: &'a [TokenCertificate],
            handle: &str,
        ) -> Result<&'a TokenCertificate, TokenError> {
            usable_certificate(found, handle, &self.listed)
        }

        fn automatic_selection_honoured(&self) -> bool {
            false
        }

        fn sha1_allowed(&self) -> bool {
            false
        }

        fn open_unrecorded(&self, _path: PathBuf) -> String {
            unreachable!("la selección no apunta documentos de paso")
        }

        fn begin(&self, _request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal> {
            unreachable!("la selección no abre el ciclo de la firma de sede")
        }

        fn sign_on_token(&self, _secret: &ProtectedSecret) -> Result<(), SigningRefusal> {
            unreachable!("la selección no firma")
        }

        fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
            unreachable!("la selección no cierra ningún ciclo")
        }

        fn the_pdf_password(&self, _after_a_wrong_one: bool) -> Option<String> {
            unreachable!("la selección no abre ningún PDF")
        }

        fn secret_of(
            &self,
            _certificate: &TokenCertificate,
        ) -> Result<StoreSecret, SigningRefusal> {
            unreachable!("la selección no pide el secreto")
        }

        fn sign(
            &self,
            _certificate: &TokenCertificate,
            _secret: &ProtectedSecret,
            _algorithm: &str,
            _data: &[u8],
        ) -> Result<Vec<u8>, SigningRefusal> {
            unreachable!("la selección no firma")
        }
    }

    fn a_desk<'a>(
        engine: &'a NativeBridge,
        neighbours: &'a OnTheFakeCard,
        scratch: &Path,
    ) -> ErrandDesk<'a, NativeBridge, NativeBridge> {
        ErrandDesk {
            engine,
            policies: engine,
            validation: engine,
            neighbours,
            scratch_dir: scratch.to_path_buf(),
            scratch: Arc::new(RealScratch),
            batch: Arc::new(RelayBatchServices::default()),
            triphase: Arc::new(HttpTriphaseServer::default()),
        }
    }

    fn a_selection_under(filter: &str) -> rfirma_lib::site::domain::protocol::AfirmaUrl {
        let properties =
            base64::engine::general_purpose::URL_SAFE.encode(format!("filters={filter}\n"));
        let text = format!(
            "afirma://selectcert?op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL&properties={properties}"
        );
        let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
            panic!("una URL del protocolo es una operación");
        };
        url
    }

    fn labels(rows: &[ListedCertificate]) -> Vec<&str> {
        rows.iter().map(|row| row.label.as_str()).collect()
    }

    #[test]
    #[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
    fn the_list_of_the_site_follows_the_card_under_authcert_and_keeps_the_handles_it_granted() {
        let card = FakeCard::new().expect("la tarjeta falsa debería montarse");
        let neighbours = OnTheFakeCard {
            store: Store::module(card.module()),
            home: tempfile::tempdir().expect("debería haber directorio temporal"),
            listed: ListedCertificates::new(),
        };
        let engine = bridge();
        let desk = a_desk(&engine, &neighbours, &neighbours.home.path().join("errand"));
        let live = LiveErrand::default();
        assert!(live.begin(Errand::of(
            NegotiatedCredential::Absent,
            ArrivalMode::Awaited,
            Arc::new(V4Codec),
        )));
        let reply = ReplyHandle::of(|_| Acknowledgement::immediate());
        let step = attend(&desk, a_selection_under("authcert:"), reply, &live);
        let Some(ErrandStep::AskingForConsent {
            certificates: granted,
            ..
        }) = step
        else {
            panic!("la tarjeta tiene el de autenticación: {step:?}");
        };
        assert_eq!(labels(&granted), [AUTHENTICATION_CERTIFICATE]);

        let again = after_the_readers(&desk, neighbours.listed().expect("se lista"), &live);
        let AfterTheReaders::Accepted(accepted) = again else {
            panic!("el consentimiento enseña una lista: {again:?}");
        };
        let rows = neighbours.rows_after_the_readers(accepted);
        assert_eq!(labels(&rows), [AUTHENTICATION_CERTIFICATE]);
        assert_eq!(rows[0].id, granted[0].id, "la sede ya concedió esa asa");

        card.take_out().expect("la tarjeta debería salir");
        let gone = after_the_readers(&desk, neighbours.listed().unwrap_or_default(), &live);
        assert!(
            matches!(&gone, AfterTheReaders::Accepted(accepted) if accepted.is_empty()),
            "{gone:?}"
        );
    }
}
