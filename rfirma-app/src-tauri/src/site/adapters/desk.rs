//! Los puertos que el trámite pide a los vecinos, servidos por sus raíces: certificados, documento de paso y firma.

use std::path::PathBuf;

use crate::crossing::Failure;
use crate::documents::adapters::failures::code_of_document;
use crate::documents::DocumentsRoot;
use crate::identity::adapters::failures::code_of_token;
use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::IdentityRoot;
use crate::signing::adapters::failures::told_of_cycle;
use crate::signing::ports::{SecretName, SecretPromptRequest, Signer};
use crate::signing::{DeclaredByTheSite, SigningRoot};
use crate::site::domain::protocol::{AskedAlgorithm, SafCode};
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::ports::{
    Certificates, ScratchDocuments, SiteSigning, SiteSigningRequest, TokenSigning,
};

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

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        self.identity.discovered_module(library)
    }

    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        self.identity.usable(found, handle)
    }

    fn automatic_selection_honoured(&self) -> bool {
        self.signing.configuration().honour_automatic_selection
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
        let algorithm = composed_for(request.algorithm, request.certificate.key_kind())
            .map_err(refusal_of_token)?;
        self.signing
            .begin_for_the_site(
                request.document,
                document,
                request.certificate,
                DeclaredByTheSite {
                    format: request.format,
                    algorithm,
                    operation: request.operation,
                    parameters: request.from_the_site,
                    allow_unregistered_signatures: request.allow_unregistered_signatures,
                },
                &self.identity.signer(),
            )
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))
    }

    fn sign_on_token(&self, secret: &ProtectedSecret) -> Result<(), SigningRefusal> {
        self.signing
            .sign_on_token(&self.identity.signer(), secret)
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))
    }

    fn finish(&self) -> Result<SiteSignature, SigningRefusal> {
        let signed = self
            .signing
            .finish()
            .map_err(|failure| signing_refusal_of(told_of_cycle(&failure)))?;
        Ok(SiteSignature {
            signature: signed.completed.into_signed_document(),
            signer_der: signed.signer_der,
        })
    }

    fn the_pdf_password(&self, after_a_wrong_one: bool) -> Option<String> {
        let typed = self
            .signing
            .prompter
            .prompt_secret(&SecretPromptRequest {
                secret: SecretName::DocumentPassword,
                holder: None,
                language: self.signing.configuration().language,
                incorrect_secret: after_a_wrong_one,
            })
            .ok()?;
        typed.as_str().ok().map(str::to_owned)
    }
}

impl TokenSigning for Neighbours<'_> {
    fn secret_of(&self, certificate: &TokenCertificate) -> Result<StoreSecret, SigningRefusal> {
        secret_for_the_batch(&self.identity.signer(), certificate)
    }

    fn sign(
        &self,
        certificate: &TokenCertificate,
        secret: &ProtectedSecret,
        algorithm: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal> {
        signed_by_the_token(
            &self.identity.signer(),
            certificate,
            secret,
            algorithm,
            data,
        )
    }
}

/// El secreto del certificado, pedido una sola vez para todas las firmas del lote.
pub fn secret_for_the_batch(
    signer: &dyn Signer,
    certificate: &TokenCertificate,
) -> Result<StoreSecret, SigningRefusal> {
    signer
        .secret_of(certificate.reference())
        .map_err(refusal_of_token)
}

/// Los bytes firmados por el token con el algoritmo que declaró la sede.
pub fn signed_by_the_token(
    signer: &dyn Signer,
    certificate: &TokenCertificate,
    secret: &ProtectedSecret,
    algorithm: &str,
    data: &[u8],
) -> Result<Vec<u8>, SigningRefusal> {
    let asked = AskedAlgorithm::named(algorithm)
        .ok_or_else(|| no_mechanism_for(algorithm))
        .and_then(|asked| composed_for(asked, certificate.key_kind()))
        .map_err(refusal_of_token)?;
    signer
        .sign_with_secret(certificate.reference(), secret, asked, data)
        .map_err(refusal_of_token)
}

/// La huella que pide la sede, compuesta con la clase de clave del certificado (`composeSignatureAlgorithmName`, 1.9.2).
pub fn composed_for(
    asked: AskedAlgorithm,
    key: Option<KeyKind>,
) -> Result<SignatureAlgorithm, TokenError> {
    let key = key.ok_or_else(|| {
        TokenError::new(
            Situation::KeyNotRsa,
            "la clave del certificado no es RSA ni de curva eliptica",
        )
    })?;
    Ok(match (asked, key) {
        (AskedAlgorithm::Sha1, KeyKind::Rsa) => SignatureAlgorithm::Sha1Rsa,
        (AskedAlgorithm::Sha256, KeyKind::Rsa) => SignatureAlgorithm::Sha256Rsa,
        (AskedAlgorithm::Sha384, KeyKind::Rsa) => SignatureAlgorithm::Sha384Rsa,
        (AskedAlgorithm::Sha512, KeyKind::Rsa) => SignatureAlgorithm::Sha512Rsa,
        (AskedAlgorithm::Sha1, KeyKind::Ec) => SignatureAlgorithm::Sha1Ecdsa,
        (AskedAlgorithm::Sha256, KeyKind::Ec) => SignatureAlgorithm::Sha256Ecdsa,
        (AskedAlgorithm::Sha384, KeyKind::Ec) => SignatureAlgorithm::Sha384Ecdsa,
        (AskedAlgorithm::Sha512, KeyKind::Ec) => SignatureAlgorithm::Sha512Ecdsa,
    })
}

fn no_mechanism_for(algorithm: &str) -> TokenError {
    TokenError::new(
        Situation::MechanismNotOffered,
        format!("el token no firma con '{algorithm}': no esta en el catalogo del original"),
    )
}

fn refusal_of_token(error: TokenError) -> SigningRefusal {
    let code = code_of_token(error.situation());
    signing_refusal_of((Failure::from(error), code))
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

#[cfg(test)]
mod tests;
