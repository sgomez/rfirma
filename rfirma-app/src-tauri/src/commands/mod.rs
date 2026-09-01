//! **Las órdenes de Tauri**: lo único que la ventana puede pedirle al backend.
//!
//! Son once, y la lista es cerrada a propósito. Cada una rellena un puerto que
//! la interfaz ya tenía declarado —`CertificateStore`, `Layer2Composer` y
//! `SigningBackend` desde el #76, `DocumentPicker` y `PdfSource` desde el #82,
//! `PreferencesStore` y `LanguagePreference` desde que hay dónde guardar—,
//! así que la ventana no aprende nada nuevo de Tauri: sigue hablando con los
//! mismos puertos y es `main.tsx` quien elige estas implementaciones.
//!
//! # Los ajustes se guardan al elegirlos, y en el disco
//!
//! `read_configuration` y `write_configuration` son las dos mitades del puerto
//! `PreferencesStore`, y `forget_activity` es lo que promete «Recordar mi
//! actividad» al apagarse. Las tres pasan por [`crate::memory::Memory`], que es
//! el único sitio donde los dos interruptores no se pueden olvidar (ADR-0010).
//!
//! # El documento entra por el portal y se nombra con un identificador
//!
//! `open_document` abre el diálogo del sistema **desde Rust** (ID-63), apunta
//! lo que el portal conceda en [`crate::memory::OpenedDocuments`] y devuelve un
//! identificador opaco; `read_document` entrega sus bytes contra ese
//! identificador. Ninguna de las dos devuelve una ruta, y ninguna reescribe las
//! reglas de [`PortalDocument`]: son un adaptador delgado encima (ID-65).
//!
//! # Y hay un camino más, que no es una orden
//!
//! Soltar un fichero en la ventana desemboca en el mismo sitio, pero **al
//! revés**: no lo pide la ventana, le ocurre. Por eso [`dropped_document`] no
//! es una novena orden sino lo que alimenta el evento [`DOCUMENT_DROPPED`], que
//! `lib.rs` emite desde el manejador del arrastre nativo (ID-67). Lo que cruza
//! sigue siendo un [`OpenedDocumentView`]: las rutas que trae el arrastre se
//! quedan de este lado.
//!
//! # Ninguna orden devuelve una ruta del anfitrión
//!
//! No es una recomendación, es una consecuencia del ADR-0011: bajo el arenero
//! la aplicación **no conoce** la ruta real de un documento —el portal solo la
//! da a un llamante `is_host`, que un flatpak nunca es—, así que devolver una
//! sería devolver una mentira. Lo que sale de aquí son **nombres**: el del
//! fichero firmado y el de la carpeta donde cayó. Hay una prueba abajo que lee
//! este mismo fichero y se pone roja si algún tipo de salida gana un campo con
//! una ruta dentro.
//!
//! # El recorrido está partido en tres porque el PIN va en medio
//!
//! `begin_signing` → `sign_with_pin` → `finish_signing`. Una sola orden que
//! hiciera las tres dejaría a la ventana sin nada que contar durante los
//! segundos de la postfirma, y —lo que importa más— obligaría a mandar el PIN
//! junto con el documento, cuando todavía no se sabe si el documento se puede
//! firmar. El ciclo a medias vive en [`SigningSession`], no en la ventana: lo
//! que la ventana no tiene no lo puede filtrar.

pub mod isolate;

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::destination::{CheckedFolder, PortalDocument};
use crate::ffi::BridgeError;
use crate::memory::{
    Configuration, Memory, MemoryError, OpenedDocuments, Situation as MemorySituation, Theme,
};
use crate::pkcs11::{
    self, CertificateRef, CertificateStatus, Situation, TokenCertificate, TokenError,
};
use crate::signing::{
    compose_layer2_text, cycle, AdmissibleDocument, Language, MediaBox, OpenCycle, Page, Refusal,
    Rotation, SealMismatch, SessionSeal, SignatureBox, SignatureConfig, SigningRequest,
    TokenSignature, UserSpaceRect, VisibleTextFields,
};

pub use isolate::{Isolate, IsolateGone};

/// Lo que la ventana recibe cuando algo sale mal.
///
/// Es la forma del ID-29 y la misma que ya tiene `TokenFailure` en TypeScript:
/// una **situación** nuestra, que el catálogo traduce a los seis idiomas, y el
/// texto original **crudo** al lado, sin traducir ni recortar, para poder
/// pegarlo en un informe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    /// El nombre de la situación, en `camelCase`, tal cual lo espera la unión
    /// de TypeScript.
    pub situation: String,
    /// El detalle crudo. Nunca vacío.
    pub detail: String,
    /// Cuántos intentos de PIN quedan, cuando el módulo lo dice.
    pub attempts_left: Option<u32>,
}

impl Failure {
    fn new(situation: &str, detail: impl Into<String>) -> Self {
        Self {
            situation: situation.to_owned(),
            detail: detail.into(),
            attempts_left: None,
        }
    }
}

/// El nombre en `camelCase` de una situación del token, que es la clave con la
/// que el catálogo la traduce.
fn situation_name(situation: Situation) -> &'static str {
    match situation {
        Situation::IncorrectPin => "incorrectPin",
        Situation::PinLocked => "pinLocked",
        Situation::TokenAbsent => "tokenAbsent",
        Situation::ExpiredSession => "expiredSession",
        Situation::ModuleNotFound => "moduleNotFound",
        Situation::CertificateNotFound => "certificateNotFound",
        Situation::Unknown => "unknown",
    }
}

impl From<TokenError> for Failure {
    fn from(error: TokenError) -> Self {
        Self::new(situation_name(error.situation()), error.detail())
    }
}

impl From<MemoryError> for Failure {
    fn from(error: MemoryError) -> Self {
        let situation = match error.situation() {
            MemorySituation::Unreadable => "settingsUnreadable",
            MemorySituation::Unwritable => "settingsUnwritable",
        };
        Self::new(situation, error.detail().to_owned())
    }
}

impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        Self::new(refusal.situation(), refusal.to_string())
    }
}

impl From<SealMismatch> for Failure {
    fn from(error: SealMismatch) -> Self {
        Self::new("sealMismatch", error.to_string())
    }
}

impl From<BridgeError> for Failure {
    fn from(error: BridgeError) -> Self {
        Self::new("bridgeFailed", error.to_string())
    }
}

impl From<IsolateGone> for Failure {
    fn from(error: IsolateGone) -> Self {
        Self::new("bridgeFailed", error.to_string())
    }
}

/// El nombre en `camelCase` de una situación del destino.
fn destination_situation_name(situation: crate::destination::Situation) -> &'static str {
    use crate::destination::Situation as Where;
    match situation {
        Where::FolderMissing => "folderMissing",
        Where::NotAFolder => "notAFolder",
        Where::FolderUnreadable => "folderUnreadable",
        Where::NoFreeName => "noFreeName",
    }
}

impl From<crate::destination::DestinationError> for Failure {
    fn from(error: crate::destination::DestinationError) -> Self {
        Self::new(
            destination_situation_name(error.situation()),
            error.detail(),
        )
    }
}

impl From<cycle::CycleError> for Failure {
    fn from(error: cycle::CycleError) -> Self {
        match error {
            cycle::CycleError::Inadmissible(refusal) => refusal.into(),
            cycle::CycleError::Bridge(error) => error.into(),
            cycle::CycleError::Token(error) => error.into(),
            cycle::CycleError::Seal(error) => error.into(),
        }
    }
}

/// El estado de un certificado tal como cruza a la ventana.
///
/// Las cinco variantes llevan **su carga**, incluidas `notYetValid` y
/// `unreadable`: sin ellas, `refusalFor` en TypeScript acababa fabricando la
/// prosa del detalle («el DER no es un X.509 legible») justo en el hueco que el
/// ID-29 reserva al texto original crudo. El dato de verdad lo tiene Rust, que
/// es quien lee el DER.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StatusView {
    Valid,
    #[serde(rename_all = "camelCase")]
    Expired {
        not_after: u64,
    },
    #[serde(rename_all = "camelCase")]
    NotYetValid {
        not_before: u64,
    },
    Revoked {
        reason: String,
    },
    Unreadable {
        detail: String,
    },
}

impl From<CertificateStatus> for StatusView {
    fn from(status: CertificateStatus) -> Self {
        match status {
            CertificateStatus::Valid => Self::Valid,
            CertificateStatus::Expired { not_after } => Self::Expired { not_after },
            CertificateStatus::NotYetValid { not_before } => Self::NotYetValid { not_before },
            CertificateStatus::Revoked { reason } => Self::Revoked { reason },
            CertificateStatus::Unreadable { detail } => Self::Unreadable { detail },
        }
    }
}

/// Un certificado, con lo justo para pintar su fila y para volver a encontrarlo.
///
/// **No lleva el DER ni la ruta del módulo.** El DER es de quien lee X.509, que
/// es Rust; la ruta del módulo es del anfitrión. Lo que la ventana devuelve para
/// firmar es la `label`, y el backend reencuentra el resto.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateView {
    pub label: String,
    pub holder_name: String,
    pub id_number: String,
    pub issuer: String,
    pub status: StatusView,
}

