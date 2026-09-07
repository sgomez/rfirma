//! Lo que la sede pide por el canal ya abierto, leído de la URL.

use base64::Engine as _;

use super::codes::{Parameter, SafCode};
use super::detection::{shape_of, DetectedShape};
use super::filters::{site_filter, SiteFilter};
use super::parameters::{
    check_local_access_is_not_requested, check_minimum_client_version, sticky_certificate,
    StickyCertificate,
};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

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

/// El formato de firma PAdES.
pub const PADES: &str = "pades";

/// `format=auto`: la sede no fija formato y pide que se deduzca del documento.
pub const AUTO: &str = "auto";

/// El algoritmo que rFirma sabe producir.
pub const ACCEPTED_ALGORITHMS: [&str; 2] = ["sha256", "sha256withrsa"];

/// Los algoritmos del lote que el original acepta (`BatchSigner`, XSD de `signbatch`).
pub const ACCEPTED_BATCH_ALGORITHMS: [&str; 4] = ["sha1", "sha256", "sha384", "sha512"];

/// `localBatchProcess=true`: el lote local, rechazado aquí hasta #468.
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
    /// `batch`: la sede pide firmar un lote remoto.
    Batch(BatchRequest),
}

/// Cuál de las dos firmas pidió la sede.
///
/// En PAdES las dos recorren el mismo camino —cofirmar es volver a firmar—, y
/// la distinción se guarda porque es lo que la sede pidió y lo que la ventana
/// tiene que contarle a la persona antes de que consienta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureRound {
    /// `sign`: se firma lo que llega.
    First,
    /// `cosign`: se vuelve a firmar sobre las firmas que el PDF ya trae.
    Again,
}

/// La petición de `sign` o de `cosign`.
///
/// Lleva **el documento ya descodificado**, y no el Base64: lo que se firma son
/// bytes, y dejar el Base64 vivo hasta el momento de firmar es tener dos
/// copias de lo mismo y una ocasión de descodificarlo dos veces distintas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRequest {
    round: SignatureRound,
    algorithm: String,
    document: Vec<u8>,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
}

impl SignRequest {
    /// `sign` o `cosign`.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// El algoritmo tal y como lo pidió la sede, ya admitido.
    pub fn algorithm(&self) -> &str {
        &self.algorithm
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
}

/// La petición de `selectcert`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectCertificate {
    filter: SiteFilter,
    sticky: StickyCertificate,
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
    algorithm: String,
    document: Option<Vec<u8>>,
    format_auto: bool,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
    filename: Option<String>,
    extensions: Vec<String>,
    description: Option<String>,
    starting_folder: Option<String>,
    load_extensions: Vec<String>,
    load_description: Option<String>,
    load_starting_folder: Option<String>,
}

