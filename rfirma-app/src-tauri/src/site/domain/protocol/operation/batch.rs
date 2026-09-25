//! La petición de `batch`, local o contra los dos servlets remotos.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use super::super::codes::Parameter;
use super::super::data_source::DataSource;
use super::super::filters::{site_filter, SiteFilter};
use super::super::key_store::module_named_by;
use super::super::parameters::{check_servlet_url, sticky_certificate, StickyCertificate};
use super::super::refusal::Refusal;
use super::super::url::AfirmaUrl;
use super::document::{data_of, is_base64_to_the_original, is_gzip};
use super::properties::{declared_properties, required, Unattended};
use super::SiteOperation;

/// `localBatchProcess=true`: el lote se firma aquí y no contra los dos servlets.
const LOCAL_BATCH_PROCESS: &str = "localBatchProcess";

/// La petición de `batch`: firmar un lote contra los dos servlets, o aquí
/// mismo cuando la sede pide `localBatchProcess=true` (`BatchSigner`,
/// `LocalBatchSigner`, 1.9.2).
///
/// Lleva el lote **tal y como llegó**: los bytes decodificados, que se leen al
/// firmar y no aquí, y el Base64 original intacto, porque lo que
/// viaja a los servlets es una sustitución textual sobre ese Base64 y no una
/// recodificación de los bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRequest {
    lote: Vec<u8>,
    lote_base64: String,
    json: bool,
    local: bool,
    presigner_url: Option<String>,
    postsigner_url: Option<String>,
    needcert: bool,
    filter: SiteFilter,
    sticky: StickyCertificate,
    unattended: Unattended,
}

impl BatchRequest {
    /// El lote decodificado, en bytes.
    pub fn lote(&self) -> &[u8] {
        &self.lote
    }

    /// El lote tal y como llegó en `dat`, todavía en Base64.
    pub fn lote_base64(&self) -> &str {
        &self.lote_base64
    }

    /// Si el lote viene en JSON (`jsonbatch=true`) o en el XML heredado.
    pub fn is_json(&self) -> bool {
        self.json
    }

    /// Si el lote se firma aquí (`localBatchProcess=true`) o contra los dos servlets.
    pub fn is_local(&self) -> bool {
        self.local
    }

    /// La URL del servlet de prefirma, que el lote local no lleva.
    pub fn presigner_url(&self) -> Option<&str> {
        self.presigner_url.as_deref()
    }

    /// La URL del servlet de postfirma, que el lote local no lleva.
    pub fn postsigner_url(&self) -> Option<&str> {
        self.postsigner_url.as_deref()
    }

    /// Si la sede pide el certificado usado además del resultado del lote.
    pub fn needcert(&self) -> bool {
        self.needcert
    }

    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }

    /// Lo que la sede pide sobre el certificado pegado.
    pub fn sticky(&self) -> StickyCertificate {
        self.sticky
    }

    /// Si la sede pidió `headless`: lo que haga falta preguntar se rechaza.
    pub fn is_headless(&self) -> bool {
        self.unattended.is_headless()
    }

    /// Si la sede se conforma con el único candidato que pase el filtro.
    pub fn waives_the_choice(&self) -> bool {
        self.unattended.waives_the_choice()
    }
}

/// La petición del lote: las dos URL de servlet del remoto y el lote sin leer
/// (`ProtocolInvocationLauncherBatch`, 1.9.2).
pub(super) fn batch_request(
    url: &AfirmaUrl,
    data: &dyn DataSource,
) -> Result<SiteOperation, Refusal> {
    let local = url
        .parameter(LOCAL_BATCH_PROCESS)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    let json = url
        .parameter("jsonbatch")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    if local && !json {
        return Err(Refusal::about(
            Parameter::Data,
            "el lote local solo existe en JSON: el XML heredado va a los dos servlets",
        ));
    }

    let servlets = match local {
        true => None,
        false => Some(batch_servlets(url)?),
    };

    let value = required(url, "dat", Parameter::Data)?;
    let lote = data_of(url, data)?;
    let lote_base64 = match is_gzip(url) || !is_base64_to_the_original(value) {
        true => STANDARD.encode(&lote),
        false => value.to_owned(),
    };

    let (presigner_url, postsigner_url) = match servlets {
        Some((presigner_url, postsigner_url)) => (Some(presigner_url), Some(postsigner_url)),
        None => (None, None),
    };
    let declared = declared_properties(url);
    Ok(SiteOperation::Batch(BatchRequest {
        lote,
        lote_base64,
        json,
        local,
        presigner_url,
        postsigner_url,
        needcert: url
            .parameter("needcert")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
        filter: site_filter(declared.crossing()).within_the_module(module_named_by(url)),
        sticky: sticky_certificate(url),
        unattended: declared.unattended(),
    }))
}

/// Las dos URL de servlet que el lote remoto exige, ya comprobadas.
fn batch_servlets(url: &AfirmaUrl) -> Result<(String, String), Refusal> {
    let presigner_url = required(url, "batchpresignerurl", Parameter::BatchPresignerUrl)?;
    check_servlet_url(presigner_url, Parameter::BatchPresignerUrl)?;
    let postsigner_url = required(url, "batchpostsignerurl", Parameter::BatchPostsignerUrl)?;
    check_servlet_url(postsigner_url, Parameter::BatchPostsignerUrl)?;

    Ok((presigner_url.to_owned(), postsigner_url.to_owned()))
}
