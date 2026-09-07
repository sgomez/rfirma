//! Los puertos que el trámite pide a los vecinos, servidos por sus raíces: certificados, documento de paso y firma.

use std::path::PathBuf;

use crate::commands::Failure;
use crate::documents::adapters::failures::code_of_document;
use crate::documents::DocumentsRoot;
use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::IdentityRoot;
use crate::signing::adapters::failures::told_of_cycle;
use crate::signing::SigningRoot;
use crate::site::domain::protocol::SafCode;
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{Certificates, ScratchDocuments, SiteSigning, SiteSigningRequest};

/// Las tres raíces vecinas, vistas por el trámite a través de sus puertos.
#[derive(Clone, Copy)]
pub struct Neighbours<'a> {
    /// Quien tiene los certificados.
    pub identity: &'a IdentityRoot,
    /// Quien apunta los documentos.
    pub documents: &'a DocumentsRoot,
    /// Quien firma.
    pub signing: &'a SigningRoot,
}

impl Certificates for Neighbours<'_> {
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        self.identity.certificates()
    }

    fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        self.identity.rows_of(found)
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        self.identity.usable(found, handle)
    }
}

impl ScratchDocuments for Neighbours<'_> {
    fn open_unrecorded(&self, path: PathBuf) -> String {
        self.documents.open_unrecorded(path)
    }
}

impl SiteSigning for Neighbours<'_> {
    fn begin(&self, request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal> {
        let document = self
            .documents
            .opened_document(request.document)
            .map_err(|error| {
                signing_refusal_of((Failure::from(error.clone()), code_of_document(&error)))
            })?;
        self.signing
            .begin_for_the_site(
                request.document,
                document,
                request.certificate,
                request.from_the_site,
                request.allow_unregistered_signatures,
                &self.identity.signer(),
            )
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        let signed = self
            .signing
            .finish()
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))?;
        Ok(SiteSignature {
            signed: signed.completed.into_pdf(),
            signer_der: signed.signer_der,
        })
    }
}

/// Lo que la sede y la ventana reciben de un fallo, tal como lo decidió quien lo tradujo.
pub fn signing_refusal_of((told, code): (Failure, SafCode)) -> SigningRefusal {
    SigningRefusal {
        code,
        situation: told.situation,
        detail: told.detail,
        attempts_left: told.attempts_left,
    }
}
