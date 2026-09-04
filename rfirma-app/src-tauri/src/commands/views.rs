//! **Lo que cruza a la ventana**: los tipos de salida, las conversiones que
//! los producen y los nombres en inglés con los que el catálogo los traduce.
//!
//! Viven aquí y no intercalados entre los cuerpos de las órdenes (ID-80): una
//! orden desempaqueta el estado, llama a un caso de uso y traduce el resultado
//! a uno de estos tipos, y quien quiere saber qué ve la ventana lee un fichero
//! y no doscientas líneas de órdenes.
//!
//! # Ninguno lleva una ruta del anfitrión
//!
//! No es una recomendación, es una consecuencia del ADR-0011: bajo el sandbox
//! la aplicación **no conoce** la ruta real de un documento —el portal solo la
//! da a un llamante `is_host`, que un flatpak nunca es—, así que devolver una
//! sería devolver una mentira. Lo que sale de aquí son **nombres**: el del
//! fichero firmado y el de la carpeta donde cayó. La guarda que lo vigila está
//! en [`super::guards`] y recorre **todos** los ficheros de este módulo, así
//! que un tipo de salida nuevo queda cubierto por existir (ID-84).

use serde::{Deserialize, Serialize};

use crate::memory::{Badge, Theme};
use crate::pkcs11::{CertificateStatus, StoreClass, StoreSecret};
use crate::signing::PageSet;

pub use super::failure::Failure;

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

/// **Cómo hay que pedirle el secreto al almacén**, tal y como sale de la
/// prefirma (ID-189).
///
/// Es el espejo exacto de [`StoreSecret`], con las mismas tres variantes: la
/// ventana lee `kind` y decide entre firmar directo y abrir el diálogo. La
/// tercera —`typedOnTheReaderKeypad`— **no llega hoy a cruzar**, porque la
/// prefirma la rechaza antes; está aquí porque el tipo es de tres variantes y
/// partirlo en dos vocabularios, uno dentro y otro fuera, costaría más que la
/// rama que sobra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SecretView {
    NotNeeded,
    #[serde(rename_all = "camelCase")]
    TypedOnScreen {
        /// Cuántos intentos quedan. **Siempre vacío**: PKCS#11 no lo cuenta
        /// (ID-191).
        attempts_left: Option<u32>,
    },
    TypedOnTheReaderKeypad,
}

impl From<StoreSecret> for SecretView {
    fn from(secret: StoreSecret) -> Self {
        match secret {
            StoreSecret::NotNeeded => Self::NotNeeded,
            StoreSecret::TypedOnScreen { attempts_left } => Self::TypedOnScreen { attempts_left },
            StoreSecret::TypedOnTheReaderKeypad => Self::TypedOnTheReaderKeypad,
        }
    }
}

/// El nombre en inglés de una clase de almacén, que es la clave con la que el
/// catálogo de la ventana la traduce.
///
/// Es la forma del ID-29 aplicada a algo que no es un fallo: cruza la **clase**
/// y el rótulo lo pone la ventana, igual que con la `situation` de [`Failure`].
/// Componer aquí «Perfil de Firefox» se saltaría los catálogos y sacaría
/// castellano en la versión en inglés.
pub fn store_name(class: StoreClass) -> &'static str {
    match class {
        StoreClass::Card => "card",
        StoreClass::Firefox => "firefox",
        StoreClass::Chrome => "chrome",
        StoreClass::Nssdb => "nssdb",
    }
}

