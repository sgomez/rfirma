//! **El recorrido de la firma, partido en tres porque el PIN va en medio.**
//!
//! [`begin`] → [`sign_on_token`] → [`finish`]. Un solo paso que hiciera las
//! tres cosas dejaría a la ventana sin nada que contar durante los segundos de
//! la postfirma, y —lo que importa más— obligaría a mandar el PIN junto con el
//! documento, cuando todavía no se sabe si el documento se puede firmar.
//!
//! El ciclo a medias vive en [`SigningSession`], **no en la ventana**: lo que
//! la ventana no tiene no lo puede filtrar ni alterar, y el sello de sesión es
//! justo lo que no puede cambiar entre la prefirma y la postfirma (ADR-0016).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::app::certificates::StampedHolder;
use crate::app::cycle::{self, OpenCycle, SigningRequest, TokenSignature};
use crate::app::{certificates, documents, lock, recents};
use crate::commands::orders::SigningOrder;
use crate::commands::views::{Failure, SignedDocumentView};
use crate::destination::PortalDocument;
use crate::isolate::Isolate;
use crate::memory::{Configuration, ListedCertificates, Memory, OpenedDocuments};
use crate::pkcs11::{self, CertificateRef, Store, TokenCertificate};
use crate::signing::{
    compose_layer2_text, AdmissibleDocument, SessionSeal, SignatureConfig, VisibleTextFields,
};

/// El ciclo a medias, entre el PIN y la postfirma.
///
/// Vive en el backend y **no cruza a la ventana**: lo que la ventana no tiene
/// no lo puede perder ni alterar, y el sello de sesión es justo lo que no puede
/// cambiar entre la prefirma y la postfirma (ADR-0016).
#[derive(Default)]
pub struct SigningSession {
    open: Mutex<Option<InFlight>>,
    /// **Dónde cayó el último documento firmado en esta sesión.**
    ///
    /// No es parte del ciclo —el ciclo ya terminó cuando esto se escribe— pero
    /// vive aquí por lo mismo que él: bajo el sandbox la ventana nunca conoce
    /// la ruta (ADR-0011), así que la única forma de que «Abrir el PDF» y
    /// «Abrir la carpeta» lleguen al fichero es que el backend recuerde a
    /// dónde lo dejó. Lo que cruza sigue siendo el nombre.
    ///
    /// Se pisa en cada firma que termina bien: el resumen que hay delante es
    /// siempre el del último documento firmado.
    delivered: Mutex<Option<PathBuf>>,
}

struct InFlight {
    cycle: OpenCycle,
    document: PortalDocument,
    signature: Option<TokenSignature>,
    /// Con qué certificado se está firmando, para poder recordarlo **si la
    /// postfirma termina bien**. Viaja aquí y no se relee del token al acabar:
    /// entre la prefirma y la postfirma la tarjeta puede haberse retirado, y
    /// entonces no habría forma de saber con cuál se acababa de firmar.
    certificate: CertificateRef,
    /// El sello, transportado aparte del ciclo que lo emitió.
    ///
    /// Están separados a propósito: si el sello viviera solo dentro de
    /// [`OpenCycle`], compararlo consigo mismo no comprobaría nada. Esta es la
    /// copia que viaja, y [`OpenCycle::postsign`] exige que llegue idéntica.
    seal: SessionSeal,
}

