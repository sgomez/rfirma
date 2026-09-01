//! El certificado tal y como sale del token, y su clasificación.
//!
//! Dos ideas mandan aquí:
//!
//! - **Del certificado se guarda cómo volver a encontrarlo, no quién es**
//!   (ID-32, ADR-0010). Eso es [`CertificateRef`]: módulo, token, etiqueta y
//!   `CKA_ID`, y nada más. El titular se lee del DER cada vez que hace falta
//!   pintarlo, con
//!   [`TokenCertificate::subject`], y por eso no hay forma de persistirlo desde
//!   aquí sin escribirlo a propósito.
//! - **Un certificado caducado no es un fallo del token.** Es un
//!   [`CertificateStatus`], se conoce leyendo el DER —sin sesión y **sin pedir
//!   el PIN**— y por eso no comparte tipo con [`TokenError`](super::TokenError).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use x509_cert::der::Decode;
use x509_cert::Certificate;

use super::stores::Store;

/// Cómo volver a encontrar un certificado en el próximo arranque (ID-32).
///
/// Es lo único de esta parte del programa que tiene sentido persistir: no lleva
/// titular, ni DNI, ni número de serie. Por eso es **este** tipo el que se
/// serializa en el estado ([`crate::memory`]) y no [`TokenCertificate`], que
/// arrastra el DER entero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateRef {
    module: PathBuf,
    token_label: String,
    label: String,
    /// El `CKA_ID` del certificado, que es lo que de verdad lo empareja con su
    /// clave privada.
    ///
    /// `None` es **un certificado recordado antes del #98**, cuando la
    /// referencia solo tenía tres coordenadas: el fichero de estado de una
    /// versión anterior se lee sin romperse, y lo que queda sin saber es el
    /// `CKA_ID`, no el certificado.
    #[serde(default)]
    cka_id: Option<Vec<u8>>,
    /// Lo que hubo que pasarle a `C_Initialize` para abrir el almacén de donde
    /// salió, o `None` cuando el módulo no necesitaba que le dijeran nada.
    ///
    /// Sin esto un certificado de NSS **no se puede reencontrar**: todos los
    /// perfiles de Firefox de una máquina se sirven por el mismo
    /// `libsoftokn3.so` y su token se llama igual en todos, «NSS Certificate
    /// DB», así que el módulo y la etiqueta del token no distinguen un perfil
    /// de otro. Lo que los distingue es el `configdir` de estos init args.
    #[serde(default)]
    init_args: Option<String>,
}

impl CertificateRef {
    /// Construye la referencia a partir del almacén y de las tres coordenadas
    /// que la sitúan dentro de él.
    ///
    /// El almacén se acepta como ruta de módulo a secas —que es lo que es para
    /// una tarjeta— o como [`Store`] entero, que es lo que hace falta para NSS.
    ///
    /// El `CKA_ID` admite tanto `vec![0x01]` como `None`; lo segundo solo tiene
    /// sentido para reconstruir una referencia recordada por una versión
    /// anterior, porque un certificado leído del token siempre trae el suyo.
    pub fn new(
        store: impl Into<Store>,
        token_label: impl Into<String>,
        label: impl Into<String>,
        cka_id: impl Into<Option<Vec<u8>>>,
    ) -> Self {
        let store = store.into();
        Self {
            module: store.path().to_path_buf(),
            token_label: token_label.into(),
            label: label.into(),
            cka_id: cka_id.into(),
            init_args: store.init_args().map(str::to_owned),
        }
    }

    /// El almacén de donde salió, listo para volver a abrirlo.
    pub fn store(&self) -> Store {
        Store::with_init_args(&self.module, self.init_args.clone())
    }

    /// Ruta del módulo PKCS#11 que lo sirve.
    pub fn module(&self) -> &Path {
        &self.module
    }

    /// Etiqueta del token dentro de ese módulo. El número de ranura **no** vale:
    /// SoftHSM lo reasigna al inicializar y una tarjeta cambia de ranura al
    /// reinsertarla.
    pub fn token_label(&self) -> &str {
        &self.token_label
    }

    /// `CKA_LABEL` del objeto dentro del token. Sirve para pintarlo y para
    /// reconocerlo, **no** para encontrar su clave: en un almacén de verdad la
    /// etiqueta se repite (ID-06).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// `CKA_ID` del objeto dentro del token: la coordenada con la que PKCS#11
    /// —y NSS— emparejan certificado y clave privada.
    ///
    /// `None` solo aparece leyendo el estado de una versión anterior al #98, o
    /// en un token que no le ponga `CKA_ID` a sus objetos.
    pub fn cka_id(&self) -> Option<&[u8]> {
        self.cka_id.as_deref()
    }

