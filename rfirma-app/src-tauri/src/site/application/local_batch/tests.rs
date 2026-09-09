//! Pruebas de las guardas del bucle que no llegan a tocar el ciclo de firma.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::identity::application::tests::a_usable_certificate;
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::signing::domain::bridge::BridgeError;
use crate::site::domain::batch::parse_local_batch;
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{
    BatchServices, Certificates, Scratch, ScratchDocuments, SiteSigning, TokenSigning,
};

/// Vecinos, motor y polìtica que revientan si el bucle llega a usarlos: las dos guardas de
/// arriba deben devolver antes de tocar nada de esto.
struct Untouched;

impl Certificates for Untouched {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        unreachable!("la guarda no llega a listar certificados")
    }

    fn rows_of(&self, _found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        unreachable!("la guarda no llega a formar filas")
    }

    fn usable<'a>(
        &self,
        _found: &'a [TokenCertificate],
        _handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        unreachable!("la guarda no llega a buscar el certificado")
    }

    fn remembered(&self) -> Option<CertificateRef> {
        unreachable!("la guarda no llega a mirar lo recordado")
    }

    fn remember(&self, _chosen: &CertificateRef) {
        unreachable!("la guarda no llega a recordar nada")
    }

    fn forget_the_remembered(&self) {
        unreachable!("la guarda no llega a olvidar nada")
    }
}

impl ScratchDocuments for Untouched {
    fn open_unrecorded(&self, _path: PathBuf) -> String {
        unreachable!("la guarda no llega a abrir ningún documento")
    }
}

impl SiteSigning for Untouched {
    fn begin(
        &self,
        _request: crate::site::ports::SiteSigningRequest<'_>,
    ) -> Result<crate::identity::domain::secret::StoreSecret, SigningRefusal> {
        unreachable!("la guarda no llega a abrir el ciclo")
    }

    fn sign_on_token(&self, _secret: &str) -> Result<(), SigningRefusal> {
        unreachable!("la guarda no llega a firmar")
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        unreachable!("la guarda no llega a cerrar el ciclo")
    }
}

impl TokenSigning for Untouched {
    fn secret_of(
        &self,
        _certificate: &TokenCertificate,
    ) -> Result<crate::identity::domain::secret::StoreSecret, SigningRefusal> {
        unreachable!("la guarda no llega a pedir el secreto")
    }

    fn sign(
        &self,
        _certificate: &TokenCertificate,
        _secret: &str,
        _algorithm: &str,
        _data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        unreachable!("la guarda no llega a firmar bytes")
    }
}

impl FilterEngine for Untouched {
    fn select(&self, _properties: &str, _certificates: &str) -> Result<Vec<usize>, BridgeError> {
        unreachable!("la guarda no llega a filtrar")
    }
}

impl PolicyEngine for Untouched {
    fn expand(&self, _extra_params: &str, _format: &str) -> Result<String, BridgeError> {
        unreachable!("la guarda no llega a expandir políticas")
    }
}

impl Scratch for Untouched {
    fn make_the_folder(&self, _folder: &Path) -> Result<(), String> {
        unreachable!("la guarda no llega a tocar disco")
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), String> {
        unreachable!("la guarda no llega a escribir nada")
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, String> {
        unreachable!("la guarda no llega a leer nada")
    }

    fn erase(&self, _path: &Path) {
        unreachable!("la guarda no llega a borrar nada")
    }
}

impl BatchServices for Untouched {
    fn presign(
        &self,
        _service_url: &str,
        _format: crate::site::domain::batch::BatchFormat,
        _lote: &str,
        _certs: &[Vec<u8>],
    ) -> Result<Vec<u8>, crate::site::domain::batch_error::BatchError> {
        unreachable!("la guarda no llega a hablar con los servlets")
    }

    fn postsign(
        &self,
        _service_url: &str,
        _format: crate::site::domain::batch::BatchFormat,
        _lote: &str,
        _certs: &[Vec<u8>],
        _triphase: &crate::site::domain::batch::TriphaseData,
    ) -> Result<Vec<u8>, crate::site::domain::batch_error::BatchError> {
        unreachable!("la guarda no llega a hablar con los servlets")
    }
}

fn a_desk_that_is_never_touched(
    home: &Path,
) -> ErrandDesk<'static, Untouched, Untouched, Untouched> {
    ErrandDesk {
        engine: Box::leak(Box::new(Untouched)),
        policies: Box::leak(Box::new(Untouched)),
        validation: &crate::site::application::tests::NotAsked,
        neighbours: Untouched,
        scratch_dir: home.join("errand"),
        scratch: Arc::new(Untouched),
        batch: Arc::new(Untouched),
    }
}

fn a_local_batch(json: &str) -> LocalBatch {
    parse_local_batch(json.as_bytes()).expect("el lote de la prueba deberia ser valido")
}

#[test]
fn an_empty_batch_cannot_even_start() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let desk = a_desk_that_is_never_touched(home.path());
    let certificate = a_usable_certificate("FIRMA");
    let batch = a_local_batch(
        r#"{"algorithm":"SHA256","format":"auto","stoponerror":false,"singlesigns":[]}"#,
    );

    let refusal = signed_local_batch(&desk, &certificate, "1234", &batch)
        .expect_err("un lote sin firmas no puede empezar");

    assert!(matches!(refusal, SiteRefusal::LocalBatch(_)));
}

#[test]
fn a_batch_with_an_algorithm_rfirma_does_not_sign_is_refused() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let desk = a_desk_that_is_never_touched(home.path());
    let certificate = a_usable_certificate("FIRMA");
    let batch = a_local_batch(
        r#"{"algorithm":"SHA1","format":"auto","stoponerror":false,"singlesigns":[{"id":"1","datareference":"ZGF0bw=="}]}"#,
    );

    let refusal =
        signed_local_batch(&desk, &certificate, "1234", &batch).expect_err("SHA1 no se atiende");

    assert!(matches!(refusal, SiteRefusal::LocalBatch(_)));
}
