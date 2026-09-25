//! La petición de `signandsave`: firmar y guardar el resultado.

use super::super::algorithm::AskedAlgorithm;
use super::super::codes::SafCode;
use super::super::data_source::DataSource;
use super::super::filters::{site_filter, SiteFilter};
use super::super::format::{format_of, RequestedFormat};
use super::super::key_store::module_named_by;
use super::super::parameters::{sticky_certificate, StickyCertificate};
use super::super::refusal::Refusal;
use super::super::url::AfirmaUrl;
use super::document::optional_document;
use super::guards::{
    check_algorithm, refuse_a_countersignature_outside_cades_and_xades,
    refuse_a_multisignature_of_an_invoice, requested_format, resolve_auto_format,
};
use super::properties::{
    comma_list_value, declared_properties, optional, property_value, Unattended,
    FILENAME_CURRENT_DIR, FILENAME_DESCRIPTION, FILENAME_EXTS, FILENAME_SAVE_CURRENT_DIR,
    FILENAME_SAVE_DESCRIPTION, FILENAME_SAVE_EXTS,
};
use super::save_load::check_filename;
use super::sign::{counter_round, SignatureRound};
use super::{SiteOperation, COSIGN, COUNTERSIGN, SIGN};
use crate::site::domain::triphase_server::ServerFormat;

/// `ProtocolLauncher.30`: el nombre por defecto cuando la sede no propone ninguno.
const DEFAULT_SIGNED_NAME: &str = "Firma";

/// La petición de `signandsave`: firmar y guardar el resultado
/// (`UrlParametersToSignAndSave`, 1.9.2).
///
/// No envuelve un [`super::sign::SignRequest`]: un `SignRequest` sin documento sería un
/// estado inválido representable, y aquí `dat` es opcional (la sede puede
/// dejar el documento por elegir). La lectura de formato, algoritmo y
/// documento se comparte con `sign_request`, no se copia.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignAndSaveRequest {
    round: SignatureRound,
    algorithm: AskedAlgorithm,
    document: Option<Vec<u8>>,
    requested: Option<RequestedFormat>,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
    sticky: StickyCertificate,
    unattended: Unattended,
    filename: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
    starting_folder: Option<String>,
    load_extensions: Vec<String>,
    load_description: Option<String>,
    load_starting_folder: Option<String>,
    load_filename: Option<String>,
    chosen_name: Option<String>,
    through_the_site_server: Option<ServerFormat>,
}

impl SignAndSaveRequest {
    /// `sign` o `cosign`, según pida `cop`.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// La huella que pidió la sede, ya admitida.
    pub fn algorithm(&self) -> AskedAlgorithm {
        self.algorithm
    }

    /// El documento que la sede manda, si vino: sin `dat` queda por elegir.
    pub fn document(&self) -> Option<&[u8]> {
        self.document.as_deref()
    }

    /// El formato efectivo: el que nombró la sede, o el del documento que ya tenga.
    pub fn format(&self) -> RequestedFormat {
        self.requested
            .unwrap_or_else(|| format_of(self.document.as_deref().unwrap_or_default()))
    }

    /// Los `extraParams` tal y como vinieron, sin expandir.
    pub fn declared_params(&self) -> &[(String, String)] {
        &self.declared
    }

    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }

    /// Lo que la sede pide sobre el certificado pegado.
    pub fn sticky(&self) -> StickyCertificate {
        self.sticky
    }

    /// Nombre de fichero que propone la sede.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Extensiones admitidas por el filtro de guardado (`filenameSaveExts`).
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Descripción del filtro de extensiones (`filenameSaveDescription`), si la sede la declaró.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Carpeta inicial sugerida (`filenameSaveCurrentDir`), nunca la única fuente de lectura.
    pub fn starting_folder(&self) -> Option<&str> {
        self.starting_folder.as_deref()
    }

    /// El nombre propuesto al diálogo de guardado: el de la sede, o el del fichero con la extensión de su formato.
    pub fn proposed_name(&self) -> String {
        self.filename.clone().unwrap_or_else(|| {
            let ext = self.format().extension();
            self.chosen_name
                .as_deref()
                .map(|name| format!("{}.{ext}", base_name(name)))
                .unwrap_or_else(|| format!("{DEFAULT_SIGNED_NAME}.{ext}"))
        })
    }

    /// Extensiones admitidas por el selector que elige el documento (`filenameExts`).
    pub fn load_extensions(&self) -> &[String] {
        &self.load_extensions
    }

    /// Descripción del filtro de extensiones del selector (`filenameDescription`), si la sede la declaró.
    pub fn load_description(&self) -> Option<&str> {
        self.load_description.as_deref()
    }

    /// Carpeta inicial sugerida al selector (`filenameCurrentDir`), nunca la única fuente de lectura.
    pub fn load_starting_folder(&self) -> Option<&str> {
        self.load_starting_folder.as_deref()
    }

    /// El nombre que la sede propone al selector (`filenameActualName`), si lo declaró.
    pub fn load_filename(&self) -> Option<&str> {
        self.load_filename.as_deref()
    }

    /// Si la sede pidió `headless`: lo que haga falta preguntar se rechaza.
    pub fn is_headless(&self) -> bool {
        self.unattended.is_headless()
    }

    /// Si la sede se conforma con el único candidato que pase el filtro.
    pub fn waives_the_choice(&self) -> bool {
        self.unattended.waives_the_choice()
    }

    /// El firmador del servidor trifásico de la sede, si la prefirma y la postfirma se hacen allí.
    pub fn through_the_site_server(&self) -> Option<ServerFormat> {
        self.through_the_site_server
    }

    /// El documento que la persona acaba de elegir, que con `format=auto` fija el formato
    /// efectivo igual que si hubiera llegado en `dat`. El nombre del fichero elegido (con su
    /// extensión) alimenta el segundo escalón de `proposed_name`.
    pub fn with_chosen_document(&self, document: Vec<u8>, chosen_name: Option<String>) -> Self {
        Self {
            document: Some(document),
            chosen_name,
            ..self.clone()
        }
    }
}

