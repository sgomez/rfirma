//! Lo que la sede pide por el canal ya abierto, leído de la URL.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use super::algorithm::AskedAlgorithm;
use super::codes::{Parameter, SafCode};
use super::data_source::{download_url, DataSource};
use super::filters::{site_filter, SiteFilter};
use super::format::{format_of, RequestedFormat};
use super::parameters::{
    check_local_access_is_not_requested, check_minimum_client_version,
    check_minimum_protocol_version, check_servlet_url, minimum_protocol_version,
    sticky_certificate, StickyCertificate,
};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

const FORBIDDEN_IN_A_FILENAME: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

const FORBIDDEN_IN_AN_EXTENSION: [char; 11] =
    ['\\', '/', ':', '*', '?', '"', '<', '>', '|', ';', ' '];

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

/// `extraParams`: a qué firmas alcanza la contrafirma.
const TARGET: &str = "target";

const TARGET_TREE: &str = "tree";
const TARGET_LEAFS: &str = "leafs";

/// `localBatchProcess=true`: el lote se firma aquí y no contra los dos servlets.
const LOCAL_BATCH_PROCESS: &str = "localBatchProcess";

/// `properties`: extensiones admitidas por el diálogo de guardado de `signandsave`.
const FILENAME_SAVE_EXTS: &str = "filenameSaveExts";

/// `properties`: descripción del filtro de extensiones del diálogo de guardado.
const FILENAME_SAVE_DESCRIPTION: &str = "filenameSaveDescription";

/// `properties`: carpeta inicial sugerida al diálogo de guardado.
const FILENAME_SAVE_CURRENT_DIR: &str = "filenameSaveCurrentDir";

/// `properties`: extensiones admitidas por el selector que elige el documento a firmar
/// (`AfirmaExtraParams.LOAD_FILE_EXTS`).
const FILENAME_EXTS: &str = "filenameExts";

/// `properties`: descripción del filtro de extensiones del selector (`LOAD_FILE_DESCRIPTION`).
const FILENAME_DESCRIPTION: &str = "filenameDescription";

/// `properties`: carpeta inicial sugerida al selector (`LOAD_FILE_CURRENT_DIR`).
const FILENAME_CURRENT_DIR: &str = "filenameCurrentDir";

/// `properties`: el nombre que la sede propone para el fichero que se va a elegir.
const FILENAME_ACTUAL_NAME: &str = "filenameActualName";

/// `properties`: la sede se conforma con el único certificado que pase el filtro.
const HEADLESS: &str = "headless";

/// `properties`: puesto a `false` dice lo mismo que `headless=true`
/// (`CertFilterManager.isMandatoryCertificate`, 1.9.2).
const MANDATORY_CERT_SELECTION: &str = "mandatoryCertSelection";

/// `properties`: el perfil *baseline*, que el original borra antes de firmar.
const PROFILE: &str = "profile";

/// Las claves de `properties` que el lanzador interpreta él mismo y nunca entrega al firmador
/// (`ProtocolInvocationLauncherSign.java:153`, `CertFilterManager.java:145`, 1.9.2).
const INTERPRETED_BY_THE_LAUNCHER: [&str; 4] = [
    HEADLESS,
    MANDATORY_CERT_SELECTION,
    PROFILE,
    FILENAME_ACTUAL_NAME,
];

/// `ProtocolLauncher.30`: el nombre por defecto cuando la sede no propone ninguno.
const DEFAULT_SIGNED_NAME: &str = "Firma";

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

/// A qué firmas de la que llega alcanza una contrafirma (`CounterSignTarget`, 1.9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterTarget {
    /// `tree`: se contrafirma el árbol entero.
    Tree,
    /// `leafs`: se contrafirman solo las hojas.
    Leafs,
}

impl CounterTarget {
    /// El objetivo que nombra ese `target=`, o nada si no es ninguno de los dos.
    pub fn named(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            TARGET_TREE => Some(Self::Tree),
            TARGET_LEAFS => Some(Self::Leafs),
            _ => None,
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
    headless: bool,
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

    /// Si la sede se conforma con el único certificado que pase el filtro (`headless`).
    pub fn is_headless(&self) -> bool {
        self.headless
    }
}

/// La petición de `selectcert`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectCertificate {
    filter: SiteFilter,
    sticky: StickyCertificate,
    headless: bool,
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