/// El documento firmado, tal como la ventana lo cuenta: **dos nombres, ninguna
/// ruta** (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedDocumentView {
    /// El nombre del fichero resultante.
    pub name: String,
    /// El nombre de la carpeta donde quedó. No su ruta.
    pub folder: String,
}

/// Lo que la ventana ha marcado en las casillas del recuadro.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleFieldsOrder {
    pub signer_name: bool,
    pub id_number: bool,
    pub signed_at: bool,
    pub reason: bool,
}

/// Dónde ha caído el recuadro, tal como lo sabe el visor.
///
/// La `MediaBox` y la `/Rotate` las trae la ventana porque quien tiene abierto
/// el PDF es `pdf.js`: el backend **no lee PDFs**, y ponerle un analizador para
/// releer lo que el visor ya sabe sería una segunda opinión sobre la misma
/// página.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementOrder {
    /// Página **1-based**, como la numera `pdf.js` y como la cuenta
    /// `signaturePage`.
    pub page: u32,
    /// La `MediaBox` de esa página: `[x0, y0, x1, y1]`.
    pub media_box: [f64; 4],
    /// Su `/Rotate`, en grados.
    pub rotation: i32,
    /// El recuadro en espacio de usuario: `[x0, y0, x1, y1]`.
    pub rect: [i32; 4],
}

impl PlacementOrder {
    fn signature_box(&self) -> Result<SignatureBox, Failure> {
        let [x0, y0, x1, y1] = self.media_box;
        let rotation = Rotation::from_degrees(self.rotation).ok_or_else(|| {
            Failure::new(
                "unknown",
                format!("una pagina no puede estar girada {} grados", self.rotation),
            )
        })?;
        let page = Page {
            number: self.page,
            media_box: MediaBox::new(x0, y0, x1, y1),
            rotation,
        };
        let [left, bottom, right, top] = self.rect;
        page.signature_box(&UserSpaceRect {
            lower_left_x: left,
            lower_left_y: bottom,
            upper_right_x: right,
            upper_right_y: top,
        })
        .map_err(|out| Failure::new("boxOutOfPage", out.to_string()))
    }
}

/// La orden de firma completa: todo lo que distingue esta firma de otra.
///
/// `signed_at` llega **ya formateado** por la ventana, que es la que conoce el
/// huso y el formato de fecha del sistema, y es **el mismo** que se enseñó en
/// la vista previa: el recuadro se compone antes de la prefirma y el PDF ya no
/// se vuelve a tocar, así que enseñar una hora y estampar otra sería enseñar
/// algo que el PDF no va a tener.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningOrder {
    /// El asa que dio el portal al abrir el documento. Entra, no sale.
    pub document: String,
    /// La `label` del certificado elegido, de [`CertificateView`].
    pub certificate: String,
    /// Dónde cae el recuadro, en **espacio de usuario PDF** (ID-21).
    ///
    /// No en puntos PAdES: la inversa de la `/Rotate` que iText aplica al
    /// cerrar el documento la hace [`crate::signing::placement`], y con ella
    /// viene gratis la guardia del ID-22 —un recuadro que se saliera de la
    /// página iText **lo recorta en silencio** y la firma sale válida igual—.
    pub placement: PlacementOrder,
    pub fields: VisibleFieldsOrder,
    /// El motivo. Vacío es «sin motivo».
    pub reason: String,
    /// La fecha y hora, ya formateadas.
    pub signed_at: String,
    /// La rúbrica en JPEG y Base64, ya normalizada por [`crate::rubric`].
    pub rubric: Option<String>,
    /// El idioma en el que se componen las etiquetas del recuadro.
    pub language: String,
}

/// El ciclo a medias, entre el PIN y la postfirma.
///
/// Vive en el backend y **no cruza a la ventana**: lo que la ventana no tiene
/// no lo puede perder ni alterar, y el sello de sesión es justo lo que no puede
/// cambiar entre la prefirma y la postfirma (ADR-0016).
#[derive(Default)]
pub struct SigningSession {
    open: Mutex<Option<InFlight>>,
}

struct InFlight {
    cycle: OpenCycle,
    document: PortalDocument,
    signature: Option<TokenSignature>,
    /// El sello, transportado aparte del ciclo que lo emitió.
    ///
    /// Están separados a propósito: si el sello viviera solo dentro de
    /// [`OpenCycle`], compararlo consigo mismo no comprobaría nada. Esta es la
    /// copia que viaja, y [`OpenCycle::postsign`] exige que llegue idéntica.
    seal: SessionSeal,
}

/// Los almacenes de certificados y la carpeta de destino.
pub struct Environment {
    /// Dónde se buscan los certificados, en orden.
    ///
    /// Es una **colección** y no una ruta única (ID-03): un almacén que no
    /// cargue no puede dejar sin certificados a los demás. Los resuelve
    /// [`crate::pkcs11::stores::from_environment`] al arrancar.
    pub stores: Vec<std::path::PathBuf>,
    /// La carpeta de documentos del usuario, para cuando no haya destino
    /// elegido.
    pub documents_folder: std::path::PathBuf,
    /// Lo que se recuerda entre sesiones, ya leído.
    ///
    /// Es la copia viva: las órdenes de firma la consultan sin tocar el disco,
    /// y [`write_configuration`] la actualiza a la vez que la guarda. Tener
    /// solo el fichero obligaría a releerlo en cada firma; tener solo la copia
    /// perdería lo elegido al cerrar la ventana.
    pub configuration: Mutex<Configuration>,
    /// Los dos ficheros donde se recuerda. Ver [`crate::memory::Memory`].
    pub memory: Memory,
}

fn language_of(tag: &str) -> Language {
    match tag {
        "ca" => Language::Catalan,
        "eu" => Language::Basque,
        "gl" => Language::Galician,
        "va" => Language::Valencian,
        "en" => Language::English,
        _ => Language::Spanish,
    }
}

/// El valor de un atributo de un nombre distinguido, o la cadena vacía si no
/// está.
fn attribute(name: &str, distinguished_name: &str) -> String {
    distinguished_name
        .split(',')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(name).map(str::to_owned))
        .unwrap_or_default()
}

/// El titular y el DNI que se leen del **subject**, para el recuadro y para la
/// fila del panel.
fn holder_of(subject: Option<&str>) -> (String, String) {
    let subject = subject.unwrap_or_default();
    (
        attribute("CN=", subject),
        attribute("SERIALNUMBER=", subject),
    )
}

/// La autoridad emisora, tal como se enseña en el panel («Emitido por …»).
///
/// Sale del **issuer**, no del `O=` del subject: ese es la organización del
/// titular. Un certificado de persona física de la FNMT no lleva `O=` en el
/// subject —así que ahí el panel se quedaba en «Emitido por »— y uno de
/// empleado público sí, con el organismo del titular, que el panel afirmaba
/// que había emitido el certificado. El emisor es el dato con el que alguien
/// decide si se fía, y no admite un valor aproximado.
///
/// Se enseña el `CN=` del issuer, que es como se nombra a una autoridad («AC
/// FNMT Usuarios»); si no lo lleva se cae al `O=` del issuer, y si tampoco, al
/// nombre distinguido entero, que es feo pero cierto.
fn issuer_of(issuer: Option<&str>) -> String {
    let issuer = issuer.unwrap_or_default().trim();
    let common_name = attribute("CN=", issuer);
    if !common_name.is_empty() {
        return common_name;
    }
    let organisation = attribute("O=", issuer);
    if !organisation.is_empty() {
        return organisation;
    }
    issuer.to_owned()
}

/// **Orden 1.** Los certificados de los tokens conectados.
///
/// No pide el PIN: los certificados son objetos públicos y su estado se decide
/// leyendo el DER. Pedir el secreto que desbloquea la clave para luego decir
/// que el certificado caducó es hacerlo teclear para nada.
#[tauri::command]
pub fn list_certificates(
    environment: State<'_, Environment>,
) -> Result<Vec<CertificateView>, Failure> {
    let found = pkcs11::list_certificates_across(&environment.stores)?;
    Ok(found
        .into_iter()
        .map(|certificate| {
            let (holder_name, id_number) = holder_of(certificate.subject().as_deref());
            CertificateView {
                label: certificate.reference().label().to_owned(),
                holder_name,
                id_number,
                issuer: issuer_of(certificate.issuer().as_deref()),
                status: certificate.status().into(),
            }
        })
        .collect())
}

/// **Orden 2.** El texto del recuadro, ya compuesto, para la vista previa.
///
/// Es la **misma** función que compone lo que se envía en `layer2Text`
/// ([`compose_layer2_text`]), y por eso la vista previa es honesta: una copia
/// en TypeScript empezaría igual y divergiría en la primera esquina.
#[tauri::command]
pub fn compose_visible_text(
    order: SigningOrder,
    environment: State<'_, Environment>,
) -> Result<String, Failure> {
    // El titular sale del token, no de la orden: la ventana solo tiene la
    // etiqueta, y componer el recuadro con la etiqueta en vez del nombre
    // enseñaría una vista previa que el PDF no va a tener.
    let holder = holder_named(&order.certificate, &environment)?;
    Ok(layer2_text_of(&order, &holder))
}