impl SignAndSaveRequest {
    /// `sign` o `cosign`, según pida `cop`.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// El algoritmo tal y como lo pidió la sede, ya admitido.
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// El documento que la sede manda, si vino: sin `dat` queda por elegir.
    pub fn document(&self) -> Option<&[u8]> {
        self.document.as_deref()
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

    /// El nombre propuesto al diálogo de guardado (`AOPDFSigner.getSignedName`, 1.9.2).
    pub fn proposed_name(&self) -> String {
        self.filename
            .clone()
            .unwrap_or_else(|| format!("{DEFAULT_SIGNED_NAME}.pdf"))
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

    /// El documento que la persona acaba de elegir, con el mismo veredicto de formato que si
    /// hubiera llegado en `dat`: solo se comprueba cuando la sede pidió `format=auto`.
    pub fn with_chosen_document(&self, document: Vec<u8>) -> Result<Self, Refusal> {
        if self.format_auto {
            reject_unless_pdf(shape_of(&document))?;
        }
        Ok(Self {
            document: Some(document),
            ..self.clone()
        })
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

/// La petición de `batch`: firmar un lote remoto (`BatchSigner`, 1.9.2).
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
    presigner_url: String,
    postsigner_url: String,
    needcert: bool,
    filter: SiteFilter,
    sticky: StickyCertificate,
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

    /// La URL del servlet de prefirma.
    pub fn presigner_url(&self) -> &str {
        &self.presigner_url
    }

    /// La URL del servlet de postfirma.
    pub fn postsigner_url(&self) -> &str {
        &self.postsigner_url
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
pub fn read_operation(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    check_minimum_client_version(url.parameter("mcv"))?;
    if let Some(data) = url.parameter("dat") {
        check_local_access_is_not_requested(data)?;
    }

    match verb_of(url).as_str() {
        SELECT_CERTIFICATE => Ok(SiteOperation::SelectCertificate(SelectCertificate {
            filter: site_filter(&declared_properties(url)?),
            sticky: sticky_certificate(url),
        })),
        SIGN => sign_request(url, SignatureRound::First),
        COSIGN => sign_request(url, SignatureRound::Again),
        COUNTERSIGN => Err(countersign_refusal()),
        SAVE => save_request(url),
        LOAD => load_request(url),
        BATCH => batch_request(url),
        SIGN_AND_SAVE => sign_and_save_request(url),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("la operacion '{other}' no se atiende"),
        )),
    }
}

/// La petición de firma, con las cuatro comprobaciones de
/// `UrlParametersToSign` que rFirma hereda.
///
/// El orden importa poco salvo en una cosa: el **formato** se mira antes que
/// nada de lo demás, porque una sede que pide XAdES no se merece un `SAF_03`
/// sobre el algoritmo cuando lo que pasa es que ese formato no se atiende.
fn sign_request(url: &AfirmaUrl, round: SignatureRound) -> Result<SiteOperation, Refusal> {
    let document = if format_verdict(url)? {
        let document = read_document(url)?;
        reject_unless_pdf(shape_of(&document))?;
        Some(document)
    } else {
        None
    };

    let algorithm = check_algorithm(url)?;

    let document = match document {
        Some(document) => document,
        None => read_document(url)?,
    };

    let declared = declared_properties(url)?;
    Ok(SiteOperation::Sign(SignRequest {
        round,
        algorithm,
        document,
        filter: site_filter(&declared),
        declared,
    }))
}

/// `true` si `format=auto` (hay que detectar el documento), `false` si es `PAdES` explícito,
/// o el `SAF_04` que nombra el formato que no se atiende.
fn format_verdict(url: &AfirmaUrl) -> Result<bool, Refusal> {
    let format = required(url, "format", Parameter::Format)
        .map_err(|refusal| refusal.because(RefusalSituation::MissingFormat))?;

    if format.trim().eq_ignore_ascii_case(AUTO) {
        return Ok(true);
    }
    if !format.trim().eq_ignore_ascii_case(PADES) {
        return Err(Refusal::new(
            SafCode::UnsupportedFormat,
            format!("el formato '{format}' no se atiende: rFirma solo firma PAdES"),
        ));
    }
    Ok(false)
}

/// El `algorithm` ya admitido, o el `SAF_03` que lo nombra.
fn check_algorithm(url: &AfirmaUrl) -> Result<String, Refusal> {
    let algorithm = required(url, "algorithm", Parameter::Algorithm)?;
    if !ACCEPTED_ALGORITHMS.contains(&algorithm.trim().to_ascii_lowercase().as_str()) {
        return Err(Refusal::about(
            Parameter::Algorithm,
            format!("el algoritmo '{algorithm}' no se atiende: rFirma firma con SHA256withRSA"),
        ));
    }
    Ok(algorithm.trim().to_owned())
}

/// `'countersign' no existe en PAdES`, compartido por `read_operation` y por el `cop` de `signandsave`.
fn countersign_refusal() -> Refusal {
    Refusal::new(
        SafCode::UnsupportedOperation,
        "'countersign' no existe en PAdES: AOPDFSigner.countersign lanza una \
         UnsupportedOperationException",
    )
}

/// La petición de `signandsave`: misma lectura y mismos rechazos que `sign`,
/// con `dat` opcional y lo del guardado (`ProtocolInvocationLauncherSignAndSave`, 1.9.2).
fn sign_and_save_request(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    let round = round_of_cop(url)?;

    let format_auto = format_verdict(url)?;
    let document = if format_auto {
        match optional_document(url)? {
            Some(document) => {
                reject_unless_pdf(shape_of(&document))?;
                Some(document)
            }
            None => None,
        }
    } else {
        None
    };

    let algorithm = check_algorithm(url)?;

    let document = match document {
        Some(document) => Some(document),
        None => optional_document(url)?,
    };

    let declared = declared_properties(url)?;
    Ok(SiteOperation::SignAndSave(SignAndSaveRequest {
        round,
        algorithm,
        document,
        format_auto,
        filter: site_filter(&declared),
        filename: optional(url, "filename"),
        extensions: comma_list_value(property_value(&declared, FILENAME_SAVE_EXTS)),
        description: property_value(&declared, FILENAME_SAVE_DESCRIPTION),
        starting_folder: property_value(&declared, FILENAME_SAVE_CURRENT_DIR),
        load_extensions: comma_list_value(property_value(&declared, FILENAME_EXTS)),
        load_description: property_value(&declared, FILENAME_DESCRIPTION),
        load_starting_folder: property_value(&declared, FILENAME_CURRENT_DIR),
        declared,
    }))
}

/// La ronda que pide `cop` (`sign`→`First`, `cosign`→`Again`), o el `SAF_04` que la nombra.
fn round_of_cop(url: &AfirmaUrl) -> Result<SignatureRound, Refusal> {
    let cop = url
        .parameter("cop")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match cop.as_str() {
        SIGN => Ok(SignatureRound::First),
        COSIGN => Ok(SignatureRound::Again),
        COUNTERSIGN => Err(countersign_refusal()),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("el 'cop' de 'signandsave' no admite '{other}': solo 'sign' o 'cosign'"),
        )),
    }
}

/// El `dat` de `signandsave`, si vino: ausente no es rechazo, vacío sí lo es.
fn optional_document(url: &AfirmaUrl) -> Result<Option<Vec<u8>>, Refusal> {
    if url.parameter("dat").is_none() {
        return Ok(None);
    }
    Ok(Some(read_document(url)?))
}

/// El documento de `dat`, decodificado: se lee una sola vez, la pida quien lo pida.
fn read_document(url: &AfirmaUrl) -> Result<Vec<u8>, Refusal> {
    let data = required(url, "dat", Parameter::Data)?;
    let document = decode_base64(data, Parameter::Data)?;
    if document.is_empty() {
        return Err(Refusal::new(
            SafCode::SignWithoutData,
            "el parametro 'dat' viene vacio: no hay nada que firmar",
        ));
    }
    Ok(document)
}

/// La única traducción de «PDF / XML / binario» a formato efectivo: hasta que
/// el #468 traiga CAdES y XAdES, solo PDF se atiende.
fn reject_unless_pdf(shape: DetectedShape) -> Result<(), Refusal> {
    match shape {
        DetectedShape::Pdf => Ok(()),
        DetectedShape::Xml | DetectedShape::Binary => Err(Refusal::new(
            SafCode::UnsupportedFormat,
            "el documento de 'format=auto' no es PDF: rFirma solo firma PAdES",
        )),
    }
}

/// La petición de guardado: solo `dat` es obligatorio (`ProtocolInvocationLauncherSave`, 1.9.2).
fn save_request(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    let data = required(url, "dat", Parameter::Data)?;
    let document = decode_base64(data, Parameter::Data)?;
    Ok(SiteOperation::Save(SaveRequest {
        data: document,
        title: optional(url, "title"),
        filename: optional(url, "filename"),
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
fn batch_request(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    if url
        .parameter(LOCAL_BATCH_PROCESS)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return Err(Refusal::new(
            SafCode::LocalBatchSign,
            "el lote local no se atiende: va en #468",
        ));
    }

    let presigner_url = required(url, "batchpresignerurl", Parameter::BatchPresignerUrl)?;
    check_absolute_https_url(presigner_url, Parameter::BatchPresignerUrl)?;
    let postsigner_url = required(url, "batchpostsignerurl", Parameter::BatchPostsignerUrl)?;
    check_absolute_https_url(postsigner_url, Parameter::BatchPostsignerUrl)?;

    let lote_base64 = required(url, "dat", Parameter::Data)?;
    let lote = decode_base64(lote_base64, Parameter::Data)?;
    let json = url
        .parameter("jsonbatch")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    let (algorithm, stop_on_error) = batch_algorithm_and_stop_on_error(json, &lote)?;

    let declared = declared_properties(url)?;
    Ok(SiteOperation::Batch(BatchRequest {
        lote,
        lote_base64: lote_base64.to_owned(),
        json,
        presigner_url: presigner_url.to_owned(),
        postsigner_url: postsigner_url.to_owned(),
        needcert: url
            .parameter("needcert")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
        filter: site_filter(&declared),
        sticky: sticky_certificate(url),
        algorithm,
        stop_on_error,
    }))
}

/// La URL de un servlet del lote: absoluta y `https`, o el `SAF_03` que la nombra.
fn check_absolute_https_url(candidate: &str, blame: Parameter) -> Result<(), Refusal> {
    const SCHEME: &str = "https://";
    if candidate.len() <= SCHEME.len() || !candidate.to_ascii_lowercase().starts_with(SCHEME) {
        return Err(Refusal::about(
            blame,
            format!("la url '{candidate}' debe ser absoluta y 'https'"),
        ));
    }
    Ok(())
}

/// El `algorithm` y el `stoponerror` del lote: atributo de `<signbatch>` en el
/// XML heredado, o campos del objeto raíz en JSON.
fn batch_algorithm_and_stop_on_error(json: bool, lote: &[u8]) -> Result<(String, bool), Refusal> {
    let (algorithm, stop_on_error) = if json {
        batch_header_from_json(lote)?
    } else {
        batch_header_from_xml(lote)?
    };

    if !ACCEPTED_BATCH_ALGORITHMS.contains(&algorithm.to_ascii_lowercase().as_str()) {
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
        .trim()
        .to_ascii_lowercase()
}

/// Los pares del `.properties` que la sede mandó dentro de `properties`.
///
/// Viaja en Base64 **URL-safe** (`Base64.encode(bytes, true)` del original), y
/// el descodificador es tolerante a propósito con lo que sí puede llegar: la
/// `/` del alfabeto normal y el relleno ausente. Un cliente que mande `/` no
/// está atacando nada, y rechazarle la llamada entera por eso sería inventarse
/// una incompatibilidad que el original no tiene.
///
/// El `+` del alfabeto normal, en cambio, **nunca llega hasta aquí**:
/// [`AfirmaUrl`] ya lo ha convertido en un espacio, porque el original pasa
/// cada valor por `URLDecoder` (ver el encabezado de [`crate::site::domain::protocol::url`]).
/// Así que una sede que mande Base64 estándar con `+` se lleva el `SAF_03` —
/// igual que en el original, que decodifica igual—, y aquí no hay ningún brazo
/// que lo intente: sería código muerto que promete una tolerancia que no
/// existe.
fn declared_properties(url: &AfirmaUrl) -> Result<Vec<(String, String)>, Refusal> {
    let Some(encoded) = url.parameter("properties").filter(|it| !it.is_empty()) else {
        return Ok(Vec::new());
    };

    let decoded = decode_base64(encoded, Parameter::Properties)?;

    let text = String::from_utf8(decoded).map_err(|error| {
        Refusal::about(
            Parameter::Properties,
            format!("el parametro 'properties' no es texto: {error}"),
        )
    })?;

    Ok(pairs_of(&text))
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