/// Un certificado, con lo justo para pintar su fila y para volver a encontrarlo.
///
/// **No lleva el DER, ni la ruta del módulo, ni el `configdir` del perfil.** El
/// DER es de quien lee X.509, que es Rust; los otros dos son rutas del
/// anfitrión (ADR-0011). Lo que la ventana devuelve para firmar es el `id`, y
/// el backend reencuentra el resto en [`crate::memory::ListedCertificates`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateView {
    /// El asa acuñada al listar. **Sin significado para la ventana**: no se
    /// deriva de nada del certificado y de ella no se reconstruye ninguna ruta.
    ///
    /// Es lo que identifica la fila, y no la `label`: las etiquetas se repiten
    /// —dos claves con el mismo `CKA_LABEL` en un perfil de Firefox, dos
    /// `FNMT-GEMELO-99999999R` en el token de pruebas— y buscando por etiqueta
    /// se cogía siempre el primero, así que el segundo era inelegible.
    pub id: String,
    pub label: String,
    pub holder_name: String,
    pub id_number: String,
    pub issuer: String,
    /// De qué clase es el almacén de donde salió: `card`, `firefox`, `chrome`
    /// o `nssdb`. **Nunca una ruta y nunca el rótulo ya escrito**: el mismo
    /// certificado en el perfil de Firefox y en `~/.pki/nssdb` es
    /// indistinguible sin esto, y quien lo traduce es el catálogo de la
    /// ventana.
    pub store: String,
    pub status: StatusView,
    /// Si es **el que se usó la última vez** (#110).
    ///
    /// Es un `bool` y no una segunda orden porque la ventana ya está pidiendo
    /// esta lista: preguntar aparte cuál se recordó obligaría a encadenar dos
    /// llamadas para pintar el desplegable una vez. Y es una propiedad de la
    /// **fila**, no del certificado: dice cuál viene ya puesto, y con el
    /// recordado fuera del token no viene marcada ninguna, que es como el panel
    /// vuelve a «Sin certificado» sin ruido.
    pub remembered: bool,
}

/// Dónde va a caer el documento que hay delante: **la carpeta y el nombre**,
/// los dos por su nombre y ninguno por su ruta (ID-63, ADR-0011).
///
/// Es lo que el pie del panel de firma enseña **antes** de firmar, así que trae
/// las dos cosas que hacen falta para pintarlo: el nombre con el que va a caer
/// —el que compone [`CheckedFolder::landing_for`](crate::destination::CheckedFolder::landing_for),
/// con su sufijo y su número de desempate ya resueltos— y si la carpeta se
/// puede escribir, que decide [`CheckedFolder::check`](crate::destination::CheckedFolder::check)
/// y no un literal (ID-67).
///
/// `name` es `None` cuando la carpeta no está o no se deja escribir: sin
/// carpeta comprobada no hay dónde resolver el homónimo, y aventurar un nombre
/// sería prometer un fichero que nadie va a escribir.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationView {
    /// El **nombre** de la carpeta de destino, su último segmento.
    pub folder: String,
    /// El nombre del fichero firmado que va a caer ahí, homónimos incluidos.
    pub name: Option<String>,
    /// Si esa carpeta está y se puede escribir **ahora mismo**. No se persiste:
    /// es un hecho sobre el disco de este instante.
    pub writable: bool,
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
    /// Cuántos bytes se han escrito.
    ///
    /// Sale de la escritura misma —`std::fs::write` recibe la rebanada y su
    /// longitud es el tamaño— y **no de volver a mirar el fichero** (ID-77):
    /// preguntarle al disco por algo que ya se sabe abre la ventana a que el
    /// resumen cuente un tamaño distinto del que se acaba de escribir.
    pub size_bytes: u64,
}

/// Un documento abierto, tal como la ventana lo recibe: **un identificador, un
/// nombre y la ruta real cuando se conoce** (ID-60, ID-185, ADR-0011).
///
/// El `modified` sale de aquí y no lo calcula la ventana porque quien tocó el
/// disco es el backend: la fila de la bandeja se pinta con metadatos cacheados
/// y sin volver a abrir el fichero (ADR-0010).
///
/// Los bytes se siguen pidiendo **contra el identificador** y nunca contra la
/// ruta (ID-66): `path` está para enseñarla, no para leer por ella.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocumentView {
    /// El identificador opaco que acuñó [`crate::memory::OpenedDocuments`].
    pub id: String,
    /// El nombre del fichero. No su ruta.
    pub name: String,
    /// El `mtime`, en segundos desde la época; `None` si no se pudo leer.
    pub modified: Option<u64>,
    /// La ruta real del documento, o `None` si entró por el portal y no se
    /// conoce (ID-185). Lo decide
    /// [`crate::app::documents::real_path_of`], y lo que **nunca** sale es el
    /// enlace de `/run/user/…`.
    pub path: Option<String>,
}

/// Lo que la ventana recibe al soltar ficheros encima.
///
/// **Ninguna ruta** (ADR-0011). Lo que se suelta son rutas del anfitrión, y
/// justamente por eso la decisión de cuál se abre se toma en el backend: lo que
/// cruza es el documento ya apuntado, con su identificador opaco, igual que si
/// se hubiera elegido por el diálogo.
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