/// El nombre y el DNI del certificado elegido, leídos del DER. **Sin PIN**:
/// los certificados son objetos públicos del token.
fn holder_named(label: &str, environment: &Environment) -> Result<(String, String), Failure> {
    let certificates = pkcs11::list_certificates_across(&environment.stores)?;
    let chosen = certificates
        .iter()
        .find(|certificate| certificate.reference().label() == label)
        .ok_or_else(|| {
            Failure::new(
                "certificateNotFound",
                format!("el token ya no tiene {label}"),
            )
        })?;
    Ok(holder_of(chosen.subject().as_deref()))
}

fn layer2_text_of(order: &SigningOrder, holder: &(String, String)) -> String {
    let (name, id) = holder;
    compose_layer2_text(
        &VisibleTextFields {
            signer_name: order.fields.signer_name.then_some(name.as_str()),
            id_number: order
                .fields
                .id_number
                .then_some(id.as_str())
                .filter(|id| !id.is_empty()),
            signed_at: order.fields.signed_at.then_some(order.signed_at.as_str()),
            reason: order
                .fields
                .reason
                .then_some(order.reason.as_str())
                .filter(|reason| !reason.is_empty()),
        },
        language_of(&order.language),
    )
}

/// El certificado que pide la orden, si sigue estando y sirve para firmar.
///
/// Se mira el estado **otra vez** aunque la ventana ya lo mirara al listar, y
/// no sobra: entre listar y firmar puede haberse retirado la tarjeta o haber
/// pasado la medianoche del `notAfter`. Es la última comprobación antes del
/// PIN, y la única que ve el token de ahora mismo.
fn usable_certificate<'a>(
    certificates: &'a [TokenCertificate],
    label: &str,
) -> Result<&'a TokenCertificate, Failure> {
    let chosen = certificates
        .iter()
        .find(|certificate| certificate.reference().label() == label)
        .ok_or_else(|| {
            Failure::new(
                "certificateNotFound",
                format!("el token ya no tiene {label}"),
            )
        })?;
    let status = chosen.status();
    if !status.is_usable() {
        return Err(Failure::new(
            "certificateNotFound",
            format!("{label}: {status:?}"),
        ));
    }
    Ok(chosen)
}

/// La configuración de firma que salen de la orden y del certificado elegido.
///
/// El nombre y el DNI se leen **del DER**, no de la orden: la ventana solo
/// manda la etiqueta, y componer el recuadro con lo que la ventana diga sería
/// dejar que estampe cualquier nombre.
fn config_for(order: &SigningOrder, chosen: &TokenCertificate) -> Result<SignatureConfig, Failure> {
    let (name, id) = holder_of(chosen.subject().as_deref());
    Ok(SignatureConfig {
        signature_box: order.placement.signature_box()?,
        layer2_text: layer2_text_of(order, &(name, id)),
        rubric_image: order.rubric.clone(),
        // Un motivo vacío **no se envía**: `signReason` con la cadena vacía
        // estampa una etiqueta «Motivo:» sin nada detrás.
        sign_reason: (!order.reason.is_empty()).then(|| order.reason.clone()),
    })
}

/// Se lleva el ciclo a medias de la sesión, exigiendo que el token ya haya
/// firmado.
///
/// **Se lo lleva, no lo copia**: al salir de aquí la sesión queda vacía, así
/// que una postfirma que falle no deja un ciclo colgando que un segundo intento
/// pudiera reusar con otro sello. El ciclo se reabre desde la prefirma o no se
/// reabre.
fn take_signed_cycle(
    session: &SigningSession,
) -> Result<(OpenCycle, PortalDocument, TokenSignature, SessionSeal), Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.take().ok_or_else(no_open_cycle)?;
    let signature = in_flight
        .signature
        .ok_or_else(|| Failure::new("unknown", "todavía no se ha firmado en el token"))?;
    Ok((
        in_flight.cycle,
        in_flight.document,
        signature,
        in_flight.seal,
    ))
}

/// Cómo se cuenta un documento firmado: **dos nombres, ninguna ruta**
/// (ADR-0011).
fn told_as(landing: &std::path::Path, folder: &CheckedFolder) -> SignedDocumentView {
    SignedDocumentView {
        name: landing
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned(),
        folder: folder.name().to_owned(),
    }
}

/// Deja caer el documento firmado en la carpeta de destino, **sin diálogo**
/// (ID-36, ADR-0011).
///
/// Lo único que se elige es la carpeta, y se eligió una vez. El nombre lo
/// resuelve [`CheckedFolder::landing_for`], que numera los homónimos: sin
/// diálogo por firma no hay ningún «ya existe, ¿reemplazar?» que avise, así que
/// sin esa numeración la segunda firma machacaría a la primera en silencio.
fn deliver(
    environment: &Environment,
    document: &PortalDocument,
    signed: &[u8],
) -> Result<SignedDocumentView, Failure> {
    let chosen = {
        let configuration = lock(&environment.configuration);
        crate::destination::chosen_folder(&configuration, environment.documents_folder.clone())
    };
    // La carpeta se comprueba y **no se crea nunca** (ID-38): bajo el arenero
    // crearla contesta OK y no deja nada en el anfitrión.
    let folder = CheckedFolder::check(&chosen)?;
    let landing = folder.landing_for(document)?;
    std::fs::write(&landing, signed)
        .map_err(|error| Failure::new("folderUnwritable", error.to_string()))?;
    Ok(told_as(&landing, &folder))
}

/// Lo que hay que saber del token y de la orden antes de cruzar la frontera.
///
/// Junta las dos preguntas que se le hacen al token —qué certificado es, y si
/// todavía sirve— con la configuración que sale de él, porque las tres son la
/// misma decisión: **con qué se firma**.
fn plan_signature(
    stores: &[std::path::PathBuf],
    order: &SigningOrder,
) -> Result<(SignatureConfig, CertificateRef, Vec<Vec<u8>>), Failure> {
    let certificates = pkcs11::list_certificates_across(stores)?;
    let chosen = usable_certificate(&certificates, &order.certificate)?;
    Ok((
        config_for(order, chosen)?,
        chosen.reference().clone(),
        vec![chosen.der().to_vec()],
    ))
}

/// Los bytes del documento, ya admitidos.
///
/// La puerta rápida del #60: se decide sobre los bytes, sin token y sin
/// frontera, y por eso puede caer **antes del diálogo del PIN**.
fn admitted_bytes(document: &PortalDocument) -> Result<Vec<u8>, Failure> {
    let bytes = std::fs::read(document.reading_path())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    AdmissibleDocument::check(&bytes)?;
    Ok(bytes)
}

/// Aplana las tres capas de resultado que devuelve un trabajo del isolate: el
/// hilo puede haberse caído, la librería puede no haber abierto, y el ciclo
/// puede haber fallado.
fn on_the_bridge<T: Send + 'static>(
    isolate: &Isolate,
    task: impl FnOnce(&crate::ffi::NativeBridge) -> Result<T, cycle::CycleError> + Send + 'static,
) -> Result<T, Failure> {
    match isolate.run(task) {
        Err(gone) => Err(gone.into()),
        Ok(Err(bridge)) => Err(bridge.into()),
        Ok(Ok(outcome)) => outcome.map_err(Failure::from),
    }
}

/// **Orden 3.** Prefirma: cruza la frontera y deja el ciclo abierto.
///
/// Antes de nada rechaza lo que no se puede firmar —cifrado, certificado, o no
/// es un PDF—, **antes de que se pida el PIN**.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    opened: State<'_, OpenedDocuments>,
) -> Result<(), Failure> {
    // Lo que la ventana manda es el identificador que se acuñó al abrir, y no
    // una ruta: quien sabe a qué documento del portal corresponde es el
    // registro, y solo él (ID-62).
    let document = opened_document(&opened, &order.document)?;
    let bytes = admitted_bytes(&document)?;
    let (config, reference, chain) = plan_signature(&environment.stores, &order)?;

    let cycle = on_the_bridge(&isolate, move |bridge| {
        // La comprobación se repite dentro del hilo porque el tipo que la
        // garantiza presta los bytes y los bytes viajan: no es un `if`
        // olvidable, es el único constructor de `AdmissibleDocument`.
        let document = AdmissibleDocument::check(&bytes)?;
        cycle::presign(
            bridge,
            SigningRequest {
                document,
                chain: &chain,
                config: &config,
                certificate: &reference,
            },
        )
    })?;

    let seal = cycle.seal_in_transit();
    *lock(&session.open) = Some(InFlight {
        cycle,
        document,
        signature: None,
        seal,
    });
    Ok(())
}

/// **Orden 4.** Firma en el token, con el PIN que se acaba de teclear.
///
/// **La única fase que toca la clave privada, y no cruza la FFI** (ADR-0001).
/// El PIN entra por aquí, se usa en `C_Login` y no se guarda en ningún sitio:
/// ni en la sesión, ni en el registro, ni de vuelta a la ventana.
#[tauri::command]
pub fn sign_with_pin(pin: String, session: State<'_, SigningSession>) -> Result<(), Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.as_mut().ok_or_else(no_open_cycle)?;
    in_flight.signature = Some(in_flight.cycle.sign_on_token(&pin)?);
    Ok(())
}

