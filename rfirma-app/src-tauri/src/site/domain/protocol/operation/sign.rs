//! Las peticiones de `sign`, `cosign` y `countersign`.

use super::super::algorithm::AskedAlgorithm;
use super::super::data_source::DataSource;
use super::super::filters::{site_filter, SiteFilter};
use super::super::format::{format_of, RequestedFormat};
use super::super::key_store::module_named_by;
use super::super::parameters::{sticky_certificate, StickyCertificate};
use super::super::refusal::Refusal;
use super::super::url::AfirmaUrl;
use super::document::{optional_document, read_document};
use super::guards::{
    check_algorithm, refuse_a_countersignature_outside_cades_and_xades,
    refuse_a_multisignature_of_an_invoice, requested_format, resolve_auto_format,
};
use super::properties::{
    comma_list_value, declared_properties, property_value, Unattended, FILENAME_CURRENT_DIR,
    FILENAME_DESCRIPTION, FILENAME_EXTS,
};
use super::SiteOperation;
use crate::site::domain::triphase_server::ServerFormat;

/// `extraParams`: a qué firmas alcanza la contrafirma.
const TARGET: &str = "target";

const TARGET_TREE: &str = "tree";
const TARGET_LEAFS: &str = "leafs";

/// A qué firmas de la que llega alcanza una contrafirma (`CounterSignTarget`, 1.9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterTarget {
    /// `tree`: se contrafirma el árbol entero.
    Tree,
    /// `leafs`: se contrafirman solo las hojas.
    Leafs,
}

impl CounterTarget {
    /// El árbol si ese `target=` es `tree`; las hojas si es cualquier otra cosa, como el original.
    pub fn named(text: &str) -> Self {
        if text.trim().eq_ignore_ascii_case(TARGET_TREE) {
            Self::Tree
        } else {
            Self::Leafs
        }
    }

    /// El valor de `target=` que lo nombra.
    pub fn name(self) -> &'static str {
        match self {
            Self::Tree => TARGET_TREE,
            Self::Leafs => TARGET_LEAFS,
        }
    }
}

/// Cuál de las tres firmas pidió la sede.
///
/// En PAdES las dos primeras recorren el mismo camino —cofirmar es volver a
/// firmar—, y la distinción se guarda porque es lo que la sede pidió y lo que
/// la ventana tiene que contarle a la persona antes de que consienta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureRound {
    /// `sign`: se firma lo que llega.
    First,
    /// `cosign`: se vuelve a firmar sobre las firmas que el documento ya trae.
    Again,
    /// `countersign`: se firman las firmas que trae el documento.
    Counter {
        /// A cuáles de ellas alcanza.
        target: CounterTarget,
    },
}

/// La petición de `sign` o de `cosign`.
///
/// Lleva **el documento ya descodificado**, y no el Base64: lo que se firma son
/// bytes, y dejar el Base64 vivo hasta el momento de firmar es tener dos
/// copias de lo mismo y una ocasión de descodificarlo dos veces distintas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRequest {
    round: SignatureRound,
    algorithm: AskedAlgorithm,
    format: RequestedFormat,
    document: Vec<u8>,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
    sticky: StickyCertificate,
    unattended: Unattended,
    through_the_site_server: Option<ServerFormat>,
}

impl SignRequest {
    /// `sign` o `cosign`.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// La huella que pidió la sede, ya admitida.
    pub fn algorithm(&self) -> AskedAlgorithm {
        self.algorithm
    }

    /// El formato efectivo: el que nombró la sede, o el del documento si pidió `auto`.
    pub fn format(&self) -> RequestedFormat {
        self.format
    }