/// La petición de `signandsave`: misma lectura y mismos rechazos que `sign`,
/// con `dat` opcional y lo del guardado (`ProtocolInvocationLauncherSignAndSave`, 1.9.2).
pub(super) fn sign_and_save_request(
    url: &AfirmaUrl,
    data: &dyn DataSource,
) -> Result<SiteOperation, Refusal> {
    let filename = optional(url, "filename");
    if let Some(filename) = &filename {
        check_filename(filename)?;
    }
    let properties = declared_properties(url);
    let declared = properties.crossing().to_vec();
    let round = round_of_cop(url, &declared)?;

    let requested = requested_format(url)?;
    if let Some(format) = requested {
        refuse_a_multisignature_of_an_invoice(round, format)?;
        refuse_a_countersignature_outside_cades_and_xades(round, format)?;
    }
    let document = match requested {
        Some(_) => None,
        None => optional_document(url, data)?,
    };

    let algorithm = check_algorithm(url)?;

    let document = match document {
        Some(document) => Some(document),
        None => optional_document(url, data)?,
    };

    if requested.is_none() {
        if let Some(doc) = &document {
            let format = resolve_auto_format(doc, round)?;
            refuse_a_multisignature_of_an_invoice(round, format)?;
            refuse_a_countersignature_outside_cades_and_xades(round, format)?;
        }
    }

    Ok(SiteOperation::SignAndSave(SignAndSaveRequest {
        round,
        algorithm,
        document,
        requested,
        filter: site_filter(&declared).within_the_module(module_named_by(url)),
        sticky: sticky_certificate(url),
        unattended: properties.unattended(),
        filename,
        extensions: comma_list_value(property_value(&declared, FILENAME_SAVE_EXTS)),
        description: property_value(&declared, FILENAME_SAVE_DESCRIPTION),
        starting_folder: property_value(&declared, FILENAME_SAVE_CURRENT_DIR),
        load_extensions: comma_list_value(property_value(&declared, FILENAME_EXTS)),
        load_description: property_value(&declared, FILENAME_DESCRIPTION),
        load_starting_folder: property_value(&declared, FILENAME_CURRENT_DIR),
        load_filename: properties.actual_name().map(str::to_owned),
        chosen_name: None,
        through_the_site_server: url.parameter("format").and_then(ServerFormat::named),
        declared,
    }))
}

/// La ronda que pide `cop`, o el `SAF_04` que la nombra.
fn round_of_cop(url: &AfirmaUrl, declared: &[(String, String)]) -> Result<SignatureRound, Refusal> {
    let cop = url
        .parameter("cop")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match cop.as_str() {
        SIGN => Ok(SignatureRound::First),
        COSIGN => Ok(SignatureRound::Again),
        COUNTERSIGN => Ok(counter_round(declared)),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!(
                "el 'cop' de 'signandsave' no admite '{other}': solo 'sign', 'cosign' o \
                 'countersign'"
            ),
        )
        .found_while_processing()),
    }
}

/// El nombre de fichero elegido sin su extensión (`AOPDFSigner.getSignedName`, 1.9.2), o el
/// nombre entero si no tiene punto.
fn base_name(chosen_name: &str) -> &str {
    chosen_name
        .rsplit_once('.')
        .map_or(chosen_name, |(base, _)| base)
}