    /// Si la sede se conforma con el único certificado que pase el filtro (`headless`).
    pub fn is_headless(&self) -> bool {
        self.headless
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
    headless: bool,
    load_extensions: Vec<String>,
    load_description: Option<String>,
    load_starting_folder: Option<String>,
    load_filename: Option<String>,
}

impl PendingSignRequest {
    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }

    /// Si la sede se conforma con el único certificado que pase el filtro (`headless`).
    pub fn is_headless(&self) -> bool {
        self.headless
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
            headless: self.headless,
        }
    }
}

/// La petición de `save`: guardar un fichero en el equipo (`UrlParametersToSave`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    data: Vec<u8>,
    title: Option<String>,
    filename: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
}

impl SaveRequest {
    /// El fichero que la sede pide guardar, en bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Título del diálogo de guardado, si la sede lo declaró.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Nombre de fichero que propone la sede.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Extensiones admitidas por el filtro del diálogo.
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Descripción del filtro de extensiones, si la sede la declaró.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// La petición de `signandsave`: firmar y guardar el resultado
/// (`UrlParametersToSignAndSave`, 1.9.2).
///
/// No envuelve un [`SignRequest`]: un `SignRequest` sin documento sería un
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
    headless: bool,
    filename: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
    starting_folder: Option<String>,
    load_extensions: Vec<String>,
    load_description: Option<String>,
    load_starting_folder: Option<String>,
    load_filename: Option<String>,
    chosen_name: Option<String>,
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