    /// El documento que la sede manda, en bytes.
    pub fn document(&self) -> &[u8] {
        &self.document
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
}

/// La firma que la sede pidió sin `dat`: todo lo suyo menos el documento, que elige la persona
/// (`ProtocolInvocationLauncherSign.java:301-360`, 1.9.2).
///
/// No es un [`SignRequest`] con el documento vacío: sin documento no hay formato efectivo que
/// nombrar, y `format=auto` se resuelve sobre lo que la persona elija.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingSignRequest {
    round: SignatureRound,
    algorithm: AskedAlgorithm,
    requested: Option<RequestedFormat>,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
    sticky: StickyCertificate,
    unattended: Unattended,
    load_extensions: Vec<String>,
    load_description: Option<String>,
    load_starting_folder: Option<String>,
    load_filename: Option<String>,
    through_the_site_server: Option<ServerFormat>,
}

impl PendingSignRequest {
    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }

    /// Si la sede pidió `headless`: lo que haga falta preguntar se rechaza.
    pub fn is_headless(&self) -> bool {
        self.unattended.is_headless()
    }

    /// Si la sede se conforma con el único candidato que pase el filtro.
    pub fn waives_the_choice(&self) -> bool {
        self.unattended.waives_the_choice()
    }

    /// El nombre que la sede propone al selector (`filenameActualName`), si lo declaró.
    pub fn load_filename(&self) -> Option<&str> {
        self.load_filename.as_deref()
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

    /// La petición ya completa con el documento que la persona eligió, que con `format=auto` fija
    /// el formato efectivo igual que si hubiera llegado en `dat`.
    pub fn with_chosen_document(self, document: Vec<u8>) -> SignRequest {
        let format = self.requested.unwrap_or_else(|| format_of(&document));
        SignRequest {
            round: self.round,
            algorithm: self.algorithm,
            format,
            document,
            declared: self.declared,
            filter: self.filter,
            sticky: self.sticky,
            unattended: self.unattended,
            through_the_site_server: self.through_the_site_server,
        }
    }
}

/// La petición de firma, con las cuatro comprobaciones de
/// `UrlParametersToSign` que rFirma hereda.
///
/// El **formato** se mira antes que el algoritmo —de ahí la guarda de
/// contrafirma repetida sobre `requested`, que es lo único que compra—, y el
/// **objetivo** de la contrafirma antes que ambos, en `read_operation`, porque
/// sin ronda no hay petición que construir.
pub(super) fn sign_request(
    url: &AfirmaUrl,
    round: SignatureRound,
    data: &dyn DataSource,
) -> Result<SiteOperation, Refusal> {
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

    let properties = declared_properties(url);
    let declared = properties.crossing().to_vec();
    let through_the_site_server = url.parameter("format").and_then(ServerFormat::named);
    let sticky = sticky_certificate(url);
    if url.parameter("dat").is_none() {
        return Ok(SiteOperation::SignWithoutDocument(PendingSignRequest {
            round,
            algorithm,
            requested,
            filter: site_filter(&declared).within_the_module(module_named_by(url)),
            sticky,
            unattended: properties.unattended(),
            load_extensions: comma_list_value(property_value(&declared, FILENAME_EXTS)),
            load_description: property_value(&declared, FILENAME_DESCRIPTION),
            load_starting_folder: property_value(&declared, FILENAME_CURRENT_DIR),
            load_filename: properties.actual_name().map(str::to_owned),
            declared,
            through_the_site_server,
        }));
    }

    let document = match document {
        Some(document) => document,
        None => read_document(url, data)?,
    };

    let format = match requested {
        Some(format) => format,
        None => resolve_auto_format(&document, round)?,
    };
    refuse_a_multisignature_of_an_invoice(round, format)?;
    refuse_a_countersignature_outside_cades_and_xades(round, format)?;
    Ok(SiteOperation::Sign(SignRequest {
        round,
        algorithm,
        format,
        document,
        filter: site_filter(&declared).within_the_module(module_named_by(url)),
        sticky,
        unattended: properties.unattended(),
        declared,
        through_the_site_server,
    }))
}

/// La ronda de `countersign`, con el objetivo que declaró la sede o el `leafs` del original.
pub(super) fn counter_round(declared: &[(String, String)]) -> SignatureRound {
    let target = property_value(declared, TARGET).map_or(CounterTarget::Leafs, |declared| {
        CounterTarget::named(&declared)
    });
    SignatureRound::Counter { target }
}
