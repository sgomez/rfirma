//! La firma trifásica ya consentida contra el servidor de la sede: prefirma allí, `PK1` con el token aquí y postfirma allí; no pasa por el puente (ADR-0001).

use std::collections::BTreeMap;

use crate::identity::domain::algorithm::KeyKind;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::{Situation as TokenSituation, TokenError};
use crate::signing::domain::properties::to_java_properties;
use crate::site::application::session::SiteRefusal;
use crate::site::domain::batch::{apply_pk1, TriphaseData};
use crate::site::domain::protocol::{AskedAlgorithm, SignatureRound};
use crate::site::domain::signing::SigningRefusal;
use crate::site::domain::triphase_server::{
    params_for_the_server, postsign_form, postsigned, presign_form, presigned, server_url_of,
    ServerCall, ServerFormat, Situation, TriphaseServerError,
};
use crate::site::ports::{TokenSigning, TriphaseServer};

/// Quién interviene en la firma: el servidor de la sede, el token y el secreto ya abierto.
pub struct ServerRun<'a> {
    /// El servidor trifásico.
    pub server: &'a dyn TriphaseServer,
    /// Quien firma con el token, sin que la clave salga de él (ADR-0001).
    pub token: &'a dyn TokenSigning,
    /// El certificado que la persona consintió.
    pub certificate: &'a TokenCertificate,
    /// El secreto ya abierto.
    pub secret: &'a str,
}

/// Lo que la sede pidió firmar.
pub struct ServerAsk<'a> {
    /// El firmador trifásico que eligió la sede.
    pub format: ServerFormat,
    /// La operación que pidió la sede.
    pub round: SignatureRound,
    /// La huella que pidió la sede.
    pub algorithm: AskedAlgorithm,
    /// Los datos, o la firma previa en cofirma y contrafirma.
    pub document: &'a [u8],
    /// Los parámetros de la sede, ya expandidos, con el `serverUrl` entre ellos.
    pub from_the_site: &'a BTreeMap<String, String>,
}

/// Caso de uso: prefirma en el servidor, firma cada `PRE` con el token y devuelve la firma que entrega la postfirma.
pub fn signed_through_the_server(
    run: &ServerRun<'_>,
    ask: &ServerAsk<'_>,
) -> Result<Vec<u8>, SiteRefusal> {
    let server_url = server_url_of(ask.from_the_site).map_err(SiteRefusal::Triphase)?;
    let params = params_for_the_server(ask.from_the_site, ask.format, ask.round);
    let params = (!params.is_empty()).then(|| to_java_properties(&params));
    let algorithm = composed_name(ask.algorithm, run.certificate.key_kind())?;
    let call = ServerCall {
        format: ask.format,
        round: ask.round,
        algorithm: &algorithm,
        certificate: run.certificate.der(),
        document: ask.document,
        params: params.as_deref(),
    };

    let answer = run
        .server
        .post(server_url, &presign_form(&call))
        .map_err(SiteRefusal::Triphase)?;
    let session = presigned(&answer).map_err(SiteRefusal::Triphase)?;
    let signed = every_pre_signed(run, ask.algorithm, session)?;
    let answer = run
        .server
        .post(server_url, &postsign_form(&call, &signed))
        .map_err(SiteRefusal::Triphase)?;
    postsigned(&answer).map_err(SiteRefusal::Triphase)
}

fn every_pre_signed(
    run: &ServerRun<'_>,
    algorithm: AskedAlgorithm,
    session: TriphaseData,
) -> Result<TriphaseData, SiteRefusal> {
    let mut refused: Option<SigningRefusal> = None;
    let signed = apply_pk1(session, |pre| {
        if refused.is_some() {
            return Vec::new();
        }
        run.token
            .sign(run.certificate, run.secret, algorithm.name(), pre)
            .unwrap_or_else(|refusal| {
                refused = Some(refusal);
                Vec::new()
            })
    })
    .map_err(|error| {
        SiteRefusal::Triphase(TriphaseServerError::new(
            Situation::UnexpectedAnswer,
            error.to_string(),
        ))
    })?;

    match refused {
        Some(refusal) => Err(SiteRefusal::Signing(refusal)),
        None => Ok(signed),
    }
}

fn composed_name(asked: AskedAlgorithm, key: Option<KeyKind>) -> Result<String, SiteRefusal> {
    let with = match key {
        Some(KeyKind::Ec) => "ECDSA",
        Some(KeyKind::Rsa) => "RSA",
        None => {
            return Err(SiteRefusal::Token(TokenError::new(
                TokenSituation::KeyNotRsa,
                "la clave del certificado no es RSA ni de curva eliptica",
            )))
        }
    };
    Ok(format!("{}with{with}", asked.name()))
}

#[cfg(test)]
mod tests;