/// **Caso de uso.** Prefirma: cruza la frontera y deja el ciclo abierto.
///
/// Antes de nada rechaza lo que no se puede firmar —cifrado, certificado, o no
/// es un PDF—, **antes de que se pida el PIN**.
pub fn begin(
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<(), Failure> {
    // Lo que la ventana manda es el identificador que se acuñó al abrir, y no
    // una ruta: quien sabe a qué documento del portal corresponde es el
    // registro, y solo él (ID-62).
    let document = documents::opened_document(opened, &order.document)?;
    let bytes = admitted_bytes(&document)?;
    let (config, reference, chain) = plan_signature(stores, listed, order)?;
    // Una copia para la sesión: la otra se va con la prefirma al otro lado de
    // la frontera.
    let certificate = reference.clone();

    let cycle = on_the_bridge(isolate, move |bridge| {
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
        certificate,
        seal,
    });
    Ok(())
}

/// **Caso de uso.** Firma en el token, con el PIN que se acaba de teclear.
///
/// **La única fase que toca la clave privada, y no cruza la FFI** (ADR-0001).
/// El PIN entra por aquí, se usa en `C_Login` y no se guarda en ningún sitio:
/// ni en la sesión, ni en el registro, ni de vuelta a la ventana.
pub fn sign_on_token(session: &SigningSession, pin: &str) -> Result<(), Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.as_mut().ok_or_else(no_open_cycle)?;
    in_flight.signature = Some(in_flight.cycle.sign_on_token(pin)?);
    Ok(())
}