/// El recuadro colocado, tal como cruza en los dos sentidos: **el rectángulo
/// en espacio de usuario PDF y el conjunto de páginas**, y ninguna ruta.
///
/// Es la forma del ID-90 —`{ rect, pages }`, un registro llano y no una unión
/// de un brazo— y la misma que la ventana tiene en `viewer/signatureBox.ts`.
///
/// El rectángulo cruza **entero** aunque se guarde partido (ID-74): la ventana
/// pinta un rectángulo, no una esquina más un tamaño global, y quien junta las
/// dos mitades es [`crate::app::recents`]. Componer el rectángulo en TypeScript
/// sería poner el reparto del ID-74 en los dos lados. El conjunto de páginas no
/// se parte: es entero del documento (ID-95).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementView {
    /// El recuadro en espacio de usuario: `[x0, y0, x1, y1]`.
    pub rect: [f64; 4],
    /// En qué páginas se estampa: `"all"` o la lista, **1-based**.
    pub pages: PageSet,
}

/// Una fila de la bandeja, tal como la ventana la recibe: **un identificador
/// opaco y un nombre, ninguna ruta** (ID-62, ID-75, ADR-0011).
///
/// La deduplicación de la bandeja sigue siendo por la ruta canónica que **solo
/// Rust conoce**; lo que cruza es el identificador con el que se piden los
/// bytes.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDocumentView {
    /// El identificador opaco con el que se lee el documento.
    pub id: String,
    /// El nombre del fichero. No su ruta.
    pub name: String,
    /// La insignia cacheada. `Unavailable` no está aquí: se recalcula al
    /// listar y viaja en `available`.
    pub badge: Badge,
    /// El `mtime` cacheado, en segundos desde la época.
    pub modified: Option<u64>,
    /// Cuándo se usó por última vez, en segundos desde la época.
    pub last_used: u64,
    /// Si la ruta responde **ahora mismo**. No se persiste nunca: es un hecho
    /// sobre el disco de este instante, y por eso lo recalcula el backend en
    /// cada listado.
    pub available: bool,
    /// Dónde cayó el recuadro en este documento, con el tamaño global ya
    /// puesto. `None` si nadie lo colocó.
    pub placement: Option<PlacementView>,
}

/// La configuración, tal como la ventana la ve: **ningún `PathBuf`**.
///
/// El destino sale por su [`nombre`](crate::destination::DestinationFolder::name) y
/// nunca por su ruta, igual que todo lo demás que cruza (ADR-0011). Y va en un
/// solo sentido de verdad: la ventana **no elige la carpeta** —bajo el sandbox
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
    /// «Avisarme cuando haya una versión nueva». Siempre visible, sin
    /// condición (ID-180).
    pub notify_new_version: bool,
    /// El tema de la ventana. Ver [`Theme`].
    pub theme: Theme,
    /// **La única pregunta al entorno** (ID-184): si Preferencias puede
    /// ofrecer «guardar junto al original».
    ///
    /// Cruza como un booleano cuyo nombre **es la pregunta**, y no como el
    /// canal en el que corre la aplicación: la ventana no tiene por qué saber
    /// si hay un sandbox debajo, solo si pinta la opción. La contesta
    /// [`crate::destination::the_original_folder_can_be_offered`].
    ///
    /// Viaja en un solo sentido, como el destino: lo que llegue en una
    /// escritura se ignora, y por eso lleva `default` —la ventana no tiene que
    /// devolverlo.
    #[serde(default)]
    pub offers_the_original_folder: bool,
}

