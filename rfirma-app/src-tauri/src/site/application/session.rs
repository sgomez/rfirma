//! Sesión de firma de un trámite de sede: prefirma filtrada y postfirma en memoria (ADR-0001, ADR-0016).

use std::collections::BTreeMap;

use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::{BridgeError, Format, SignatureOperation};
use crate::signing::domain::Refusal as Inadmissible;
use crate::site::application::filtering;
use crate::site::domain::batch_error::BatchError;
use crate::site::domain::protocol::{AskedAlgorithm, SiteFilter};
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::domain::triphase_server::TriphaseServerError;
use crate::site::ports::{FilterEngine, Neighbours, SiteSigningRequest};

/// Por qué el trámite no sigue, antes de traducirlo a la ventana y al cable.
#[derive(Debug)]
pub enum SiteRefusal {
    /// El token no ha dejado listar los certificados.
    Token(TokenError),
    /// El documento que manda la sede no se puede firmar.
    Inadmissible(Inadmissible),
    /// Las políticas de la sede no se han podido expandir.
    Policies(BridgeError),
    /// El filtro de la sede no se ha podido aplicar al listado.
    CouldNotFilter(filtering::FilteringError),
    /// La sede excluye todos los certificados que hay.
    NoCertificateTheSiteAccepts,
    /// El certificado elegido ya no vale para la sede.
    NotUsableForTheSite(filtering::FilteringError),
    /// La carpeta de paso del documento no se ha podido crear.
    ScratchFolderMissing(String),
    /// El documento de paso no se ha podido escribir.
    ScratchUnwritable(String),
    /// Uno de los ficheros elegidos por la persona para cargar no se ha podido leer.
    CannotLoadData(String),
    /// El documento que eligió la persona para firmar no se ha podido leer.
    CannotReadData(String),
    /// La firma no ha salido, y quien la hizo ya dijo con qué código y con qué vista.
    Signing(SigningRefusal),
    /// El lote remoto no se ha podido completar: alcance de los servlets o forma de su respuesta.
    Batch(BatchError),
    /// La firma del `PRE` de una firma del lote remoto ha fallado.
    BatchSigningFailed(SigningRefusal),
    /// El lote local, o uno de sus elementos, no se atiende.
    LocalBatch(String),
    /// El documento trae una firma que el validador del original no da por buena.
    InvalidSignature(String),
    /// Seguir necesita que la persona confirme, y la sede pidió `headless`.
    ConfirmationNeeded(String),
    /// Las firmas del documento no se han podido examinar.
    CouldNotValidate(BridgeError),
    /// La firma contra el servidor trifásico de la sede no ha salido.
    Triphase(TriphaseServerError),
}

impl From<SigningRefusal> for SiteRefusal {
    fn from(refusal: SigningRefusal) -> Self {
        Self::Signing(refusal)
    }
}

impl SiteRefusal {
    /// El texto de este rechazo, sin el código con el que la sede lo recibe.
    pub fn description(&self) -> String {
        match self {
            Self::Token(error) => error.to_string(),
            Self::Inadmissible(refusal) => refusal.to_string(),
            Self::Policies(error) | Self::CouldNotValidate(error) => error.to_string(),
            Self::CouldNotFilter(_) => "el filtro de la sede no se ha podido aplicar".to_owned(),
            Self::NoCertificateTheSiteAccepts => {
                "la sede excluye todos los certificados que hay".to_owned()
            }
            Self::NotUsableForTheSite(_) => "el certificado elegido ya no vale".to_owned(),
            Self::ScratchFolderMissing(detail)
            | Self::ScratchUnwritable(detail)
            | Self::CannotLoadData(detail)
            | Self::CannotReadData(detail)
            | Self::LocalBatch(detail)
            | Self::InvalidSignature(detail)
            | Self::ConfirmationNeeded(detail) => detail.clone(),
            Self::Signing(refusal) | Self::BatchSigningFailed(refusal) => refusal.detail.clone(),
            Self::Batch(error) => error.detail().to_owned(),
            Self::Triphase(error) => error.detail().to_owned(),
        }
    }
}

/// Lo que la sede declaró para esta firma: su filtro y sus parámetros ya expandidos.
pub struct SiteTerms<'a, E: FilterEngine> {
    /// Motor de filtros sobre certificados.
    pub engine: &'a E,
    /// Filtro de certificados declarado por la sede.
    pub filter: &'a SiteFilter,
    /// Formato de firma que pidió la sede, ya atendido por el puente.
    pub format: Format,
    /// Huella que pidió la sede para esta firma.
    pub algorithm: AskedAlgorithm,
    /// Qué pidió hacer la sede con el documento.
    pub operation: SignatureOperation,
    /// Parámetros adicionales declarados por la sede.
    pub from_the_site: &'a BTreeMap<String, String>,
    /// Si la sede consintió cofirmar sobre firmas que no se reconocen.
    pub allow_unregistered_signatures: bool,
}

/// Prefirma de un trámite de sede: vuelve a pasar el filtro de la sede antes de pedir el secreto.
pub fn begin_for_the_site<E: FilterEngine>(
    terms: &SiteTerms<'_, E>,
    document: &str,
    certificate: &str,
    neighbours: &dyn Neighbours,
) -> Result<StoreSecret, SiteRefusal> {
    let found = neighbours.listed().map_err(SiteRefusal::Token)?;
    let chosen = filtering::usable_certificate_for_the_site(
        terms.engine,
        terms.filter,
        &found,
        certificate,
        neighbours,
    )
    .map_err(SiteRefusal::NotUsableForTheSite)?;
    Ok(neighbours.begin(SiteSigningRequest {
        document,
        certificate: chosen,
        format: terms.format,
        algorithm: terms.algorithm,
        operation: terms.operation,
        from_the_site: terms.from_the_site,
        allow_unregistered_signatures: terms.allow_unregistered_signatures,
    })?)
}

/// Postfirma de un trámite de sede: la firma vuelve en memoria y no se escribe nada (ADR-0011).
pub fn finish_for_the_site(neighbours: &dyn Neighbours) -> Result<SiteSignature, SiteRefusal> {
    Ok(neighbours.finish()?)
}

#[cfg(test)]
mod tests;