/// **Caso de uso.** Postfirma: comprueba el sello, ensambla el PDF y lo deja
/// caer.
///
/// El documento cae **sin diálogo** (ID-36, ADR-0011): lo único que se elige es
/// la carpeta, y se eligió una vez.
///
/// Y es aquí donde se apunta el certificado usado: este es el **único** punto
/// del programa en el que una firma ha terminado bien (#110).
pub fn finish(
    isolate: &Isolate,
    session: &SigningSession,
    memory: &Memory,
    configuration: &Configuration,
    documents_folder: &Path,
) -> Result<SignedDocumentView, Failure> {
    let SignedCycle {
        cycle,
        document,
        signature,
        seal,
        certificate,
    } = take_signed_cycle(session)?;

    let signed = on_the_bridge(isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    let (landing, delivered) =
        documents::deliver(configuration, documents_folder, &document, &signed)?;
    // **Después** de que el documento haya caído, y no antes: mientras la
    // postfirma pueda fallar todavía no se ha firmado nada con este
    // certificado (#110).
    certificates::remember_the_certificate(memory, configuration, &certificate);
    // Y aquí, y **solo aquí**, se escribe la insignia `Firmado` (ID-76): el
    // documento que la lleva es el que acaba de caer, y nada más lo escribe.
    recents::note_signed(memory, configuration, &landing);
    // Y aquí se guarda la ruta, para los dos botones del resumen: es lo único
    // que puede llevar al usuario hasta el fichero, porque él nunca la ve
    // (ID-79, ADR-0011).
    *lock(&session.delivered) = Some(landing);
    Ok(delivered)
}

/// **Caso de uso.** El fichero que quedó escrito en la última firma.
///
/// Devuelve la ruta **hacia dentro**, para que la orden se la dé al portal: no
/// cruza a la ventana ni por asomo (ADR-0011). Sin firma terminada en esta
/// sesión no hay nada que abrir, y eso es un fallo y no un silencio: los dos
/// botones solo se pintan con el resumen delante, así que llegar aquí sin
/// documento entregado es que algo se ha descolocado.
pub fn signed_document(session: &SigningSession) -> Result<PathBuf, Failure> {
    lock(&session.delivered)
        .clone()
        .ok_or_else(no_signed_document)
}

/// **Caso de uso.** La carpeta donde quedó el fichero de la última firma.
///
/// Es el directorio padre de [`signed_document`], y no la carpeta de destino
/// leída otra vez de la configuración: si el usuario la ha cambiado desde que
/// firmó, «Abrir la carpeta» tiene que abrir aquella en la que está el fichero
/// del resumen, no la que se usaría en la siguiente firma.
pub fn signed_folder(session: &SigningSession) -> Result<PathBuf, Failure> {
    let landing = signed_document(session)?;
    landing
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(no_signed_document)
}

/// Ninguna firma ha terminado todavía en esta sesión.
fn no_signed_document() -> Failure {
    Failure::new("unknown", "no hay ningun documento firmado en esta sesion")
}

/// **Caso de uso.** Cancelar: se olvida el ciclo a medias.
///
/// Existe porque un ciclo abierto que no se cierra deja el sello y los bytes a
/// firmar vivos en memoria hasta que se cierre la ventana.
pub fn cancel(session: &SigningSession) {
    *lock(&session.open) = None;
}

/// El texto del recuadro que sale de la orden y del titular.
fn layer2_text_of(order: &SigningOrder, holder: &StampedHolder) -> String {
    compose_layer2_text(
        &VisibleTextFields {
            signer_name: order
                .fields
                .signer_name
                .then_some(holder.common_name.as_str())
                .filter(|name| !name.is_empty()),
            issuer: order
                .fields
                .issuer
                .then_some(holder.issuer.as_str())
                .filter(|issuer| !issuer.is_empty()),
            signed_at: order.fields.signed_at.then_some(order.signed_at.as_str()),
            reason: order
                .fields
                .reason
                .then_some(order.reason.as_str())
                .filter(|reason| !reason.is_empty()),
            pseudonym: holder.pseudonym,
        },
        super::configuration::language_of(&order.language),
    )
}

/// La configuración de firma que sale de la orden y del certificado elegido.
///
/// El nombre y el emisor se leen **del DER**, no de la orden: la ventana solo
/// manda el asa, y componer el recuadro con lo que la ventana diga sería dejar
/// que estampe cualquier nombre.
pub fn config_for(
    order: &SigningOrder,
    chosen: &TokenCertificate,
) -> Result<SignatureConfig, Failure> {
    let holder = certificates::stamped_holder_of(chosen);
    Ok(SignatureConfig {
        placement: order.placement.placement()?,
        layer2_text: layer2_text_of(order, &holder),
        rubric_image: order.rubric.clone(),
        // Un motivo vacío **no se envía**: `signReason` con la cadena vacía
        // estampa una etiqueta «Motivo:» sin nada detrás.
        sign_reason: (!order.reason.is_empty()).then(|| order.reason.clone()),
    })
}

/// Lo que hay que saber del token y de la orden antes de cruzar la frontera.
///
/// Junta las dos preguntas que se le hacen al token —qué certificado es, y si
/// todavía sirve— con la configuración que sale de él, porque las tres son la
/// misma decisión: **con qué se firma**.
///
/// Lo comparte [`crate::app::preview`], que llega al puente con exactamente
/// este mismo plan: la vista previa que se compusiera con otra configuración
/// enseñaría un sello que la firma no va a estampar.
pub(crate) fn plan_signature(
    stores: &[Store],
    listed: &ListedCertificates,
    order: &SigningOrder,
) -> Result<(SignatureConfig, CertificateRef, Vec<Vec<u8>>), Failure> {
    let found = pkcs11::list_certificates_across(stores)?;
    let chosen = certificates::usable_certificate(&found, &order.certificate, listed)?;
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
pub fn admitted_bytes(document: &PortalDocument) -> Result<Vec<u8>, Failure> {
    let bytes = std::fs::read(document.reading_path())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    AdmissibleDocument::check(&bytes)?;
    Ok(bytes)
}

/// Se lleva el ciclo a medias de la sesión, exigiendo que el token ya haya
/// firmado.
///
/// **Se lo lleva, no lo copia**: al salir de aquí la sesión queda vacía, así
/// que una postfirma que falle no deja un ciclo colgando que un segundo intento
/// pudiera reusar con otro sello. El ciclo se reabre desde la prefirma o no se
/// reabre.
pub fn take_signed_cycle(session: &SigningSession) -> Result<SignedCycle, Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.take().ok_or_else(no_open_cycle)?;
    let signature = in_flight
        .signature
        .ok_or_else(|| Failure::new("unknown", "todavía no se ha firmado en el token"))?;
    Ok(SignedCycle {
        cycle: in_flight.cycle,
        document: in_flight.document,
        signature,
        seal: in_flight.seal,
        certificate: in_flight.certificate,
    })
}

/// El ciclo ya firmado en el token, sacado de la sesión y listo para la
/// postfirma.
pub struct SignedCycle {
    cycle: OpenCycle,
    document: PortalDocument,
    signature: TokenSignature,
    seal: SessionSeal,
    certificate: CertificateRef,
}

/// Aplana las tres capas de resultado que devuelve un trabajo del isolate: el
/// hilo puede haberse caído, la librería puede no haber abierto, y el ciclo
/// puede haber fallado.
///
/// Lo comparte [`crate::app::preview`]: el isolate es uno y las tres capas de
/// resultado son las mismas, se esté firmando o solo viendo.
pub(crate) fn on_the_bridge<T: Send + 'static>(
    isolate: &Isolate,
    task: impl FnOnce(&crate::ffi::NativeBridge) -> Result<T, cycle::CycleError> + Send + 'static,
) -> Result<T, Failure> {
    match isolate.run(task) {
        Err(gone) => Err(gone.into()),
        Ok(Err(bridge)) => Err(bridge.into()),
        Ok(Ok(outcome)) => outcome.map_err(Failure::from),
    }
}