    /// El nombre propuesto al diálogo de guardado (`AOPDFSigner.getSignedName`, 1.9.2): el
    /// `filename` de la sede si vino; si no, el nombre base del fichero elegido más `.pdf`; y
    /// solo sin ninguno de los dos, `Firma.pdf`.
    pub fn proposed_name(&self) -> String {
        self.filename.clone().unwrap_or_else(|| {
            self.chosen_name
                .as_deref()
                .map(|name| format!("{}.pdf", base_name(name)))
                .unwrap_or_else(|| format!("{DEFAULT_SIGNED_NAME}.pdf"))
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

    /// Si la sede se conforma con el único certificado que pase el filtro (`headless`).
    pub fn is_headless(&self) -> bool {
        self.headless
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

/// La petición de `load`: cargar uno o varios ficheros del equipo (`UrlParametersToLoad`, 1.9.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadRequest {
    title: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
    starting_folder: Option<String>,
    multiple: bool,
}

impl LoadRequest {
    /// Título del selector, si la sede lo declaró.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Extensiones admitidas por el filtro del selector.
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Descripción del filtro de extensiones, si la sede la declaró.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Carpeta inicial sugerida por la sede, nunca la única fuente de lectura.
    pub fn starting_folder(&self) -> Option<&str> {
        self.starting_folder.as_deref()
    }

    /// Si la sede pide varios ficheros (`multiload=true`) o uno solo.
    pub fn multiple(&self) -> bool {
        self.multiple
    }
}

/// La petición de `batch`: firmar un lote contra los dos servlets, o aquí
/// mismo cuando la sede pide `localBatchProcess=true` (`BatchSigner`,
/// `LocalBatchSigner`, 1.9.2).
///
/// Lleva el lote **tal y como llegó**: los bytes decodificados para leer lo
/// mínimo que hace falta aquí, y el Base64 original intacto, porque lo que
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
    headless: bool,
    algorithm: String,
    stop_on_error: bool,
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

    /// Si la sede se conforma con el único certificado que pase el filtro (`headless`).
    pub fn is_headless(&self) -> bool {
        self.headless
    }

    /// El algoritmo del lote, ya admitido (fija el algoritmo del PKCS#1).
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Si el lote para en el primer error (aquí solo se lee y se guarda).
    pub fn stops_on_error(&self) -> bool {
        self.stop_on_error
    }
}

/// Lee la operación que llegó por el canal, o por qué se rechaza.
pub fn read_operation(url: &AfirmaUrl, data: &dyn DataSource) -> Result<SiteOperation, Refusal> {
    check_minimum_client_version(url.parameter("mcv"))?;
    check_minimum_protocol_version(minimum_protocol_version(url))?;
    if let Some(data) = url.parameter("dat") {
        check_local_access_is_not_requested(data)?;
    }

    match verb_of(url).as_str() {
        SELECT_CERTIFICATE => {
            let declared = declared_properties(url);
            Ok(SiteOperation::SelectCertificate(SelectCertificate {
                filter: site_filter(declared.crossing()),
                sticky: sticky_certificate(url),
                headless: declared.is_headless(),
            }))
        }
        SIGN => sign_request(url, SignatureRound::First, data),
        COSIGN => sign_request(url, SignatureRound::Again, data),
        COUNTERSIGN => sign_request(
            url,
            counter_round(declared_properties(url).crossing())?,
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
    }
}

/// La petición de firma, con las cuatro comprobaciones de
/// `UrlParametersToSign` que rFirma hereda.
///
/// El **formato** se mira antes que el algoritmo —de ahí la guarda de
/// contrafirma repetida sobre `requested`, que es lo único que compra—, y el
/// **objetivo** de la contrafirma antes que ambos, en `read_operation`, porque
/// sin ronda no hay petición que construir.
fn sign_request(
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
    if url.parameter("dat").is_none() {
        return Ok(SiteOperation::SignWithoutDocument(PendingSignRequest {
            round,
            algorithm,
            requested,
            filter: site_filter(&declared),
            headless: properties.is_headless(),
            load_extensions: comma_list_value(property_value(&declared, FILENAME_EXTS)),
            load_description: property_value(&declared, FILENAME_DESCRIPTION),
            load_starting_folder: property_value(&declared, FILENAME_CURRENT_DIR),
            load_filename: properties.actual_name().map(str::to_owned),
            declared,
        }));
    }

    let document = match document {
        Some(document) => document,
        None => read_document(url, data)?,
    };

    let format = requested.unwrap_or_else(|| format_of(&document));
    refuse_a_multisignature_of_an_invoice(round, format)?;
    refuse_a_countersignature_outside_cades_and_xades(round, format)?;
    Ok(SiteOperation::Sign(SignRequest {
        round,
        algorithm,
        format,
        document,
        filter: site_filter(&declared),
        headless: properties.is_headless(),
        declared,
    }))
}

/// La ronda de `countersign`, con el objetivo que declaró la sede o el `leafs` del original.
fn counter_round(declared: &[(String, String)]) -> Result<SignatureRound, Refusal> {
    let Some(declared) = property_value(declared, TARGET) else {
        return Ok(SignatureRound::Counter {
            target: CounterTarget::Leafs,
        });
    };
    CounterTarget::named(&declared)
        .map(|target| SignatureRound::Counter { target })
        .ok_or_else(|| {
            Refusal::about(
                Parameter::Properties,
                format!(
                    "el objetivo de contrafirma '{declared}' no se atiende: solo 'tree' o 'leafs'"
                ),
            )
        })
}

/// Una factura ni se cofirma ni se contrafirma: `AOFacturaESigner` lanza una
/// `UnsupportedOperationException` en las dos, y aquí es un `SAF_04`.
pub fn refuse_a_multisignature_of_an_invoice(
    round: SignatureRound,
    format: RequestedFormat,
) -> Result<(), Refusal> {
    let first = matches!(round, SignatureRound::First);
    if !first && matches!(format, RequestedFormat::FacturaE) {
        return Err(Refusal::new(
            SafCode::UnsupportedOperation,
            "una factura ni se cofirma ni se contrafirma: AOFacturaESigner lanza una \
             UnsupportedOperationException en las dos",
        ));
    }
    Ok(())
}

/// CAdES y XAdES contrafirman: el resto sale con el rechazo del original.
pub fn refuse_a_countersignature_outside_cades_and_xades(
    round: SignatureRound,
    format: RequestedFormat,
) -> Result<(), Refusal> {
    let counters = matches!(round, SignatureRound::Counter { .. });
    let supported = matches!(
        format,
        RequestedFormat::Cades | RequestedFormat::Cms | RequestedFormat::Xades(_)
    );
    if counters && !supported {
        return Err(countersign_refusal());
    }
    Ok(())
}

/// `mode=explicit` con XAdES: `SAF_06`, la desviación que documenta `domain/protocol/mod.rs`.
pub fn refuse_explicit_xades(
    format: RequestedFormat,
    declared_params: &[(String, String)],
) -> Result<(), Refusal> {
    let explicit = declared_params.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("mode") && value.eq_ignore_ascii_case("explicit")
    });
    if explicit && matches!(format, RequestedFormat::Xades(_)) {
        return Err(Refusal::new(
            SafCode::UnsupportedFormat,
            "'mode=explicit' con XAdES no se reproduce: ver domain/protocol/mod.rs",
        ));
    }
    Ok(())
}

/// El formato que nombra la sede, nada si pide `auto`, o el `SAF_06` que nombra
/// el que el original no firma en tres fases.
fn requested_format(url: &AfirmaUrl) -> Result<Option<RequestedFormat>, Refusal> {
    let format = required(url, "format", Parameter::Format)
        .map_err(|refusal| refusal.because(RefusalSituation::MissingFormat))?;

    if format.trim().eq_ignore_ascii_case(AUTO) {
        return Ok(None);
    }
    RequestedFormat::named(format).map(Some).ok_or_else(|| {
        Refusal::new(
            SafCode::UnsupportedFormat,
            format!("el formato '{format}' no se atiende: no lo firma AutoFirma en tres fases"),
        )
    })
}

/// La huella del `algorithm` que pide la sede, o el `SAF_03` que lo nombra.
fn check_algorithm(url: &AfirmaUrl) -> Result<AskedAlgorithm, Refusal> {
    let algorithm = required(url, "algorithm", Parameter::Algorithm)?;
    AskedAlgorithm::named(algorithm).ok_or_else(|| {
        Refusal::about(
            Parameter::Algorithm,
            format!("el algoritmo '{algorithm}' no se atiende: rFirma firma con SHA-2"),
        )
    })
}

/// `'countersign' solo existe en CAdES y XAdES`, compartido por `read_operation`
/// y por el `cop` de `signandsave`.
fn countersign_refusal() -> Refusal {
    Refusal::new(
        SafCode::UnsupportedOperation,
        "'countersign' no existe fuera de CAdES y XAdES: AOPDFSigner.countersign lanza una \
         UnsupportedOperationException",
    )
}

/// La petición de `signandsave`: misma lectura y mismos rechazos que `sign`,
/// con `dat` opcional y lo del guardado (`ProtocolInvocationLauncherSignAndSave`, 1.9.2).
fn sign_and_save_request(url: &AfirmaUrl, data: &dyn DataSource) -> Result<SiteOperation, Refusal> {
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

    Ok(SiteOperation::SignAndSave(SignAndSaveRequest {
        round,
        algorithm,
        document,
        requested,
        filter: site_filter(&declared),
        headless: properties.is_headless(),
        filename,
        extensions: comma_list_value(property_value(&declared, FILENAME_SAVE_EXTS)),
        description: property_value(&declared, FILENAME_SAVE_DESCRIPTION),
        starting_folder: property_value(&declared, FILENAME_SAVE_CURRENT_DIR),
        load_extensions: comma_list_value(property_value(&declared, FILENAME_EXTS)),
        load_description: property_value(&declared, FILENAME_DESCRIPTION),
        load_starting_folder: property_value(&declared, FILENAME_CURRENT_DIR),
        load_filename: properties.actual_name().map(str::to_owned),
        chosen_name: None,
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
        COUNTERSIGN => counter_round(declared),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!(
                "el 'cop' de 'signandsave' no admite '{other}': solo 'sign', 'cosign' o \
                 'countersign'"
            ),
        )),
    }
}

/// El `dat` de una firma, si vino: ausente no es rechazo, vacío sí lo es.
fn optional_document(url: &AfirmaUrl, data: &dyn DataSource) -> Result<Option<Vec<u8>>, Refusal> {
    if url.parameter("dat").is_none() {
        return Ok(None);
    }
    Ok(Some(read_document(url, data)?))
}

/// El documento de `dat`, ya obtenido: se lee una sola vez, la pida quien lo pida.
fn read_document(url: &AfirmaUrl, data: &dyn DataSource) -> Result<Vec<u8>, Refusal> {
    let document = data_of(url, data)?;
    if document.is_empty() {
        return Err(Refusal::new(
            SafCode::SignWithoutData,
            "el parametro 'dat' viene vacio: no hay nada que firmar",
        ));
    }
    Ok(document)
}

/// Los bytes de `dat`, bajados de su URL o descodificados del Base64
/// (`DataDownloader.downloadData`, 1.9.2).
///
/// El `gzip=true` se resuelve antes de mirar si el valor era una URL, igual que el original: lo
/// que se baja de una URL no se descomprime nunca, porque una URL no es Base64.
fn data_of(url: &AfirmaUrl, data: &dyn DataSource) -> Result<Vec<u8>, Refusal> {
    let value = required(url, "dat", Parameter::Data)?;
    if let Some(remote) = download_url(value) {
        return data.download(remote).map_err(|detail| {
            Refusal::about(
                Parameter::Data,
                format!("no se han podido obtener los datos de '{remote}': {detail}"),
            )
        });
    }

    let decoded = decode_base64(value, Parameter::Data)?;
    if is_gzip(url) && !decoded.is_empty() {
        return decompress_gzip(&decoded);
    }
    Ok(decoded)
}

fn is_gzip(url: &AfirmaUrl) -> bool {
    url.parameter("gzip")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

fn decompress_gzip(compressed: &[u8]) -> Result<Vec<u8>, Refusal> {
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|error| {
        Refusal::about(
            Parameter::Data,
            format!("el parametro 'dat' no es un gzip valido: {error}"),
        )
    })?;
    Ok(decompressed)
}

/// El nombre de fichero elegido sin su extensión (`AOPDFSigner.getSignedName`, 1.9.2), o el
/// nombre entero si no tiene punto.
fn base_name(chosen_name: &str) -> &str {
    chosen_name
        .rsplit_once('.')
        .map_or(chosen_name, |(base, _)| base)
}

/// La petición de guardado: solo `dat` es obligatorio (`ProtocolInvocationLauncherSave`, 1.9.2).
fn save_request(url: &AfirmaUrl, data: &dyn DataSource) -> Result<SiteOperation, Refusal> {
    let filename = optional(url, "filename");
    if let Some(filename) = &filename {
        check_filename(filename)?;
    }
    if let Some(extensions) = optional(url, "exts") {
        check_extensions(&extensions)?;
    }

    Ok(SiteOperation::Save(SaveRequest {
        data: data_of(url, data)?,
        title: optional(url, "title"),
        filename,
        extensions: comma_list(url, "exts"),
        description: optional(url, "desc"),
    }))
}

/// La petición de carga: nada es obligatorio (`ProtocolInvocationLauncherLoad`, 1.9.2).
fn load_request(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    Ok(SiteOperation::Load(LoadRequest {
        title: optional(url, "title"),
        extensions: comma_list(url, "exts"),
        description: optional(url, "desc"),
        starting_folder: optional(url, "filePath"),
        multiple: url
            .parameter("multiload")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
    }))
}

/// La petición del lote remoto: dos URL de servlet, el lote y lo mínimo que se
/// lee de dentro de él (`ProtocolInvocationLauncherBatch`, 1.9.2).
fn batch_request(url: &AfirmaUrl, data: &dyn DataSource) -> Result<SiteOperation, Refusal> {
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
    let lote_base64 = match is_gzip(url) || download_url(value).is_some() {
        true => STANDARD.encode(&lote),
        false => value.to_owned(),
    };
    let (algorithm, stop_on_error) = batch_algorithm_and_stop_on_error(json, &lote)?;

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
        filter: site_filter(declared.crossing()),
        sticky: sticky_certificate(url),
        headless: declared.is_headless(),
        algorithm,
        stop_on_error,
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

/// Un nombre de fichero sin los caracteres que el original prohíbe, o el `SAF_03` que lo nombra.
fn check_filename(candidate: &str) -> Result<(), Refusal> {
    check_free_of(candidate, &FORBIDDEN_IN_A_FILENAME, Parameter::Filename)
}

/// Las extensiones sin los caracteres que el original prohíbe, el `;` y el espacio incluidos.
fn check_extensions(candidate: &str) -> Result<(), Refusal> {
    check_free_of(candidate, &FORBIDDEN_IN_AN_EXTENSION, Parameter::Extensions)
}

fn check_free_of(candidate: &str, forbidden: &[char], blame: Parameter) -> Result<(), Refusal> {
    match candidate.chars().find(|it| forbidden.contains(it)) {
        Some(character) => Err(Refusal::about(
            blame,
            format!("el parametro '{blame}' trae un caracter que no se admite: {character}"),
        )),
        None => Ok(()),
    }
}

/// El `algorithm` y el `stoponerror` del lote: atributo de `<signbatch>` en el
/// XML heredado, o campos del objeto raíz en JSON.
fn batch_algorithm_and_stop_on_error(json: bool, lote: &[u8]) -> Result<(String, bool), Refusal> {
    let (algorithm, stop_on_error) = if json {
        batch_header_from_json(lote)?
    } else {
        batch_header_from_xml(lote)?
    };

    if AskedAlgorithm::named(&algorithm).is_none() {
        return Err(Refusal::about(
            Parameter::Algorithm,
            format!("el algoritmo de lote '{algorithm}' no se atiende"),
        ));
    }

    Ok((algorithm, stop_on_error))
}

fn batch_header_from_json(lote: &[u8]) -> Result<(String, bool), Refusal> {
    let value: serde_json::Value = serde_json::from_slice(lote).map_err(|error| {
        Refusal::about(
            Parameter::Data,
            format!("el lote no es JSON valido: {error}"),
        )
    })?;

    let algorithm = value
        .get("algorithm")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Refusal::about(Parameter::Algorithm, "falta el parametro 'algorithm'"))?
        .to_owned();
    let stop_on_error = value
        .get("stoponerror")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    Ok((algorithm, stop_on_error))
}

fn batch_header_from_xml(lote: &[u8]) -> Result<(String, bool), Refusal> {
    let text = std::str::from_utf8(lote).map_err(|error| {
        Refusal::about(Parameter::Data, format!("el lote no es UTF-8: {error}"))
    })?;

    let mut reader = quick_xml::Reader::from_str(text);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(tag) | quick_xml::events::Event::Empty(tag)) => {
                let mut algorithm = None;
                let mut stop_on_error = false;
                for attribute in tag.attributes().flatten() {
                    let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                    match attribute.key.as_ref() {
                        b"algorithm" => algorithm = Some(value),
                        b"stoponerror" => stop_on_error = value.eq_ignore_ascii_case("true"),
                        _ => {}
                    }
                }
                let algorithm = algorithm.ok_or_else(|| {
                    Refusal::about(Parameter::Algorithm, "falta el parametro 'algorithm'")
                })?;
                return Ok((algorithm, stop_on_error));
            }
            Ok(quick_xml::events::Event::Eof) => {
                return Err(Refusal::about(
                    Parameter::Data,
                    "el lote no tiene elemento raiz",
                ));
            }
            Err(error) => {
                return Err(Refusal::about(
                    Parameter::Data,
                    format!("el lote no es XML valido: {error}"),
                ));
            }
            _ => {}
        }
    }
}

