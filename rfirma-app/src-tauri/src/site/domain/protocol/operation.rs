//! Lo que la sede pide por el canal ya abierto, leído de la URL.

#[cfg(test)]
use base64::Engine as _;

#[cfg(test)]
use super::algorithm::AskedAlgorithm;
#[cfg(test)]
use super::codes::Parameter;
use super::codes::SafCode;
use super::data_source::DataSource;
use super::filters::{site_filter, SiteFilter};
#[cfg(test)]
use super::format::RequestedFormat;
use super::key_store::{module_named_by, refuse_a_key_store_rfirma_does_not_open};
use super::parameters::{
    check_common_parameters, check_minimum_client_version, check_operation_identifier,
    check_protocol_version_bounds, minimum_protocol_version, sticky_certificate, StickyCertificate,
};
use super::refusal::Refusal;
use super::url::AfirmaUrl;

mod batch;
mod document;
mod guards;
mod properties;
mod save_load;
mod sign;
mod sign_and_save;

use batch::batch_request;
use properties::{declared_properties, verb_of, Unattended};
use save_load::{load_request, save_request};
use sign::{counter_round, sign_request};
use sign_and_save::sign_and_save_request;

pub use batch::BatchRequest;
pub use guards::{
    refuse_a_countersignature_outside_cades_and_xades, refuse_a_multisignature_of_an_invoice,
    refuse_explicit_xades,
};
pub use properties::{pairs_of, without_the_launcher_keys};
pub use save_load::{LoadRequest, SaveRequest};
pub use sign::{CounterTarget, PendingSignRequest, SignRequest, SignatureRound};
pub use sign_and_save::SignAndSaveRequest;

/// El verbo de la selección de certificado, tal y como viaja por el cable.
///
/// **No es el nombre que el JS usa por dentro**: allí la constante se llama
/// `OPERATION_SELECT_CERTIFICATE = "certificate"` (`autoscript.js:1761`), que
/// es sólo la etiqueta con la que el cliente recuerda qué respuesta espera. Lo
/// que viaja es esto (`autoscript.js:1943`).
pub const SELECT_CERTIFICATE: &str = "selectcert";

/// El verbo de la firma (`autoscript.js:1828`).
pub const SIGN: &str = "sign";

/// El verbo de la cofirma.
pub const COSIGN: &str = "cosign";

/// El verbo de la contrafirma.
pub const COUNTERSIGN: &str = "countersign";

/// El verbo que guarda un fichero en el equipo.
pub const SAVE: &str = "save";

/// El verbo que carga uno o varios ficheros del equipo.
pub const LOAD: &str = "load";

/// El verbo que firma y además guarda.
pub const SIGN_AND_SAVE: &str = "signandsave";

/// El verbo del lote remoto.
pub const BATCH: &str = "batch";

/// `format=auto`: la sede no fija formato y pide que se deduzca del documento.
pub const AUTO: &str = "auto";

/// Lo que la sede pide, ya leído.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteOperation {
    /// `selectcert`: la sede pide identidad.
    SelectCertificate(SelectCertificate),
    /// `sign` o `cosign` sobre un PDF: la sede pide una firma.
    Sign(SignRequest),
    /// `save`: la sede pide guardar un fichero en el equipo.
    Save(SaveRequest),
    /// `load`: la sede pide cargar uno o varios ficheros del equipo.
    Load(LoadRequest),
    /// `signandsave`: la sede pide firmar y guardar el resultado.
    SignAndSave(SignAndSaveRequest),
    /// `batch`: la sede pide firmar un lote, remoto o local.
    Batch(BatchRequest),
    /// `sign`, `cosign` o `countersign` sin `dat`: el documento lo elige la persona.
    SignWithoutDocument(PendingSignRequest),
}

impl SiteOperation {
    /// Si la operación acaba en un certificado, que es lo que le da sentido a nombrar un almacén.
    pub fn chooses_a_certificate(&self) -> bool {
        !matches!(self, Self::Save(_) | Self::Load(_))
    }
}

/// La petición de `selectcert`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectCertificate {
    filter: SiteFilter,
    sticky: StickyCertificate,
    unattended: Unattended,
}

impl SelectCertificate {
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

const DISPATCHED_VERBS: [&str; 8] = [
    SELECT_CERTIFICATE,
    SIGN,
    COSIGN,
    COUNTERSIGN,
    SAVE,
    LOAD,
    BATCH,
    SIGN_AND_SAVE,
];

/// Lee la operación como `read_operation`, pero rechaza con `SAF_21` la que exige en `ver` un protocolo posterior.
pub fn read_operation_within_the_protocol(
    url: &AfirmaUrl,
    data: &dyn DataSource,
) -> Result<SiteOperation, Refusal> {
    check_the_parameters_the_original_parses(url)?;
    if DISPATCHED_VERBS.contains(&verb_of(url).as_str()) {
        check_protocol_version_bounds(minimum_protocol_version(url))?;
    }
    read_operation(url, data)
}

/// Lee la operación que llegó por el canal, o por qué se rechaza.
pub fn read_operation(url: &AfirmaUrl, data: &dyn DataSource) -> Result<SiteOperation, Refusal> {
    check_the_parameters_the_original_parses(url)?;
    check_minimum_client_version(url.parameter("mcv"))?;

    let asked = match verb_of(url).as_str() {
        SELECT_CERTIFICATE => {
            let declared = declared_properties(url);
            Ok(SiteOperation::SelectCertificate(SelectCertificate {
                filter: site_filter(declared.crossing()).within_the_module(module_named_by(url)),
                sticky: sticky_certificate(url),
                unattended: declared.unattended(),
            }))
        }
        SIGN => sign_request(url, SignatureRound::First, data),
        COSIGN => sign_request(url, SignatureRound::Again, data),
        COUNTERSIGN => sign_request(
            url,
            counter_round(declared_properties(url).crossing()),
            data,
        ),
        SAVE => save_request(url, data),
        LOAD => load_request(url),
        BATCH => batch_request(url, data),
        SIGN_AND_SAVE => sign_and_save_request(url, data),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("la operacion '{other}' no se atiende"),
        )),
    }?;

    if asked.chooses_a_certificate() {
        refuse_a_key_store_rfirma_does_not_open(url)?;
    }
    Ok(asked)
}

/// Las guardias comunes, solo en los verbos cuya URL analiza el original; el resto sale con `SAF_04` sin mirarla.
fn check_the_parameters_the_original_parses(url: &AfirmaUrl) -> Result<(), Refusal> {
    match url.verb() {
        LOAD => check_common_parameters(url),
        SIGN | COSIGN | COUNTERSIGN | SIGN_AND_SAVE | SELECT_CERTIFICATE | SAVE | BATCH => {
            check_common_parameters(url)?;
            check_operation_identifier(url)
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
