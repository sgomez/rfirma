//! **Las órdenes de Tauri**: lo único que la ventana puede pedirle al backend.
//!
//! Son seis, y la lista es cerrada a propósito. Cada una rellena un puerto que
//! la interfaz ya tenía declarado desde el #76 —`CertificateStore`,
//! `Layer2Composer`, `SigningBackend`—, así que la ventana no aprende nada
//! nuevo de Tauri: sigue hablando con los mismos puertos y es `main.tsx` quien
//! elige estas implementaciones.
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

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::destination::{CheckedFolder, PortalDocument};
use crate::ffi::BridgeError;
use crate::memory::Configuration;
use crate::pkcs11::{self, CertificateRef, CertificateStatus, Situation, TokenError};
use crate::signing::{
    compose_layer2_text, cycle, AdmissibleDocument, Language, OpenCycle, Refusal, SealMismatch,
    SessionSeal, SignatureBox, SignatureConfig, SigningRequest, TokenSignature, VisibleTextFields,
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
    Detail {
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
            CertificateStatus::Unreadable { detail } => Self::Detail { detail },
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
    /// Dónde cae el recuadro, en puntos PAdES.
    pub signature_box: SignatureBox,
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

/// El módulo PKCS#11 y la carpeta de destino, que salen de la configuración.
pub struct Environment {
    /// Dónde está el `.so` del token.
    pub module: std::path::PathBuf,
    /// La carpeta de documentos del usuario, para cuando no haya destino
    /// elegido.
    pub documents_folder: std::path::PathBuf,
    /// Lo que se recuerda entre sesiones.
    pub configuration: Mutex<Configuration>,
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

/// El titular y el DNI que se leen del certificado, para el recuadro y para la
/// fila del panel.
fn holder_of(subject: Option<&str>) -> (String, String, String) {
    let subject = subject.unwrap_or_default();
    let field = |name: &str| {
        subject
            .split(',')
            .map(str::trim)
            .find_map(|part| part.strip_prefix(name).map(str::to_owned))
            .unwrap_or_default()
    };
    (field("CN="), field("SERIALNUMBER="), field("O="))
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
    let found = pkcs11::list_certificates(&environment.module)?;
    Ok(found
        .into_iter()
        .map(|certificate| {
            let (holder_name, id_number, issuer) = holder_of(certificate.subject().as_deref());
            CertificateView {
                label: certificate.reference().label().to_owned(),
                holder_name,
                id_number,
                issuer,
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
    let certificates = pkcs11::list_certificates(&environment.module)?;
    let chosen = certificates
        .iter()
        .find(|certificate| certificate.reference().label() == label)
        .ok_or_else(|| {
            Failure::new(
                "certificateNotFound",
                format!("el token ya no tiene {label}"),
            )
        })?;
    let (name, id, _) = holder_of(chosen.subject().as_deref());
    Ok((name, id))
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
) -> Result<(), Failure> {
    let document = PortalDocument::opened(&order.document);
    let bytes = std::fs::read(document.reading_path())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    // La puerta rápida del #60: se decide sobre los bytes, sin token y sin
    // frontera, y por eso puede caer antes del diálogo del PIN. El préstamo se
    // acaba aquí mismo para que los bytes puedan viajar al hilo del isolate.
    AdmissibleDocument::check(&bytes).map_err(Failure::from)?;

    let certificates = pkcs11::list_certificates(&environment.module)?;
    let chosen = certificates
        .iter()
        .find(|certificate| certificate.reference().label() == order.certificate)
        .ok_or_else(|| {
            Failure::new(
                "certificateNotFound",
                format!("el token ya no tiene {}", order.certificate),
            )
        })?;
    // La segunda vez que se mira el estado, y no sobra: entre listar y firmar
    // puede haber pasado la medianoche del `notAfter`.
    if !chosen.status().is_usable() {
        return Err(Failure::new(
            "certificateNotFound",
            format!("{:?}", chosen.status()),
        ));
    }
    let (name, id, _) = holder_of(chosen.subject().as_deref());
    let config = SignatureConfig {
        signature_box: order.signature_box,
        layer2_text: layer2_text_of(&order, &(name, id)),
        rubric_image: order.rubric.clone(),
        sign_reason: (!order.reason.is_empty()).then(|| order.reason.clone()),
    };
    let reference: CertificateRef = chosen.reference().clone();
    let chain = vec![chosen.der().to_vec()];

    let cycle = isolate
        .run(move |bridge| {
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
        })
        .map_err(Failure::from)?
        .map_err(Failure::from)?
        .map_err(Failure::from)?;

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
    let (cycle, document, signature, seal) = {
        let mut open = lock(&session.open);
        let in_flight = open.take().ok_or_else(no_open_cycle)?;
        let signature = in_flight
            .signature
            .ok_or_else(|| Failure::new("unknown", "todavía no se ha firmado en el token"))?;
        (
            in_flight.cycle,
            in_flight.document,
            signature,
            in_flight.seal,
        )
    };

    let chosen = {
        let configuration = lock(&environment.configuration);
        crate::destination::chosen_folder(&configuration, environment.documents_folder.clone())
    };
    // La carpeta se comprueba y **no se crea nunca** (ID-38): bajo el arenero
    // crearla contesta OK y no deja nada en el anfitrión.
    let folder = CheckedFolder::check(&chosen)?;
    let landing = folder.landing_for(&document)?;

    let signed = isolate
        .run(move |bridge| cycle.postsign(bridge, &signature, &seal))
        .map_err(Failure::from)?
        .map_err(Failure::from)?
        .map_err(Failure::from)?;

    std::fs::write(&landing, &signed)
        .map_err(|error| Failure::new("folderUnwritable", error.to_string()))?;

    Ok(SignedDocumentView {
        name: landing
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned(),
        folder: folder.name().to_owned(),
    })
}

/// **Orden 6.** Cancelar: se olvida el ciclo a medias.
///
/// Existe porque un ciclo abierto que no se cierra deja el sello y los bytes a
/// firmar vivos en memoria hasta que se cierre la ventana.
#[tauri::command]
pub fn cancel_signing(session: State<'_, SigningSession>) {
    *lock(&session.open) = None;
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
        holder_of, language_of, situation_name, CertificateView, Failure, SignedDocumentView,
        StatusView,
    };
    use crate::pkcs11::{CertificateStatus, Situation};
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
            r#"{"kind":"detail","detail":"PEM error"}"#
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
        let (name, id, issuer) = holder_of(Some(
            "CN=LOVELACE BYRON ADA, SERIALNUMBER=IDCES-00000000T, O=FNMT-RCM",
        ));

        assert_eq!(name, "LOVELACE BYRON ADA");
        assert_eq!(id, "IDCES-00000000T");
        assert_eq!(issuer, "FNMT-RCM");
    }

    #[test]
    fn a_subject_without_the_fields_gives_empty_strings_and_not_a_panic() {
        assert_eq!(
            holder_of(None),
            (String::new(), String::new(), String::new())
        );
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