/// Un parámetro opcional, o nada si no vino o vino vacío.
fn optional(url: &AfirmaUrl, name: &str) -> Option<String> {
    url.parameter(name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Una lista separada por comas, o vacía si el parámetro no vino.
fn comma_list(url: &AfirmaUrl, name: &str) -> Vec<String> {
    comma_list_value(url.parameter(name).map(str::to_owned))
}

/// Una lista separada por comas a partir de un valor ya leído, o vacía si no vino.
fn comma_list_value(value: Option<String>) -> Vec<String> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|piece| !piece.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// El valor de una clave del `properties` ya decodificado, o nada si no vino o vino vacío.
fn property_value(declared: &[(String, String)], key: &str) -> Option<String> {
    declared
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
        .filter(|value| !value.is_empty())
}

/// Un parámetro que la operación exige, o el `SAF_03` que lo nombra.
fn required<'u>(url: &'u AfirmaUrl, name: &str, blame: Parameter) -> Result<&'u str, Refusal> {
    url.parameter(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| Refusal::about(blame, format!("falta el parametro '{name}'")))
}

/// El verbo que pide la sede: el parámetro `op` si viene, y si no, el dominio
/// de la URL.
fn verb_of(url: &AfirmaUrl) -> String {
    url.parameter("op")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| url.verb())
        .to_owned()
}