/// **Orden 5.** Postfirma: comprueba el sello, ensambla el PDF y lo deja caer.
///
/// El documento cae **sin diálogo** (ID-36, ADR-0011): lo único que se elige es
/// la carpeta, y se eligió una vez.
#[tauri::command]
pub fn finish_signing(
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
) -> Result<SignedDocumentView, Failure> {
    let (cycle, document, signature, seal) = take_signed_cycle(&session)?;

    let signed = on_the_bridge(&isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    deliver(&environment, &document, &signed)
}

/// **Orden 6.** Cancelar: se olvida el ciclo a medias.
///
/// Existe porque un ciclo abierto que no se cierra deja el sello y los bytes a
/// firmar vivos en memoria hasta que se cierre la ventana.
#[tauri::command]
pub fn cancel_signing(session: State<'_, SigningSession>) {
    *lock(&session.open) = None;
}

/// Un documento abierto, tal como la ventana lo recibe: **un identificador y un
/// nombre, ninguna ruta** (ID-60, ADR-0011).
///
/// El `modified` sale de aquí y no lo calcula la ventana porque quien tocó el
/// disco es el backend: la fila de la bandeja se pinta con metadatos cacheados
/// y sin volver a abrir el fichero (ADR-0010).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocumentView {
    /// El identificador opaco que acuñó [`OpenedDocuments`].
    pub id: String,
    /// El nombre del fichero. No su ruta.
    pub name: String,
    /// El `mtime`, en segundos desde la época; `None` si no se pudo leer.
    pub modified: Option<u64>,
}

/// Dónde se abre el diálogo de abrir: **la última carpeta usada**, y si no se
/// sabe, la de destino.
///
/// Las dos mitades de la frase importan, porque no todos los canales saben lo
/// mismo:
///
/// - **Fuera del arenero** —deb, rpm, Windows, macOS— el diálogo devuelve una
///   ruta de verdad, así que la carpeta de la que salió el documento se sabe y
///   se recuerda. Lo hace [`remembered_folder`], y vive en el estado, no en la
///   configuración: la acumula la aplicación sola.
/// - **Bajo el arenero no se puede saber**, y no hay forma de arreglarlo con
///   más código: lo que el portal devuelve es
///   `/run/user/1000/doc/<id>/nombre.pdf`, cuyo directorio padre tiene un solo
///   fichero dentro y no es ninguna carpeta del usuario; preguntar por la real
///   —`org.freedesktop.portal.Documents.Info` y `.Lookup`— contesta
///   `Not allowed in sandbox`, y `--filesystem=home` tampoco la devolvería.
///   Medido en `docs/research/flatpak-canal-unico.md`, apartado 4.
///
/// El respaldo para ese caso es la **carpeta de destino**, la de Preferencias:
/// la única carpeta del usuario que la aplicación conoce y nombra en el
/// flatpak. Resuelve lo que se quería de verdad —no empezar cada vez en la
/// lista de «Recientes» del sistema— y además deja a la vista lo ya firmado.
///
/// Esto **no es una ruta donde escribir** y no puede llegar a serlo: lo único
/// que la recibe es `set_directory`, y la única forma de nombrar un sitio
/// donde cae un fichero sigue siendo [`CheckedFolder::landing_for`]
/// (ADR-0011).
///
/// Devuelve `None` si no queda ninguna de las dos, y entonces el diálogo se
/// abre donde el sistema quiera: [`CheckedFolder`] solo mira, **nunca crea**
/// (ID-38).
fn starting_folder(environment: &Environment) -> Option<PathBuf> {
    if let Some(remembered) = remembered_folder(environment) {
        return Some(remembered);
    }
    let configuration = lock(&environment.configuration).clone();
    let folder = crate::destination::chosen_folder(&configuration, &environment.documents_folder);
    CheckedFolder::check(&folder)
        .ok()
        .map(|checked| checked.path().to_path_buf())
}

/// La última carpeta apuntada, **si sigue estando ahí**.
///
/// Se comprueba porque una carpeta que se borró o que estaba en un disco que
/// ya no está montado no es un punto de partida: es un diálogo que se abre en
/// un sitio que no existe. Bajo el arenero esto es siempre `None`, y también
/// con «Recordar mi actividad» apagado, porque entonces no hay fichero de
/// estado que leer.
fn remembered_folder(environment: &Environment) -> Option<PathBuf> {
    environment
        .memory
        .state()
        .ok()?
        .into_value()
        .last_open_folder
        .filter(|folder| folder.is_dir())
}

/// Apunta de dónde salió el documento, **cuando se puede saber**.
///
/// Es lo mejor posible en cada canal y a propósito: donde el diálogo devuelve
/// una ruta de verdad, la próxima vez se abre justo ahí; donde devuelve un
/// enlace del portal, [`folder_it_came_from`] contesta `None` y no se apunta
/// nada. Un fallo al escribir el estado **no impide abrir el documento**:
/// recordar la carpeta es una comodidad, y perderla no puede costar el
/// recorrido.
fn remember_the_folder(environment: &Environment, document: &PortalDocument) {
    let Some(folder) = folder_it_came_from(document) else {
        return;
    };
    let configuration = lock(&environment.configuration).clone();
    let Ok(loaded) = environment.memory.state() else {
        return;
    };
    let mut state = loaded.into_value();
    if state.last_open_folder.as_deref() == Some(folder) {
        return;
    }
    state.last_open_folder = Some(folder.to_path_buf());
    let _ = environment.memory.remember_state(&configuration, &state);
}

/// La carpeta de la que salió el documento, o `None` si entró por el portal.
///
/// El `None` **no es una precaución, es la verdad**: el directorio padre de un
/// enlace del portal contiene ese solo fichero y no dice nada de dónde está el
/// original. Apuntarlo abriría el diálogo la próxima vez en un directorio del
/// arenero que para entonces ni existe.
///
/// Vive aquí y no en [`PortalDocument`] para no darle a ese tipo un método que
/// devuelva un directorio: que no lo tenga es lo que impide que «guardar junto
/// al original» se cuele por la puerta de atrás (ADR-0011).
fn folder_it_came_from(document: &PortalDocument) -> Option<&std::path::Path> {
    if document.came_through_the_portal() {
        return None;
    }
    document.reading_path().parent()
}

/// **Orden 7.** Abre el diálogo del sistema y apunta lo que el portal conceda.
///
/// El diálogo se abre **desde aquí y no desde el frontal** (ID-63): así la
/// ventana sigue con un solo fichero que conoce `invoke`, y la lista de
/// permisos de `capabilities/default.json` no crece, porque los permisos de
/// Tauri v2 vigilan lo que la ventana puede pedir y no lo que Rust hace.
/// Filtra por PDF porque es lo único que la aplicación sabe firmar (ID-64).
///
/// Cerrar el diálogo sin elegir nada devuelve `None`, que **no es un fallo**:
/// es lo que deja el documento activo, la lista y el visor como estaban
/// (ID-73).
///
/// El diálogo se abre en la última carpeta usada, y donde esa no se puede
/// saber, en la de destino: ver [`starting_folder`].
#[tauri::command(async)]
pub fn open_document(
    app: tauri::AppHandle,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<Option<OpenedDocumentView>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let mut dialog = app.dialog().file().add_filter("PDF", &["pdf"]);
    if let Some(folder) = starting_folder(&environment) {
        dialog = dialog.set_directory(folder);
    }
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let handle = chosen
        .into_path()
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    let document = PortalDocument::opened(handle);
    remember_the_folder(&environment, &document);
    let name = document.name().to_owned();
    let modified = modified_seconds(&document);
    Ok(Some(OpenedDocumentView {
        id: opened.remember(document),
        name,
        modified,
    }))
}

/// **Orden 8.** Los bytes del documento abierto, **como bytes** (ID-66).
///
/// Devuelve una [`tauri::ipc::Response`] y no un `Vec<u8>`: serializado a JSON,
/// un PDF de unos pocos megabytes se convierte en un array de miles de números
/// y multiplica el tamaño y el tiempo. Esta es la respuesta binaria que el
/// puente de Tauri ofrece justo para esto, y al otro lado llega un
/// `ArrayBuffer` que `pdf.js` abre sin nada en medio.
#[tauri::command(async)]
pub fn read_document(
    id: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    let document = opened_document(&opened, &id)?;
    let bytes = std::fs::read(document.reading_path())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// La configuración, tal como la ventana la ve: **ningún `PathBuf`**.
///
/// El destino sale por su [`nombre`](crate::memory::DestinationFolder::name) y
/// nunca por su ruta, igual que todo lo demás que cruza (ADR-0011). Y va en un
/// solo sentido de verdad: la ventana **no elige la carpeta** —bajo el arenero
/// hay una y solo una—, así que el destino que llegue en una escritura se
/// ignora. Está aquí para pintarlo, no para cambiarlo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationView {
    /// La etiqueta corta del idioma: `es`, `ca`, `eu`, `gl`, `va` o `en`.
    pub language: String,
    /// El **nombre** de la carpeta de destino. Nunca su ruta.
    pub destination: String,
    /// «Recordar la última configuración de firma visible».
    pub remember_visible_signature: bool,
    /// «Recordar mi actividad».
    pub remember_activity: bool,
    /// El tema de la ventana. Ver [`Theme`].
    pub theme: Theme,
}