fn no_open_cycle() -> Failure {
    Failure::new("unknown", "no hay ninguna firma empezada")
}

#[cfg(test)]
mod tests {
    use super::{
        admitted_bytes, begin, cancel, config_for, finish, sign_on_token, signed_document,
        signed_folder, take_signed_cycle, SigningSession,
    };
    use crate::app::fixtures::{a_certificate, a_memory, an_order};
    use crate::commands::orders::{PlacementOrder, SigningOrder};
    use crate::destination::PortalDocument;
    use crate::isolate::Isolate;
    use crate::memory::{Configuration, ListedCertificates, OpenedDocuments};
    use crate::signing::PageSet;

    /// **Grada A**: lo que se comprueba leyendo esta fuente son invariantes de
    /// forma —qué guarda la sesión y desde dónde se recuerda el certificado—.
    /// El ciclo contra el token y `pdfsig` es la grada C de
    /// `tests/native_cycle.rs`.
    const SOURCE: &str = include_str!("signing.rs");

    /// La mitad de producción, sin las pruebas: si no, esta comprobación se
    /// leería a sí misma y encontraría siempre sus propios literales.
    fn production_half() -> &'static str {
        half_of(SOURCE)
    }

    /// La mitad de producción de **cualquier** fuente, sin sus pruebas: si no,
    /// estas comprobaciones leerían los literales de los tests y se creerían
    /// cualquier cosa.
    fn half_of(source: &'static str) -> &'static str {
        source
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(source)
    }

    #[test]
    fn the_pin_is_never_kept_in_the_half_open_cycle() {
        // Entra por `sign_on_token`, se usa en el token y no se guarda: ni en
        // la sesión a medias, ni en ningún tipo de salida.
        let session = production_half()
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

    /// **La insignia `Firmado` la escribe solo la postfirma** (ID-76).
    ///
    /// Se lee la fuente y no el resultado porque lo que se vigila es una
    /// propiedad de **todo** el backend, no de un recorrido: un
    /// `Badge::Signed` nuevo en el caso de uso que abre un documento, o en la
    /// orden que anota la fila, pondría `Firmado` en un PDF que rFirma no ha
    /// firmado. Contar las firmas de un PDF ajeno es la ficha 14 y es de v1.0.
    ///
    /// La vista previa entra en la lista aunque no escriba nada: componer el
    /// PDF con un `PK1` inventado y anotar `Firmado` sería poner la insignia a
    /// un documento que nadie ha firmado (ID-136).
    ///
    /// La mitad de comportamiento la fija
    /// `the_signed_document_is_the_only_row_that_gets_the_signed_badge`, en
    /// [`crate::app::recents`]; esta es la que solo se ve mirando los cuatro
    /// ficheros a la vez.
    #[test]
    fn the_signed_badge_is_written_by_the_postsign_and_by_nothing_else() {
        let writers = [
            ("app/signing.rs", production_half()),
            ("app/recents.rs", half_of(include_str!("recents.rs"))),
            ("app/documents.rs", half_of(include_str!("documents.rs"))),
            ("app/preview.rs", half_of(include_str!("preview.rs"))),
            (
                "commands/mod.rs",
                half_of(include_str!("../commands/mod.rs")),
            ),
        ];

        for (file, source) in writers {
            let written = source.matches("Badge::Signed").count();
            let expected = usize::from(file == "app/recents.rs");
            assert_eq!(
                written, expected,
                "«{file}» escribe la insignia Firmado {written} veces y tenia que escribirla \
                 {expected}: el unico sitio es `recents::note_signed`, y quien lo llama es la \
                 postfirma"
            );
        }

        let recents = half_of(include_str!("recents.rs"));
        let note_signed = recents
            .split_once("pub fn note_signed(")
            .expect("el anotador del firmado sigue aqui")
            .1;
        assert!(
            note_signed.contains("Badge::Signed"),
            "y esta dentro de `note_signed`"
        );
        let postsign = production_half()
            .split_once("pub fn finish(")
            .expect("la postfirma sigue aqui")
            .1;
        assert!(
            postsign.contains("recents::note_signed("),
            "a quien solo llama la postfirma"
        );
    }

    /// **Elegir no es firmar.** Lo que recuerda el certificado es la postfirma,
    /// y solo ella: si `remember_the_certificate` apareciera también en el caso
    /// de uso que lista o en el que prefirma, elegir uno en el desplegable y
    /// cerrar sin firmar cambiaría lo recordado —y con «Recordar mi actividad»
    /// apagado escribiría en disco por cada clic—.
    #[test]
    fn only_the_postsign_remembers_the_certificate() {
        let source = production_half();

        assert_eq!(
            source
                .matches("certificates::remember_the_certificate(")
                .count(),
            1,
            "se recuerda desde un solo sitio"
        );
        let postsign = source
            .split_once("pub fn finish(")
            .expect("la postfirma sigue aqui")
            .1;
        assert!(
            postsign.contains("certificates::remember_the_certificate("),
            "y ese sitio es la postfirma"
        );
    }

    #[test]
    fn the_geometry_of_the_order_becomes_pades_points() {
        let certificate = a_certificate("FIRMA", &[]);

        let config = config_for(&an_order(), &certificate).expect("el recuadro cabe");

        assert_eq!(config.placement.pages, PageSet::only_page(1));
        assert_eq!(config.placement.rect.lower_left_x, 72);
        assert_eq!(config.placement.rect.upper_right_y, 600);
    }

    #[test]
    fn a_box_outside_the_page_is_refused_instead_of_being_clipped_in_silence() {
        // iText lo recortaría sin decir nada y la firma saldría válida igual,
        // con la rúbrica de trece puntos de ancho (ID-22).
        let order = SigningOrder {
            placement: PlacementOrder {
                rect: [72.0, 500.0, 900.0, 600.0],
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
    fn there_is_nothing_to_finish_when_no_cycle_was_started() {
        let session = SigningSession::default();

        let Err(failure) = take_signed_cycle(&session) else {
            panic!("no hay ciclo abierto que llevarse");
        };

        assert_eq!(failure.situation, "unknown");
    }

    /// Los dos botones del resumen no tienen nada que abrir hasta que una
    /// firma termina: preguntar antes es un fallo, no un silencio.
    #[test]
    fn there_is_nothing_to_open_before_the_first_signature_of_the_session() {
        let session = SigningSession::default();

        let Err(failure) = signed_document(&session) else {
            panic!("no se ha firmado nada todavia");
        };
        assert_eq!(failure.situation, "unknown");
        assert!(signed_folder(&session).is_err());
    }

    /// Y la ruta que abren la guarda la sesión, **no la ventana**: bajo el
    /// sandbox la ventana nunca conoce la ruta del fichero (ADR-0011), así que
    /// las dos órdenes no reciben ninguna y leen la que dejó la postfirma.
    #[test]
    fn the_two_openers_read_the_landing_the_postsign_left_behind() {
        let session = SigningSession::default();
        let folder = tempfile::tempdir().expect("deberia haber temporal");
        let landing = folder.path().join("contrato-firmado.pdf");
        *crate::app::lock(&session.delivered) = Some(landing.clone());

        assert_eq!(signed_document(&session).expect("hay firmado"), landing);
        assert_eq!(signed_folder(&session).expect("y carpeta"), folder.path());
    }

    /// Ni la ruta ni nada que se le parezca sale por la orden: lo que la
    /// sesión guarda se queda dentro (ADR-0011).
    #[test]
    fn the_remembered_landing_never_leaves_the_backend() {
        let crossing = production_half()
            .split_once("pub struct SigningSession {")
            .expect("la sesion sigue aqui")
            .1
            .split_once("\n}")
            .expect("y tiene cuerpo")
            .0;

        assert!(
            crossing.contains("delivered"),
            "la sesion tiene que recordar donde cayo el firmado: {crossing}"
        );
        assert!(
            !crossing.contains("Serialize"),
            "la sesion no se serializa: si cruzara, cruzaria una ruta del anfitrion"
        );
    }

    /// Lo que no es un PDF cae **antes del diálogo del PIN** (#60): se decide
    /// sobre los bytes, sin token y sin cruzar la frontera.
    #[test]
    fn what_is_not_a_pdf_is_refused_before_the_pin() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let other = home.path().join("hoja.ods");
        std::fs::write(&other, b"PK\x03\x04").expect("deberia escribirse el temporal");

        let failure =
            admitted_bytes(&PortalDocument::opened(other)).expect_err("no es un PDF que firmar");

        assert_eq!(failure.situation, "notAPdf");
    }

    /// Y un documento que ya no está se cuenta aparte: el fallo no es que no
    /// sea un PDF, es que no se puede leer.
    #[test]
    fn a_document_that_is_gone_is_told_apart_from_one_that_is_not_a_pdf() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");

        let failure = admitted_bytes(&PortalDocument::opened(home.path().join("no-esta.pdf")))
            .expect_err("no esta");

        assert_eq!(failure.situation, "documentUnreadable");
    }

    /// La prefirma pide el documento **por su identificador**, y uno que no es
    /// de esta sesión no abre ningún ciclo.
    #[test]
    fn a_signature_cannot_begin_on_a_document_that_is_not_open() {
        let order = SigningOrder {
            document: "00000000000000000000000000000000".to_owned(),
            ..an_order()
        };

        let failure = begin(
            &order,
            &[],
            &ListedCertificates::new(),
            &OpenedDocuments::new(),
            &Isolate::start(),
            &SigningSession::default(),
        )
        .expect_err("ese documento no esta abierto");

        assert_eq!(failure.situation, "documentUnreadable");
    }

    /// Y la postfirma no ensambla nada si no hay ciclo: se para antes de cruzar
    /// la frontera, no después.
    #[test]
    fn the_postsign_stops_before_the_bridge_when_no_cycle_was_started() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");

        let failure = finish(
            &Isolate::start(),
            &SigningSession::default(),
            &a_memory(home.path()),
            &Configuration::default(),
            home.path(),
        )
        .expect_err("no hay ciclo abierto");

        assert_eq!(failure.situation, "unknown");
    }

    /// El PIN no tiene nada que firmar sin un ciclo abierto, y esa es la única
    /// respuesta: no se abre uno por el camino.
    #[test]
    fn the_pin_has_nothing_to_sign_when_no_cycle_was_started() {
        let failure =
            sign_on_token(&SigningSession::default(), "1234").expect_err("no hay ciclo abierto");

        assert_eq!(failure.situation, "unknown");
    }

    /// Cancelar deja la sesión vacía, que es lo que suelta el sello y los bytes
    /// a firmar sin esperar a que se cierre la ventana.
    #[test]
    fn cancelling_leaves_no_cycle_behind() {
        let session = SigningSession::default();

        cancel(&session);

        assert!(
            take_signed_cycle(&session).is_err(),
            "no queda ciclo que llevarse"
        );
    }
}