/// El `properties` que mandó la sede, partido en lo que cruza al firmador y lo que el
/// lanzador interpreta él mismo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeclaredProperties {
    crossing: Vec<(String, String)>,
    headless: bool,
    actual_name: Option<String>,
}

impl DeclaredProperties {
    /// Los pares que sí se le entregan al firmador.
    pub fn crossing(&self) -> &[(String, String)] {
        &self.crossing
    }

    /// Si la sede se conforma con el único certificado que pase el filtro.
    pub fn is_headless(&self) -> bool {
        self.headless
    }

    /// El nombre que la sede propone al selector de documento.
    pub fn actual_name(&self) -> Option<&str> {
        self.actual_name.as_deref()
    }
}

/// Los pares del `.properties` que la sede mandó dentro de `properties`.
///
/// Viaja en Base64 **URL-safe** (`Base64.encode(bytes, true)` del original), y
/// el descodificador es tolerante a propósito con lo que sí puede llegar: la
/// `/` del alfabeto normal y el relleno ausente.
///
/// Un valor que aun así no se pueda leer **no tumba la operación**: se descarta
/// con traza y el trámite sigue sin parámetros adicionales, que es lo que hace
/// `UrlParametersToSign.setSignParameters` (`UrlParametersToSign.java:207`, 1.9.2).
/// El precio de rechazarlo sería una firma que habría salido.
fn declared_properties(url: &AfirmaUrl) -> DeclaredProperties {
    let all = readable_properties(url);
    DeclaredProperties {
        headless: asks_to_skip_the_dialog(&all),
        actual_name: property_value(&all, FILENAME_ACTUAL_NAME),
        crossing: without_the_launcher_keys(all),
    }
}