/// **La versión nueva que se anuncia en la franja** (ID-181).
///
/// Un solo campo, y es el número: la franja no ofrece descargar nada (ID-177),
/// su acción lleva a *Acerca de*, que es donde están las órdenes de alta del
/// repositorio. Cruza como cadena porque lo que la ventana hace con ella es
/// pintarla.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewVersionView {
    /// La versión publicada, `mayor.menor.parche` y sin la `v` delante.
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::{
        store_name, CertificateView, OpenedDocumentView, SecretView, SignedDocumentView, StatusView,
    };
    use crate::pkcs11::{CertificateStatus, StoreClass, StoreSecret};

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

    /// Lo que la ventana lee para decidir entre firmar directo y abrir el
    /// diálogo son estos tres nombres, así que están fijados aquí (ID-189).
    #[test]
    fn the_secret_crosses_as_one_of_three_kinds_and_never_as_a_string() {
        assert_eq!(
            serde_json::to_string(&SecretView::from(StoreSecret::NotNeeded)).expect("serializa"),
            r#"{"kind":"notNeeded"}"#
        );
        assert_eq!(
            serde_json::to_string(&SecretView::from(StoreSecret::TypedOnScreen {
                attempts_left: None
            }))
            .expect("serializa"),
            r#"{"kind":"typedOnScreen","attemptsLeft":null}"#
        );
        assert_eq!(
            serde_json::to_string(&SecretView::from(StoreSecret::TypedOnTheReaderKeypad))
                .expect("serializa"),
            r#"{"kind":"typedOnTheReaderKeypad"}"#
        );
    }

    #[test]
    fn a_signed_document_is_told_with_two_names_and_its_size() {
        let view = SignedDocumentView {
            name: "contrato_signed.pdf".to_owned(),
            folder: "Documentos".to_owned(),
            size_bytes: 2_400_000,
        };

        assert_eq!(
            serde_json::to_string(&view).expect("serializa"),
            r#"{"name":"contrato_signed.pdf","folder":"Documentos","sizeBytes":2400000}"#
        );
    }

    #[test]
    fn a_certificate_crosses_without_its_der_and_without_its_module() {
        let view = CertificateView {
            id: "0123456789abcdef0123456789abcdef".to_owned(),
            label: "ETIQUETA".to_owned(),
            holder_name: "Ada Lovelace Byron".to_owned(),
            id_number: "IDCES-00000000T".to_owned(),
            issuer: "FNMT-RCM".to_owned(),
            store: store_name(StoreClass::Firefox).to_owned(),
            status: StatusView::Valid,
            remembered: false,
        };
        let json = serde_json::to_string(&view).expect("serializa");

        assert!(json.contains(r#""holderName":"Ada Lovelace Byron""#));
        assert!(!json.contains(r#""der""#), "el DER no sale: {json}");
        assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
    }

    /// El almacén cruza como **clase en inglés** y no como rótulo ni como
    /// ruta: el nombre en castellano lo pone el catálogo de la ventana, igual
    /// que hace con la `situation` de un fallo.
    #[test]
    fn the_store_crosses_as_a_class_and_never_as_a_path() {
        let names = [
            store_name(StoreClass::Card),
            store_name(StoreClass::Firefox),
            store_name(StoreClass::Chrome),
            store_name(StoreClass::Nssdb),
        ];

        assert_eq!(names, ["card", "firefox", "chrome", "nssdb"]);
        for name in names {
            assert!(!name.contains('/'), "«{name}» parece una ruta");
            assert!(
                name.chars().all(|letter| letter.is_ascii_lowercase()),
                "«{name}» no es una clase en ingles"
            );
        }
    }

    /// El documento que entró por el portal cruza **sin ruta ninguna**: bajo el
    /// sandbox no se conoce, y el enlace de `/run/user/…` no es la del usuario
    /// (ID-185).
    #[test]
    fn an_opened_document_from_the_portal_is_told_without_a_path() {
        let view = OpenedDocumentView {
            id: "0f1e2d3c4b5a69788796a5b4c3d2e1f0".to_owned(),
            name: "contrato.pdf".to_owned(),
            modified: Some(1_700_000_000),
            path: None,
        };

        let json = serde_json::to_string(&view).expect("serializa");

        assert_eq!(
            json,
            r#"{"id":"0f1e2d3c4b5a69788796a5b4c3d2e1f0","name":"contrato.pdf","modified":1700000000,"path":null}"#
        );
        assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
    }

    /// Y el de ruta directa cruza **con la ruta real**, como la enseña
    /// cualquier aplicación de escritorio (ID-185).
    #[test]
    fn an_opened_document_with_a_direct_path_is_told_with_the_real_one() {
        let view = OpenedDocumentView {
            id: "0f1e2d3c4b5a69788796a5b4c3d2e1f0".to_owned(),
            name: "contrato.pdf".to_owned(),
            modified: Some(1_700_000_000),
            path: Some("/home/quien/Contratos/contrato.pdf".to_owned()),
        };

        let json = serde_json::to_string(&view).expect("serializa");

        assert!(
            json.contains(r#""path":"/home/quien/Contratos/contrato.pdf""#),
            "la ruta real se enseña: {json}"
        );
    }
}