    /// Si esta referencia y otra señalan al **mismo** certificado.
    ///
    /// No es `==`, y la diferencia es justo el certificado que recordó una
    /// versión anterior: al leído del token le sobran coordenadas que al
    /// recordado le faltan —el `CKA_ID` antes del #98, los `init_args` antes
    /// del #99—, y comparando por igualdad ninguno de esos volvería a
    /// encontrarse nunca. Lo que **no se sabe no descarta**: una coordenada
    /// ausente en cualquiera de los dos lados deja de opinar.
    ///
    /// El precio de esa tolerancia es conocido y pequeño: dos certificados que
    /// compartan etiqueta en el mismo token son indistinguibles para una
    /// referencia sin `CKA_ID`, y entonces se reencuentra el primero. Es lo que
    /// se podía saber en aquella versión; a la primera firma se reescribe con
    /// las cinco coordenadas y deja de pasar.
    pub fn is_the_same_as(&self, other: &Self) -> bool {
        self.module == other.module
            && self.token_label == other.token_label
            && self.label == other.label
            && agree(self.cka_id.as_deref(), other.cka_id.as_deref())
            && agree(self.init_args.as_deref(), other.init_args.as_deref())
    }
}

/// Dos coordenadas concuerdan si las dos están y valen lo mismo, o si a
/// cualquiera de las dos le falta.
fn agree<T: PartialEq + ?Sized>(one: Option<&T>, other: Option<&T>) -> bool {
    match (one, other) {
        (Some(one), Some(other)) => one == other,
        _ => true,
    }
}

/// En qué estado está el certificado, decidido **antes** de pedir el PIN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateStatus {
    /// En vigor, hasta donde se puede saber sin red.
    Valid,
    /// Ya caducó. La fecha va en segundos desde la época.
    Expired { not_after: u64 },
    /// Todavía no ha entrado en vigor.
    NotYetValid { not_before: u64 },
    /// Revocado por su emisora.
    ///
    /// Esto **no** lo produce este módulo: comprobar la revocación es hablar con
    /// el OCSP, que es grada D y solo corre en el cron (TD-08). La variante está
    /// para que el resultado de esa comprobación tenga dónde caer sin
    /// disfrazarse de fallo del token.
    Revoked { reason: String },
    /// El DER que hay en el token no es un certificado X.509 que sepamos leer.
    Unreadable { detail: String },
}

impl CertificateStatus {
    /// Si se puede firmar con él. Lo mira el recorrido de firma antes de abrir
    /// el diálogo del PIN.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Un certificado leído del token: la referencia para reencontrarlo, y el DER
/// para todo lo demás.
#[derive(Clone, Debug)]
pub struct TokenCertificate {
    reference: CertificateRef,
    der: Vec<u8>,
}

impl TokenCertificate {
    /// Envuelve el DER tal cual sale de `CKA_VALUE`.
    pub fn new(reference: CertificateRef, der: Vec<u8>) -> Self {
        Self { reference, der }
    }

    /// Las coordenadas persistibles.
    pub fn reference(&self) -> &CertificateRef {
        &self.reference
    }

    /// El certificado en DER, tal cual está en el token.
    pub fn der(&self) -> &[u8] {
        &self.der
    }

    /// El titular, para pintarlo. Se recalcula del DER cada vez: no se almacena
    /// ni se devuelve dentro de [`CertificateRef`], porque el ADR-0010 dice que
    /// esto no se persiste.
    pub fn subject(&self) -> Option<String> {
        Certificate::from_der(&self.der)
            .ok()
            .map(|certificate| certificate.tbs_certificate().subject().to_string())
    }

    /// La autoridad que lo emitió, para pintarla. **No es el `O=` del titular**:
    /// ese es la organización de quien firma, y enseñarlo como emisor le dice a
    /// quien firma que su propio organismo emitió el certificado. El emisor es
    /// el dato con el que se decide si un certificado es de fiar, así que sale
    /// del campo que de verdad lo lleva.
    ///
    /// Se recalcula del DER cada vez, por lo mismo que [`Self::subject`].
    pub fn issuer(&self) -> Option<String> {
        Certificate::from_der(&self.der)
            .ok()
            .map(|certificate| certificate.tbs_certificate().issuer().to_string())
    }

    /// El estado ahora mismo, leyendo el reloj del sistema.
    pub fn status(&self) -> CertificateStatus {
        self.status_at(SystemTime::now())
    }

    /// El estado en un instante dado. Existe con parámetro para poder probar la
    /// caducidad sin fabricar certificados ni tocar el reloj de la máquina.
    pub fn status_at(&self, instant: SystemTime) -> CertificateStatus {
        let certificate = match Certificate::from_der(&self.der) {
            Ok(certificate) => certificate,
            Err(error) => {
                return CertificateStatus::Unreadable {
                    detail: error.to_string(),
                }
            }
        };

        let validity = certificate.tbs_certificate().validity();
        let not_before = validity.not_before.to_unix_duration();
        let not_after = validity.not_after.to_unix_duration();
        let now = instant.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);

        if now > not_after {
            CertificateStatus::Expired {
                not_after: not_after.as_secs(),
            }
        } else if now < not_before {
            CertificateStatus::NotYetValid {
                not_before: not_before.as_secs(),
            }
        } else {
            CertificateStatus::Valid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: no hay token de por medio.
    #[test]
    fn a_reference_carries_the_four_coordinates_and_nothing_else() {
        let reference =
            CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x2a, 0x01]);