/// Los pares que se hayan podido leer de `properties`, vacío si no se pudo leer ninguno.
fn readable_properties(url: &AfirmaUrl) -> Vec<(String, String)> {
    let Some(encoded) = url.parameter("properties").filter(|it| !it.is_empty()) else {
        return Vec::new();
    };

    let Ok(decoded) = decode_base64(encoded, Parameter::Properties) else {
        return discarded(encoded.len(), "no es Base64");
    };

    match String::from_utf8(decoded) {
        Ok(text) => pairs_of(&text),
        Err(_) => discarded(encoded.len(), "no es texto UTF-8"),
    }
}

fn discarded(length: usize, reason: &str) -> Vec<(String, String)> {
    eprintln!("rfirma: se descarta 'properties' ({length} caracteres): {reason}");
    Vec::new()
}

/// `headless=true`, o su sinónimo `mandatoryCertSelection=false`
/// (`CertFilterManager.isMandatoryCertificate`, 1.9.2).
fn asks_to_skip_the_dialog(declared: &[(String, String)]) -> bool {
    let headless = property_value(declared, HEADLESS)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    let mandatory = property_value(declared, MANDATORY_CERT_SELECTION)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("false"));
    headless || mandatory
}

/// Los pares sin las cuatro claves que el lanzador interpreta él mismo, compartido con los
/// `extraparams` de un elemento de lote.
pub fn without_the_launcher_keys(declared: Vec<(String, String)>) -> Vec<(String, String)> {
    declared
        .into_iter()
        .filter(|(key, _)| {
            !INTERPRETED_BY_THE_LAUNCHER
                .iter()
                .any(|interpreted| key.eq_ignore_ascii_case(interpreted))
        })
        .collect()
}