/// Cómo se ve desde la ventana la configuración que hay guardada.
fn shown(configuration: &Configuration, documents_folder: &std::path::Path) -> ConfigurationView {
    let folder = crate::destination::chosen_folder(configuration, documents_folder.to_path_buf());
    ConfigurationView {
        language: configuration.language.tag().to_owned(),
        destination: folder.name().to_owned(),
        remember_visible_signature: configuration.remember_visible_signature,
        remember_activity: configuration.remember_activity,
        theme: configuration.theme,
    }
}

/// **Orden 9.** Lo que hay guardado, para pintar Preferencias al abrir.
///
/// Lee de la copia viva y no del disco: el fichero se leyó una vez al arrancar
/// (`lib.rs`), y volver a leerlo aquí abriría la puerta a que la ventana y las
/// órdenes de firma vieran configuraciones distintas.
#[tauri::command]
pub fn read_configuration(environment: State<'_, Environment>) -> ConfigurationView {
    let configuration = lock(&environment.configuration);
    shown(&configuration, &environment.documents_folder)
}

/// **Orden 10.** Guarda lo que el usuario acaba de elegir.
///
/// Actualiza la copia viva **y** el fichero, en ese orden, y las dos cosas o
/// ninguna: si la escritura falla, la copia se deja como estaba, porque una
/// ventana que enseña un ajuste que el disco no tiene miente en la sesión
/// siguiente.
///
/// El borrado del estado al apagar «Recordar mi actividad» **no está aquí**:
/// lo hace [`Memory::remember_configuration`](crate::memory::Memory), que es
/// donde no se puede olvidar (ADR-0010).
#[tauri::command(async)]
pub fn write_configuration(
    configuration: ConfigurationView,
    environment: State<'_, Environment>,
) -> Result<(), Failure> {
    let mut live = lock(&environment.configuration);
    let next = merged(&live, &configuration);
    environment.memory.remember_configuration(&next)?;
    *live = next;
    Ok(())
}

/// Lo elegido, encima de lo guardado.
///
/// Vive aparte de la orden porque es la única decisión que hay dentro —qué
/// campos manda la ventana y cuáles no— y así se puede comprobar sin montar un
/// entorno de Tauri.
fn merged(live: &Configuration, chosen: &ConfigurationView) -> Configuration {
    Configuration {
        language: language_of(&chosen.language),
        // El destino no viaja de vuelta: la ventana lo enseña, no lo elige.
        destination: live.destination.clone(),
        remember_visible_signature: chosen.remember_visible_signature,
        remember_activity: chosen.remember_activity,
        theme: chosen.theme,
    }
}

/// **Orden 11.** Olvida lo acumulado: los recientes y el certificado.
///
/// Es «Vaciar la lista» y también lo que arrastra apagar «Recordar mi
/// actividad» (ID-34): las dos son la misma promesa y por eso son la misma
/// orden.
#[tauri::command(async)]
pub fn forget_activity(environment: State<'_, Environment>) -> Result<(), Failure> {
    environment.memory.forget_activity()?;
    Ok(())
}

/// El nombre del evento con el que la ventana se entera de un arrastre.
///
/// Es un **evento** y no una novena orden a propósito: el arrastre no lo pide
/// la ventana, le ocurre. En Tauri v2 el arrastre y la soltura del WebView
/// vienen desactivados por omisión a favor del evento nativo (ID-67), así que
/// un manejador de soltura en el JSX no se dispararía nunca; lo que hay debajo
/// es esto, y al otro lado lo recoge el puerto `DocumentDrops`.
pub const DOCUMENT_DROPPED: &str = "document-dropped";

/// Lo que la ventana recibe al soltar ficheros encima.
///
/// **Ninguna ruta** (ADR-0011). Lo que se suelta son rutas del anfitrión, y
/// justamente por eso la decisión de cuál se abre se toma aquí: lo que cruza es
/// el documento ya apuntado, con su identificador opaco, igual que si se
/// hubiera elegido por el diálogo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedDocumentView {
    /// El documento que se ha abierto, o `None` si no se ha abierto ninguno.
    pub document: Option<OpenedDocumentView>,
    /// Por qué no se ha abierto ninguno. `None` cuando sí se abrió.
    pub failure: Option<Failure>,
    /// Cuántos ficheros más venían en el mismo gesto y no se han abierto: la
    /// aplicación firma de uno en uno y lo dice (ID-70).
    pub ignored: usize,
}

/// Decide qué hacer con lo que se ha soltado y lo apunta si se puede abrir.
///
/// Es el adaptador entre [`crate::dropped::first_pdf`], que es quien decide, y
/// lo que la ventana entiende. Devuelve `None` cuando no se ha soltado nada:
/// entonces no hay nada que contar y no se emite ningún evento.
pub fn dropped_document(
    paths: &[std::path::PathBuf],
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    match crate::dropped::first_pdf(paths) {
        crate::dropped::Dropped::Nothing => None,
        crate::dropped::Dropped::Opened { path, ignored } => {
            let document = PortalDocument::opened(path);
            let name = document.name().to_owned();
            let modified = modified_seconds(&document);
            Some(DroppedDocumentView {
                document: Some(OpenedDocumentView {
                    id: opened.remember(document),
                    name,
                    modified,
                }),
                failure: None,
                ignored,
            })
        }
        crate::dropped::Dropped::NotAPdf { ignored } => Some(DroppedDocumentView {
            document: None,
            failure: Some(Failure::from(Refusal::NotAPdf)),
            ignored,
        }),
        // El aviso que el ID-68 exige: no es «ha fallado» a secas, es una
        // situación propia cuyo texto dice qué hacer —usar el botón de abrir,
        // que sí pasa por el portal—. Por qué existe este caso y desde qué
        // carpetas ocurre está medido en
        // `docs/research/arrastre-bajo-el-arenero.md`.
        crate::dropped::Dropped::Unreadable { detail, ignored } => Some(DroppedDocumentView {
            document: None,
            failure: Some(Failure::new("droppedFileUnreadable", detail)),
            ignored,
        }),
    }
}

/// El documento que se abrió con ese identificador.
///
/// Que no esté apuntado no es un fallo del programa: se cuenta como un
/// documento que no se puede leer, que es lo que la ventana sabe enseñar.
fn opened_document(opened: &OpenedDocuments, id: &str) -> Result<PortalDocument, Failure> {
    opened.get(id).ok_or_else(|| {
        Failure::new(
            "documentUnreadable",
            "el documento ya no esta abierto en esta sesion",
        )
    })
}