        assert_eq!(reference.module(), Path::new("/usr/lib/x.so"));
        assert_eq!(reference.token_label(), "rfirma-test");
        assert_eq!(reference.label(), "ETIQUETA");
        assert_eq!(reference.cka_id(), Some([0x2a, 0x01].as_slice()));
    }

    /// Lo que escribio una version anterior al #98 no lleva `cka_id`, y leerlo
    /// tiene que dar una referencia sin `CKA_ID` en vez de un error: el fichero
    /// de estado de quien actualiza no puede tumbar el arranque.
    #[test]
    fn a_reference_remembered_before_the_cka_id_existed_still_reads() {
        let written = r#"{
            "module": "/usr/lib/x.so",
            "token_label": "rfirma-test",
            "label": "ETIQUETA"
        }"#;

        let reference: CertificateRef =
            serde_json::from_str(written).expect("una referencia antigua tiene que leerse");

        assert_eq!(reference.label(), "ETIQUETA");
        assert_eq!(reference.cka_id(), None);
    }

    #[test]
    fn a_reference_round_trips_through_the_state_file_with_its_cka_id() {
        let reference = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x05]);

        let written = serde_json::to_string(&reference).expect("deberia serializarse");
        let read: CertificateRef = serde_json::from_str(&written).expect("deberia leerse");

        assert_eq!(read, reference);
        assert_eq!(read.cka_id(), Some([0x05].as_slice()));
    }

    #[test]
    fn a_der_that_is_not_a_certificate_is_unreadable_rather_than_a_panic() {
        let certificate = TokenCertificate::new(
            CertificateRef::new("/usr/lib/x.so", "rfirma-test", "BASURA", vec![0x01]),
            vec![0x00, 0x01, 0x02],
        );

        assert!(matches!(
            certificate.status(),
            CertificateStatus::Unreadable { .. }
        ));
        assert_eq!(certificate.subject(), None);
        assert_eq!(certificate.issuer(), None);
        assert!(!certificate.status().is_usable());
    }

    /// Lo que hace falta al arrancar: la referencia recordada reconoce a la
    /// que acaba de salir del token, y no reconoce a ninguna otra.
    #[test]
    fn a_remembered_reference_recognises_the_one_that_came_out_of_the_token() {
        let remembered = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "FIRMA", vec![0x01]);

        assert!(remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/x.so",
            "rfirma-test",
            "FIRMA",
            vec![0x01]
        )));
        assert!(!remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/x.so",
            "rfirma-test",
            "FIRMA",
            vec![0x02]
        )));
        assert!(!remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/x.so",
            "otro-token",
            "FIRMA",
            vec![0x01]
        )));
        assert!(!remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/otro.so",
            "rfirma-test",
            "FIRMA",
            vec![0x01]
        )));
    }

    /// El certificado que recordó una versión anterior al #98 y al #99 no lleva
    /// `CKA_ID` ni `init_args`, y aun así tiene que reencontrarse: lo que no se
    /// sabe no descarta.
    #[test]
    fn a_reference_remembered_by_an_older_version_still_finds_its_certificate() {
        let written = r#"{
            "module": "/usr/lib/libsoftokn3.so",
            "token_label": "NSS Certificate DB",
            "label": "FIRMA"
        }"#;
        let remembered: CertificateRef =
            serde_json::from_str(written).expect("una referencia antigua tiene que leerse");

        let listed = CertificateRef::new(
            Store::with_init_args(
                "/usr/lib/libsoftokn3.so",
                Some("configdir='/home/quien/.mozilla/firefox/abc'".to_owned()),
            ),
            "NSS Certificate DB",
            "FIRMA",
            vec![0x01],
        );

        assert!(remembered.is_the_same_as(&listed));
    }

    /// Y dos perfiles de Firefox distintos **no** son el mismo certificado
    /// aunque compartan módulo, token y etiqueta: lo que los separa son los
    /// `init_args`, que es para lo que entraron en el #99.
    #[test]
    fn two_firefox_profiles_are_not_the_same_certificate() {
        let one = CertificateRef::new(
            Store::with_init_args(
                "/usr/lib/libsoftokn3.so",
                Some("configdir='/uno'".to_owned()),
            ),
            "NSS Certificate DB",
            "FIRMA",
            vec![0x01],
        );
        let other = CertificateRef::new(
            Store::with_init_args(
                "/usr/lib/libsoftokn3.so",
                Some("configdir='/otro'".to_owned()),
            ),
            "NSS Certificate DB",
            "FIRMA",
            vec![0x01],
        );

        assert!(!one.is_the_same_as(&other));
    }

    #[test]
    fn a_revocation_is_not_a_token_failure() {
        let status = CertificateStatus::Revoked {
            reason: "superseded".to_owned(),
        };

        assert!(!status.is_usable());
    }
}