/// El Base64 **URL-safe** del protocolo, con la misma tolerancia en todos los
/// parámetros que lo llevan.
///
/// Tolerante a propósito con lo que sí puede llegar: la `/` del alfabeto normal
/// y el relleno ausente o de más. El `+` del alfabeto normal, en cambio, nunca
/// llega hasta aquí: [`AfirmaUrl`] ya lo ha convertido en un espacio, porque el
/// original pasa cada valor por `URLDecoder`.
fn decode_base64(encoded: &str, blame: Parameter) -> Result<Vec<u8>, Refusal> {
    let normalized: String = encoded
        .chars()
        .filter(|character| *character != '=')
        .map(|character| if character == '/' { '_' } else { character })
        .collect();

    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(normalized.as_bytes())
        .map_err(|error| {
            Refusal::about(
                blame,
                format!("el parametro '{blame}' no es Base64: {error}"),
            )
        })
}

/// Los pares de un bloque `java.util.Properties`.
pub fn pairs_of(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for line in text.lines() {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some(at) = separator_in(line) else {
            continue;
        };
        let key = unescape(line[..at].trim_end());
        let value = unescape(line[at + 1..].trim_start());
        if !key.is_empty() {
            pairs.push((key, value));
        }
    }

    pairs
}

/// Dónde parte la línea: el primer `=` o `:` que no venga escapado.
fn separator_in(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'=' | b':' => return Some(index),
            _ => {}
        }
    }
    None
}

/// Deshace las barras de escape: las tres que escribe el proyecto —`\\`, `\n`,
/// `\r`— más `\t`, y cualquier otra barra que se queda con lo que lleve detrás.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => break,
        }
    }

    out
}

#[cfg(test)]
mod tests;