/// El `mtime` del documento, en segundos desde la época.
fn modified_seconds(document: &PortalDocument) -> Option<u64> {
    std::fs::metadata(document.reading_path())
        .and_then(|metadata| metadata.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

fn no_open_cycle() -> Failure {
    Failure::new("unknown", "no hay ninguna firma empezada")
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{
        attribute, config_for, deliver, dropped_document, folder_it_came_from, holder_of,
        issuer_of, language_of, merged, remember_the_folder, shown, situation_name,
        starting_folder, take_signed_cycle, told_as, usable_certificate, CertificateView,
        CheckedFolder, Configuration, ConfigurationView, Environment, Failure, Mutex,
        OpenedDocumentView, OpenedDocuments, PlacementOrder, PortalDocument, SignedDocumentView,
        SigningOrder, SigningSession, StatusView, Theme, VisibleFieldsOrder,
    };
    use crate::pkcs11::{CertificateRef, CertificateStatus, Situation, TokenCertificate};
    use crate::signing::Language;

    /// **Grada A**: lo que se comprueba aquí es la **forma** de lo que sale por
    /// las órdenes, que es lo que la ventana lee. El ciclo contra el token y
    /// `pdfsig` es la grada C de `tests/native_cycle.rs`.
    const SOURCE: &str = include_str!("mod.rs");

    /// La mitad de producción, sin las pruebas: si no, esta comprobación se
    /// leería a sí misma y encontraría siempre sus propios literales.
    fn production_half() -> &'static str {
        SOURCE
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(SOURCE)
    }

    #[test]
    fn no_output_of_any_command_carries_a_host_path() {
        // Bajo el arenero la aplicación no conoce la ruta real de un documento,
        // así que devolver una sería devolver una mentira (ADR-0011). Lo que
        // sale son nombres.
        let outputs = ["struct CertificateView", "struct SignedDocumentView"];
        for output in outputs {
            let body = production_half()
                .split_once(output)
                .expect("el tipo de salida sigue aquí")
                .1
                .split_once('}')
                .expect("y tiene cuerpo")
                .0;
            for leak in ["PathBuf", "&Path", "path:", "module:", "reading_path"] {
                assert!(
                    !body.contains(leak),
                    "«{output}» ha ganado un «{leak}»: eso es una ruta del anfitrión saliendo"
                );
            }
        }
    }

    #[test]
    fn the_pin_is_taken_and_never_given_back() {
        // Entra por `sign_with_pin`, se usa en el token y no se guarda: ni en
        // la sesión a medias, ni en ningún tipo de salida.
        let source = production_half();
        let session = source
            .split_once("struct InFlight {")
            .expect("la sesión sigue aquí")
            .1
            .split_once("\n}")
            .expect("y tiene cuerpo")
            .0;

        assert!(
            !session.contains("pin"),
            "el PIN se está guardando: {session}"
        );
        assert_eq!(
            source.matches("pin: String").count(),
            1,
            "el PIN entra por una sola orden"
        );
    }

    #[test]
    fn the_seal_travels_apart_from_the_cycle_that_issued_it() {
        // Si el sello viviera solo dentro de `OpenCycle`, compararlo consigo
        // mismo no comprobaría nada y el ADR-0016 sería un comentario.
        let session = production_half()
            .split_once("struct InFlight {")
            .expect("la sesión sigue aquí")
            .1;

        assert!(session.contains("seal: SessionSeal"));
    }

    /// La misma guardia del ADR-0011, sobre lo que las **dos órdenes nuevas**
    /// devuelven. Va aparte de
    /// [`no_output_of_any_command_carries_a_host_path`] a propósito: aquella es
    /// la prueba que el spec exige que siga verde **sin tocarla** (TD-19), y
    /// ampliarle la lista habría sido tocarla.
    #[test]
    fn the_opened_document_that_crosses_carries_no_host_path() {
        let body = production_half()
            .split_once("struct OpenedDocumentView")
            .expect("el tipo de salida sigue aqui")
            .1
            .split_once('}')
            .expect("y tiene cuerpo")
            .0;

        for leak in ["PathBuf", "&Path", "path:", "module:", "reading_path"] {
            assert!(
                !body.contains(leak),
                "«OpenedDocumentView» ha ganado un «{leak}»: eso es una ruta del anfitrion saliendo"
            );
        }
    }

    #[test]
    fn an_opened_document_is_told_with_an_identifier_and_a_name() {
        let view = OpenedDocumentView {
            id: "0f1e2d3c4b5a69788796a5b4c3d2e1f0".to_owned(),
            name: "contrato.pdf".to_owned(),
            modified: Some(1_700_000_000),
        };

        let json = serde_json::to_string(&view).expect("serializa");

        assert_eq!(
            json,
            r#"{"id":"0f1e2d3c4b5a69788796a5b4c3d2e1f0","name":"contrato.pdf","modified":1700000000}"#
        );
        assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
    }

    /// El identificador cruza; la ruta que hay detrás se queda en el registro.
    #[test]
    fn the_identifier_crosses_and_the_reading_path_stays_behind() {
        let opened = OpenedDocuments::new();
        let handle = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

        let id = opened.remember(PortalDocument::opened(handle));

        assert!(
            !id.contains("1e8b83b9"),
            "el identificador no lleva el del portal: {id}"
        );
        assert!(!id.contains("contrato"), "ni el nombre: {id}");
        assert_eq!(
            opened
                .get(&id)
                .map(|document| document.reading_path().to_owned()),
            Some(std::path::PathBuf::from(handle)),
            "y el backend sí sabe por dónde leerlo"
        );
    }

    /// La misma guardia, sobre lo que cruza al soltar. Importa más aquí que en
    /// ningún otro sitio: el arrastre es el **único** camino por el que a la
    /// aplicación le llega una ruta del anfitrión de verdad, así que este es
    /// justo el tipo por el que se escaparía (ADR-0011).
    #[test]
    fn the_dropped_document_that_crosses_carries_no_host_path() {
        let body = production_half()
            .split_once("struct DroppedDocumentView")
            .expect("el tipo de salida sigue aqui")
            .1
            .split_once("\n}")
            .expect("y tiene cuerpo")
            .0;

        for leak in ["PathBuf", "&Path", "path:", "module:", "reading_path"] {
            assert!(
                !body.contains(leak),
                "«DroppedDocumentView» ha ganado un «{leak}»: eso es una ruta del anfitrion saliendo"
            );
        }
    }

    /// Soltar un PDF legible acaba igual que elegirlo por el diálogo: un
    /// documento apuntado, con su identificador opaco y su nombre.
    #[test]
    fn a_dropped_pdf_crosses_as_an_opened_document() {
        let opened = OpenedDocuments::new();
        let pdf = std::env::temp_dir().join("rfirma-commands-soltado.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n").expect("se puede escribir en el temporal");

        let view = dropped_document(&[pdf], &opened).expect("algo se ha soltado");

        let document = view.document.expect("y se ha abierto");
        assert_eq!(document.name, "rfirma-commands-soltado.pdf");
        assert_eq!(document.id.len(), 32);
        assert_eq!(view.failure, None);
        assert_eq!(view.ignored, 0);
        assert_eq!(opened.len(), 1);
    }

    /// Y lo que no es un PDF no apunta nada: se cuenta con la misma situación
    /// con la que se rechaza al firmar, que ya está en los seis catálogos.
    #[test]
    fn dropping_something_that_is_not_a_pdf_opens_nothing_and_says_so() {
        let opened = OpenedDocuments::new();
        let other = std::env::temp_dir().join("rfirma-commands-soltado.ods");

        let view = dropped_document(&[other], &opened).expect("algo se ha soltado");

        assert!(view.document.is_none());
        assert_eq!(
            view.failure.map(|failure| failure.situation),
            Some("notAPdf".to_owned())
        );
        assert!(opened.is_empty(), "no se apunta lo que no se abre");
    }

    /// El aviso del ID-68 tiene **situación propia**: `documentUnreadable`
    /// dice «comprueba que sigue donde estaba», y aquí el fichero está donde
    /// estaba —lo que falta es la concesión—, así que ese texto mandaría a
    /// mirar lo que no es. El suyo dice qué hacer: usar el botón de abrir.
    #[test]
    fn a_dropped_file_the_sandbox_cannot_read_names_its_own_situation() {
        let opened = OpenedDocuments::new();
        let unreachable = std::env::temp_dir().join("rfirma-commands-no-existe/contrato.pdf");

        let view = dropped_document(&[unreachable], &opened).expect("algo se ha soltado");

        let failure = view.failure.expect("se cuenta como un fallo con nombre");
        assert_eq!(failure.situation, "droppedFileUnreadable");
        assert!(!failure.detail.is_empty(), "con su detalle crudo (ID-29)");
    }

    /// Soltar nada no es un suceso que contar, así que no se emite nada.
    #[test]
    fn dropping_no_files_at_all_says_nothing() {
        assert_eq!(dropped_document(&[], &OpenedDocuments::new()), None);
    }

    /// La lista sigue cerrada (ID-59): ocho órdenes más las tres de los
    /// ajustes.
    ///
    /// Cuenta el prefijo `#[tauri::command` y no el atributo entero porque
    /// varias llevan `(async)`: lo que se cierra es cuántas órdenes hay, no
    /// cómo se ejecuta cada una.
    #[test]
    fn the_list_of_commands_grew_to_eleven_and_no_further() {
        assert_eq!(
            production_half().matches("#[tauri::command").count(),
            11,
            "la lista de ordenes es cerrada a proposito"
        );
    }

    /// Y las dos que hablan con el disco o con el portal **no bloquean el hilo
    /// principal**: `#[tauri::command]` a secas genera un cuerpo `Blocking`
    /// que corre dentro del manejador del IPC —el hilo del bucle GTK—, y
    /// `blocking_pick_file()` espera allí a un cierre que solo ese hilo puede
    /// ejecutar. Punto muerto: la ventana se clava y el diálogo no aparece.
    #[test]
    fn the_two_commands_that_touch_the_portal_run_off_the_main_thread() {
        for command in ["pub fn open_document(", "pub fn read_document("] {
            let source = production_half();
            let declaration = source
                .find(command)
                .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
            let before = &source[..declaration];
            assert!(
                before.ends_with("#[tauri::command(async)]\n"),
                "«{command}» tiene que ser #[tauri::command(async)]"
            );
        }
    }

    #[test]
    fn every_token_situation_has_a_camel_case_name_for_the_catalogue() {
        let all = [
            Situation::IncorrectPin,
            Situation::PinLocked,
            Situation::TokenAbsent,
            Situation::ExpiredSession,
            Situation::ModuleNotFound,
            Situation::CertificateNotFound,
            Situation::Unknown,
        ];
        for situation in all {
            let name = situation_name(situation);
            assert!(!name.is_empty());
            assert!(
                !name.contains('_') && name.chars().next().is_some_and(char::is_lowercase),
                "«{name}» no está en camelCase"
            );
        }
    }

    #[test]
    fn a_failure_keeps_the_raw_detail_of_the_token() {
        let failure: Failure = crate::pkcs11::TokenError::new(
            Situation::CertificateNotFound,
            "el token no tiene ninguna clave privada etiquetada X",
        )
        .into();

        assert_eq!(failure.situation, "certificateNotFound");
        assert_eq!(
            failure.detail,
            "el token no tiene ninguna clave privada etiquetada X"
        );
    }

    #[test]
    fn the_status_crosses_with_its_payload() {
        // Los dos cabos del #76: sin la carga, `refusalFor` en TypeScript
        // fabricaba la prosa del detalle en el hueco que el ID-29 reserva al
        // texto original crudo.
        let not_yet = StatusView::from(CertificateStatus::NotYetValid { not_before: 42 });
        let unreadable = StatusView::from(CertificateStatus::Unreadable {
            detail: "PEM error".to_owned(),
        });

        assert_eq!(
            serde_json::to_string(&not_yet).expect("serializa"),
            r#"{"kind":"notYetValid","notBefore":42}"#
        );
        assert_eq!(
            serde_json::to_string(&unreadable).expect("serializa"),
            r#"{"kind":"unreadable","detail":"PEM error"}"#
        );
    }

    #[test]
    fn a_signed_document_is_told_with_two_names() {
        let view = SignedDocumentView {
            name: "contrato_signed.pdf".to_owned(),
            folder: "Documentos".to_owned(),
        };

        assert_eq!(
            serde_json::to_string(&view).expect("serializa"),
            r#"{"name":"contrato_signed.pdf","folder":"Documentos"}"#
        );
    }

    #[test]
    fn a_certificate_crosses_without_its_der_and_without_its_module() {
        let view = CertificateView {
            label: "ETIQUETA".to_owned(),
            holder_name: "Ada Lovelace Byron".to_owned(),
            id_number: "IDCES-00000000T".to_owned(),
            issuer: "FNMT-RCM".to_owned(),
            status: StatusView::Valid,
        };
        let json = serde_json::to_string(&view).expect("serializa");

        assert!(json.contains(r#""holderName":"Ada Lovelace Byron""#));
        assert!(!json.contains(r#""der""#), "el DER no sale: {json}");
        assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
    }

    #[test]
    fn reads_the_holder_and_the_id_out_of_the_subject() {
        let (name, id) = holder_of(Some(
            "CN=LOVELACE BYRON ADA, SERIALNUMBER=IDCES-00000000T, O=FNMT-RCM",
        ));

        assert_eq!(name, "LOVELACE BYRON ADA");
        assert_eq!(id, "IDCES-00000000T");
    }

    #[test]
    fn a_subject_without_the_fields_gives_empty_strings_and_not_a_panic() {
        assert_eq!(holder_of(None), (String::new(), String::new()));
    }

    /// El caso que rompía: el subject de un certificado de persona física de la
    /// FNMT **no lleva `O=`**, así que leer el emisor de ahí dejaba el panel en
    /// «Emitido por » y nada más.
    #[test]
    fn the_issuer_is_the_authority_and_not_the_organisation_of_the_holder() {
        let subject =
            "CN=EIDAS CERTIFICADO PRUEBAS - 99999999R, serialNumber=IDCES-99999999R, C=ES";
        let issuer = "CN=AC FNMT Usuarios, OU=Ceres, O=FNMT-RCM, C=ES";

        assert_eq!(issuer_of(Some(issuer)), "AC FNMT Usuarios");
        assert_eq!(attribute("O=", subject), "");
    }

    /// El otro caso malo: el `O=` del subject de un empleado público es su
    /// organismo, y enseñarlo como emisor afirmaba que ese organismo emitió el
    /// certificado.
    #[test]
    fn the_organisation_of_a_public_employee_is_never_read_as_the_issuer() {
        let subject = "CN=LOVELACE BYRON ADA, O=AYUNTAMIENTO DE CADIZ, C=ES";
        let issuer = "CN=AC Administracion Publica, O=FNMT-RCM, C=ES";

        let (name, id) = holder_of(Some(subject));

        assert_eq!(name, "LOVELACE BYRON ADA");
        assert_eq!(id, "");
        assert_eq!(issuer_of(Some(issuer)), "AC Administracion Publica");
    }

    /// Un issuer sin `CN=` no deja el panel mudo: se cae al `O=`, y sin ninguno
    /// de los dos, al nombre distinguido entero.
    #[test]
    fn an_issuer_without_a_common_name_falls_back_instead_of_going_blank() {
        assert_eq!(issuer_of(Some("O=FNMT-RCM, C=ES")), "FNMT-RCM");
        assert_eq!(issuer_of(Some("OU=Ceres, C=ES")), "OU=Ceres, C=ES");
        assert_eq!(issuer_of(None), "");
    }

    /// Un certificado del token con el DER que se le dé. Con basura dentro el
    /// estado sale `Unreadable`, que es justo lo que hace falta para probar la
    /// negativa sin fabricar un X.509.
    fn a_certificate(label: &str, der: &[u8]) -> TokenCertificate {
        TokenCertificate::new(
            CertificateRef::new("/usr/lib/softhsm/libsofthsm2.so", "rfirma-test", label),
            der.to_vec(),
        )
    }

    fn an_order() -> SigningOrder {
        SigningOrder {
            document: "/run/user/1000/doc/1e8b83b9/contrato.pdf".to_owned(),
            certificate: "FIRMA".to_owned(),
            placement: PlacementOrder {
                page: 1,
                media_box: [0.0, 0.0, 595.0, 842.0],
                rotation: 0,
                rect: [72, 500, 272, 600],
            },
            fields: VisibleFieldsOrder {
                signer_name: true,
                id_number: true,
                signed_at: true,
                reason: true,
            },
            reason: String::new(),
            signed_at: "31/08/26, 12:00:00".to_owned(),
            rubric: None,
            language: "es".to_owned(),
        }
    }

    #[test]
    fn refuses_a_certificate_that_is_no_longer_in_the_token() {
        let failure = usable_certificate(&[], "FIRMA").expect_err("ya no esta");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("FIRMA"), "{}", failure.detail);
    }

    #[test]
    fn looks_at_the_status_again_between_listing_and_signing() {
        // La ventana ya lo miró al listar, y aun así se vuelve a mirar: entre
        // una cosa y otra puede haberse retirado la tarjeta o haber pasado la
        // medianoche del `notAfter`.
        let certificates = [a_certificate("FIRMA", &[0x00, 0x01, 0x02])];

        let failure = usable_certificate(&certificates, "FIRMA").expect_err("no es legible");

        assert_eq!(failure.situation, "certificateNotFound");
        assert!(failure.detail.contains("Unreadable"), "{}", failure.detail);
    }

    #[test]
    fn the_geometry_of_the_order_becomes_pades_points() {
        let certificate = a_certificate("FIRMA", &[]);

        let config = config_for(&an_order(), &certificate).expect("el recuadro cabe");

        assert_eq!(config.signature_box.page, 1);
        assert_eq!(config.signature_box.lower_left_x, 72);
        assert_eq!(config.signature_box.upper_right_y, 600);
    }

    #[test]
    fn a_box_outside_the_page_is_refused_instead_of_being_clipped_in_silence() {
        // iText lo recortaría sin decir nada y la firma saldría válida igual,
        // con la rúbrica de trece puntos de ancho (ID-22).
        let order = SigningOrder {
            placement: PlacementOrder {
                rect: [72, 500, 900, 600],
                ..an_order().placement
            },
            ..an_order()
        };

        let failure = config_for(&order, &a_certificate("FIRMA", &[])).expect_err("se sale");

        assert_eq!(failure.situation, "boxOutOfPage");
    }

    #[test]
    fn an_empty_reason_is_not_sent_at_all() {
        // `signReason` vacío estampa una etiqueta «Motivo:» sin nada detrás.
        let config = config_for(&an_order(), &a_certificate("FIRMA", &[])).expect("cabe");

        assert_eq!(config.sign_reason, None);
    }

    #[test]
    fn a_reason_that_was_written_does_travel() {
        let order = SigningOrder {
            reason: "Conforme".to_owned(),
            ..an_order()
        };

        let config = config_for(&order, &a_certificate("FIRMA", &[])).expect("cabe");

        assert_eq!(config.sign_reason.as_deref(), Some("Conforme"));
    }

    #[test]
    fn a_signed_document_is_named_by_its_file_and_its_folder_and_nothing_else() {
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let checked = CheckedFolder::at(folder.path()).expect("existe");
        let landing = folder.path().join("contrato-firmado.pdf");

        let view = told_as(&landing, &checked);

        assert_eq!(view.name, "contrato-firmado.pdf");
        assert_eq!(
            view.folder,
            folder.path().file_name().and_then(|n| n.to_str()).unwrap()
        );
        // Ni el nombre ni la carpeta llevan un separador: si lo llevaran, sería
        // una ruta del anfitrión saliendo por la orden (ADR-0011).
        assert!(!view.name.contains('/'));
        assert!(!view.folder.contains('/'));
    }

    fn an_environment(documents_folder: &std::path::Path) -> Environment {
        Environment {
            stores: vec!["/usr/lib/softhsm/libsofthsm2.so".into()],
            documents_folder: documents_folder.to_path_buf(),
            configuration: Mutex::new(Configuration::default()),
            memory: super::Memory::at(&crate::paths::Paths::under(documents_folder)),
        }
    }

    /// El diálogo de abrir arranca donde caen los firmados, que es la única
    /// carpeta que la aplicación conoce bajo el arenero.
    #[test]
    fn the_open_dialog_starts_in_the_destination_folder() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let chosen = documents.path().join("Firmados");
        std::fs::create_dir(&chosen).expect("deberia crearse la carpeta de prueba");
        let environment = Environment {
            configuration: Mutex::new(Configuration {
                destination: Some(crate::memory::DestinationFolder::at(&chosen)),
                ..Configuration::default()
            }),
            ..an_environment(documents.path())
        };

        assert_eq!(starting_folder(&environment), Some(chosen));
    }

    /// Sin destino elegido manda la carpeta de documentos, igual que al
    /// guardar: las dos puntas del recorrido miran al mismo sitio.
    #[test]
    fn without_a_chosen_destination_it_starts_in_the_documents_folder() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let environment = an_environment(documents.path());

        assert_eq!(
            starting_folder(&environment),
            Some(documents.path().to_path_buf())
        );
    }

    /// La carpeta **no se crea nunca** (ID-38): si no está, el diálogo se abre
    /// donde el sistema quiera y ya está. Fabricarla aquí sería justo el fallo
    /// silencioso que midió el #27.
    #[test]
    fn a_missing_folder_neither_gets_created_nor_stops_the_dialog() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let absent = documents.path().join("Firmados");
        let environment = Environment {
            configuration: Mutex::new(Configuration {
                destination: Some(crate::memory::DestinationFolder::at(&absent)),
                ..Configuration::default()
            }),
            ..an_environment(documents.path())
        };

        assert_eq!(starting_folder(&environment), None);
        assert!(!absent.exists(), "la carpeta no se puede haber creado");
    }

    /// Fuera del arenero el diálogo devuelve una ruta de verdad, y entonces la
    /// carpeta de la que salió el documento **sí** se sabe.
    #[test]
    fn outside_the_sandbox_the_folder_the_document_came_from_is_the_real_one() {
        let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

        assert_eq!(
            folder_it_came_from(&document),
            Some(std::path::Path::new("/home/quien/Contratos"))
        );
    }

    /// Y bajo el arenero no se sabe. El padre del enlace del portal tiene ese
    /// solo fichero dentro y no es ninguna carpeta del usuario: apuntarlo
    /// abriría el diálogo la próxima vez en un directorio que ni existe ya.
    #[test]
    fn a_document_from_the_portal_leaves_no_folder_to_remember() {
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

        assert_eq!(folder_it_came_from(&document), None);
    }

    /// Lo pedido: la próxima vez el diálogo se abre donde estuvo la última vez,
    /// y no en el destino.
    #[test]
    fn the_last_folder_used_wins_over_the_destination_folder() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let contracts = documents.path().join("Contratos");
        std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
        let environment = an_environment(documents.path());
        remember_the_folder(
            &environment,
            &PortalDocument::opened(contracts.join("contrato.pdf")),
        );

        assert_eq!(starting_folder(&environment), Some(contracts));
    }

    /// Una carpeta que ya no está no es un punto de partida: es un diálogo que
    /// se abre en un sitio que no existe.
    #[test]
    fn a_remembered_folder_that_is_gone_falls_back_to_the_destination() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let contracts = documents.path().join("Contratos");
        std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
        let environment = an_environment(documents.path());
        remember_the_folder(
            &environment,
            &PortalDocument::opened(contracts.join("contrato.pdf")),
        );
        std::fs::remove_dir(&contracts).expect("deberia borrarse");

        assert_eq!(
            starting_folder(&environment),
            Some(documents.path().to_path_buf())
        );
    }

    /// Bajo el arenero no se apunta nada, así que el diálogo sigue abriéndose
    /// en el destino por los siglos de los siglos. Es lo correcto: la
    /// alternativa es guardar un directorio del portal.
    #[test]
    fn opening_through_the_portal_never_writes_a_folder_into_the_state() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let environment = an_environment(documents.path());

        remember_the_folder(
            &environment,
            &PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf"),
        );

        assert_eq!(
            environment
                .memory
                .state()
                .expect("deberia leerse el estado")
                .value()
                .last_open_folder,
            None
        );
        assert_eq!(
            starting_folder(&environment),
            Some(documents.path().to_path_buf())
        );
    }

    /// La carpeta es actividad, y «Recordar mi actividad» manda: con el
    /// interruptor apagado no se apunta, y el diálogo vuelve al destino.
    #[test]
    fn the_folder_is_not_remembered_with_the_activity_switch_off() {
        let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
        let contracts = documents.path().join("Contratos");
        std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
        let environment = Environment {
            configuration: Mutex::new(Configuration {
                remember_activity: false,
                ..Configuration::default()
            }),
            ..an_environment(documents.path())
        };

        remember_the_folder(
            &environment,
            &PortalDocument::opened(contracts.join("contrato.pdf")),
        );

        assert_eq!(
            starting_folder(&environment),
            Some(documents.path().to_path_buf())
        );
    }

    /// La misma guardia del ADR-0011 sobre lo que devuelven los ajustes.
    #[test]
    fn the_configuration_that_crosses_carries_no_host_path() {
        let body = production_half()
            .split_once("struct ConfigurationView")
            .expect("el tipo de salida sigue aqui")
            .1
            .split_once('}')
            .expect("y tiene cuerpo")
            .0;

        for leak in ["PathBuf", "&Path", "path:", "module:", "reading_path"] {
            assert!(
                !body.contains(leak),
                "«ConfigurationView» ha ganado un «{leak}»: eso es una ruta del anfitrion saliendo"
            );
        }
    }

    /// Sin destino elegido manda la carpeta de documentos, y sale por su
    /// nombre: la ruta se queda de este lado (ADR-0011).
    #[test]
    fn the_configuration_shows_the_destination_folder_by_its_name() {
        let view = shown(
            &Configuration::default(),
            std::path::Path::new("/home/quien/Documentos"),
        );

        assert_eq!(view.destination, "Documentos");
        assert!(!view.destination.contains('/'));
    }

    /// La ventana no elige la carpeta —bajo el arenero hay una sola—, así que
    /// lo que mande en ese campo no puede reescribir lo guardado.
    #[test]
    fn writing_the_configuration_never_moves_the_destination_folder() {
        let live = Configuration {
            destination: Some(crate::memory::DestinationFolder::at(
                "/home/quien/Documentos/Firmados",
            )),
            ..Configuration::default()
        };
        let chosen = ConfigurationView {
            language: "en".to_owned(),
            destination: "Otra".to_owned(),
            remember_visible_signature: false,
            remember_activity: true,
            theme: Theme::Dark,
        };

        let next = merged(&live, &chosen);

        assert_eq!(
            next.destination, live.destination,
            "el destino no lo elige la ventana"
        );
        assert_eq!(next.language, Language::English);
        assert!(!next.remember_visible_signature);
        assert_eq!(next.theme, Theme::Dark);
    }

    /// Un tema desconocido no puede tumbar la lectura de los ajustes: lo que
    /// hay guardado es un valor cerrado, y el catálogo de la ventana es el
    /// mismo. Aquí solo se comprueba el viaje de ida y vuelta.
    #[test]
    fn the_theme_survives_the_round_trip_to_the_window() {
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            let configuration = Configuration {
                theme,
                ..Configuration::default()
            };
            let view = shown(
                &configuration,
                std::path::Path::new("/home/quien/Documentos"),
            );

            assert_eq!(merged(&configuration, &view).theme, theme);
        }
    }

    #[test]
    fn the_signed_document_falls_into_the_destination_folder_without_a_dialog() {
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let environment = an_environment(folder.path());
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let view = deliver(&environment, &document, b"%PDF-firmado").expect("cae");

        assert_eq!(view.name, "contrato-firmado.pdf");
        assert_eq!(
            std::fs::read(folder.path().join("contrato-firmado.pdf")).expect("esta"),
            b"%PDF-firmado"
        );
    }

    #[test]
    fn a_second_signature_is_numbered_instead_of_overwriting_the_first() {
        // Sin diálogo por firma no hay ningún «ya existe, ¿reemplazar?» que
        // avise: sin la numeración, la segunda machacaría a la primera callando.
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let environment = an_environment(folder.path());
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        deliver(&environment, &document, b"la primera").expect("cae");
        let second = deliver(&environment, &document, b"la segunda").expect("cae tambien");

        assert_ne!(second.name, "contrato-firmado.pdf");
        assert_eq!(
            std::fs::read(folder.path().join("contrato-firmado.pdf")).expect("sigue"),
            b"la primera"
        );
    }

    #[test]
    fn a_destination_folder_that_is_not_there_is_told_and_never_created() {
        // Bajo el arenero crearla contesta OK y no deja nada en el anfitrión
        // (ID-38): la única respuesta correcta es decirlo.
        let missing = tempfile::tempdir()
            .expect("temporal")
            .path()
            .join("no-esta");
        let environment = an_environment(&missing);
        let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

        let failure = deliver(&environment, &document, b"x").expect_err("no esta");

        assert_eq!(failure.situation, "folderMissing");
        assert!(!missing.exists(), "la carpeta se ha creado, y no debía");
    }

    #[test]
    fn there_is_nothing_to_finish_when_no_cycle_was_started() {
        let session = SigningSession::default();

        let failure = take_signed_cycle(&session).expect_err("no hay ciclo");

        assert_eq!(failure.situation, "unknown");
    }

    #[test]
    fn the_language_of_the_window_picks_the_labels_of_the_box() {
        assert_eq!(language_of("ca"), Language::Catalan);
        assert_eq!(language_of("en"), Language::English);
        // Lo que no reconozcamos cae en castellano, que es el idioma del
        // documento administrativo corriente, y no en un panic.
        assert_eq!(language_of("de"), Language::Spanish);
    }
}
