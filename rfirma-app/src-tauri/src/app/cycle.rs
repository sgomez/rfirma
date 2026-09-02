//! **El ciclo trifásico**: prefirma en Java, firma en Rust, postfirma en Java
//! (ID-14, ID-15, ID-16, ID-17, ADR-0001, ADR-0016).
//!
//! Es el único sitio del programa donde se juntan la frontera FFI, el token, el
//! sello de sesión, las coordenadas y la rúbrica. Está entero aquí y no
//! repartido a propósito: tres módulos negociando esta frontera por separado es
//! justo el fallo que el ADR-0016 documenta.
//!
//! **Vive en [`crate::app`] y no en [`crate::signing`]** (ID-82). Es
//! orquestación: llama a la frontera nativa y al token, así que ponerlo entre
//! las reglas puras de la firma obligaba a que esas reglas importaran
//! [`crate::ffi`], y eso cerraba un ciclo entre los dos módulos. Aquí las
//! dependencias van todas hacia el dominio (ID-81), y las recibe explícitas:
//! el puente entra por argumento en [`presign`] y en [`OpenCycle::postsign`].
//!
//! ```text
//!   1. prefirma   Java   PDF + cadena + extraParams  ->  PRE (DER) + sesión + sello
//!   2. firma      Rust   PRE  --CKM_SHA256_RSA_PKCS-->  PK1
//!   3. postfirma  Java   sello intacto + sesión + PK1 ->  PDF firmado
//! ```
//!
//! # La clave privada no entra en el isolate de Java
//!
//! No es una recomendación: es lo que hace que un DNIe sirva de algo. La fase 2
//! es la única que toca la clave y corre contra el PKCS#11 del sistema
//! ([`crate::pkcs11`]). Ni el PIN ni la clave aparecen en ningún argumento que
//! cruce la FFI, y hay una prueba abajo que se pone roja si alguien abre esa
//! puerta —[`java_has_no_entry_point_for_the_signing_phase`] y
//! [`the_pin_has_no_way_across_the_border`]—.
//!
//! # El sello se transporta, no se lee
//!
//! [`OpenCycle`] guarda el sello que devolvió la prefirma y **no lo enseña**:
//! lo único que sale es una copia para quien tenga que transportarlo, y
//! [`OpenCycle::postsign`] exige que vuelva idéntica **antes** de cruzar. Sin
//! esa comparación la postfirma completa sin error y el PDF sale con
//! `Digest Mismatch`: la firma se invalida en silencio, que es peor que un
//! fallo porque nadie se entera.
//!
//! # La cofirma no es otro recorrido
//!
//! Un PDF que ya viene firmado se cofirma por este mismo camino, con los mismos
//! `extraParams`: PAdES añade una firma en una actualización incremental y no
//! toca la anterior. Lo único que cambia es que
//! [`AdmissibleDocument::already_signed`] dice que sí.

use base64::Engine;

use crate::ffi::{BridgeError, NativeBridge, PostSignRequest, PreSignRequest};
use crate::pkcs11::{self, CertificateRef, TokenError};
use crate::signing::{
    to_java_properties, AdmissibleDocument, Refusal, SealMismatch, SessionSeal, SignatureConfig,
};

/// El algoritmo de firma, en el nombre que entiende Java.
///
/// Va emparejado con `CKM_SHA256_RSA_PKCS` del ID-16, que es el mecanismo con
/// el que el token firma los bytes de `PRE`. **Los dos nombres son la misma
/// decisión**: cambiar uno sin el otro produce una firma que no valida y que
/// nadie ve fallar hasta que un validador la abre.
pub const ALGORITHM: &str = "SHA256withRSA";

/// El separador de la cadena de certificados que espera
/// `PadesBridge.parseCertificates`.
const CHAIN_SEPARATOR: &str = ";";

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Lo que hace falta para abrir un ciclo de firma.
#[derive(Clone, Copy, Debug)]
pub struct SigningRequest<'a> {
    /// El documento, ya admitido: cifrado o certificado no llega hasta aquí.
    pub document: AdmissibleDocument<'a>,
    /// La cadena de certificados en DER, **el del firmante primero**.
    pub chain: &'a [Vec<u8>],
    /// Dónde cae el recuadro y qué lleva dentro.
    pub config: &'a SignatureConfig,
    /// Con qué clave se firmará la fase 2. Solo son coordenadas: ni el PIN ni
    /// la clave viven en este tipo.
    pub certificate: &'a CertificateRef,
}

/// Lo que puede salir mal en el ciclo, con cada causa en su sitio.
///
/// Están separadas porque se cuentan en sitios distintos: la del token vuelve
/// al diálogo del PIN, la del documento se dice antes de abrirlo, y la del
/// sello no es ni una cosa ni la otra —es la invariante del ADR-0016 saltando—.
#[derive(Debug)]
pub enum CycleError {
    /// El documento no se puede firmar, y se sabía antes de pedir el PIN.
    Inadmissible(Refusal),
    /// La prefirma o la postfirma han fallado al otro lado de la frontera.
    Bridge(BridgeError),
    /// El token ha dicho que no.
    Token(TokenError),
    /// El sello que llega a la postfirma no es el que produjo la prefirma.
    Seal(SealMismatch),
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inadmissible(refusal) => write!(f, "{refusal}"),
            Self::Bridge(error) => write!(f, "{error}"),
            Self::Token(error) => write!(f, "{error}"),
            Self::Seal(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CycleError {}

impl From<Refusal> for CycleError {
    fn from(refusal: Refusal) -> Self {
        Self::Inadmissible(refusal)
    }
}

impl From<BridgeError> for CycleError {
    fn from(error: BridgeError) -> Self {
        Self::Bridge(error)
    }
}

impl From<TokenError> for CycleError {
    fn from(error: TokenError) -> Self {
        Self::Token(error)
    }
}

impl From<SealMismatch> for CycleError {
    fn from(error: SealMismatch) -> Self {
        Self::Seal(error)
    }
}

/// Cuántos bytes ocupa el `PK1` inventado de [`TokenSignature::invented`].
///
/// Son los de una firma RSA de 2048 bits, que es la de los certificados de
/// firma que rFirma maneja. La longitud no cambia lo que se pinta —el hueco que
/// el PAdES reserva para la firma se rellena igual—, así que aquí no hace falta
/// leerle el módulo al certificado ni hacer criptografía RSA en Rust, que es
/// justo lo que el proyecto delega en el token.
const INVENTED_PKCS1_BYTES: usize = 256;

/// El PKCS#1 que ha calculado el token. Es lo único que produce la fase 2.
///
/// Existe como tipo propio, y no como un `Vec<u8>` suelto, para que la firma no
/// se pueda confundir con los bytes que se firmaron: los dos son secuencias de
/// bytes y se pasan seguidos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSignature(Vec<u8>);

impl TokenSignature {
    /// **Un `PK1` inventado**, para la prefirma en seco de la vista previa
    /// (ID-136).
    ///
    /// No lo ha calculado ningún token y no vale como firma: existe para que
    /// [`OpenCycle::postsign`] tenga con qué ensamblar el PDF cuando lo que se
    /// quiere no es firmar sino **ver el sello que va a quedar**. Lo que la
    /// postfirma necesita del `PK1` es su sitio dentro del CMS, no su valor: el
    /// sondeo del #115 midió que los bytes visibles del PDF compuesto así son
    /// idénticos a los del firmado de verdad.
    ///
    /// Se llama desde [`crate::app::preview`], y de ningún otro sitio: un
    /// recorrido de firma que llegara a la postfirma con esto en vez de con lo
    /// que devolvió el token produciría un PDF que ningún validador acepta.
    pub fn invented() -> Self {
        Self(vec![0; INVENTED_PKCS1_BYTES])
    }

    /// La firma cruda, tal cual la devolvió el token.
    pub fn raw(&self) -> &[u8] {
        &self.0
    }

    /// La firma en Base64, que es como viaja en el campo `PK1` (ID-16).
    pub fn to_pkcs1_base64(&self) -> String {
        base64(&self.0)
    }
}

/// Un ciclo de firma **empezado y sin cerrar**: la prefirma ya está hecha y
/// falta el PIN.
///
/// El sello vive aquí dentro y no sale nunca entero: [`OpenCycle::seal_in_transit`]
/// entrega una copia para transportarla y [`OpenCycle::postsign`] exige esa
/// misma copia de vuelta. Ni un `get`, ni un `parse`: en cuanto Rust
/// interpretara el sello podría reconstruirlo, y un sello reconstruible no
/// protege de nada (ADR-0016).
pub struct OpenCycle {
    pdf_b64: String,
    chain_b64: String,
    session: String,
    to_be_signed: Vec<u8>,
    seal: SessionSeal,
    certificate: CertificateRef,
    already_signed_before: bool,
}

/// **Fase 1.** Cruza la frontera con el documento, la cadena y la configuración.
///
/// Lo que vuelve son los **atributos firmados CAdES en ASN.1 DER** (ID-15): un
/// bloque para hashear y firmar, no un hash y no un `DigestInfo`.
pub fn presign(
    bridge: &NativeBridge,
    request: SigningRequest<'_>,
) -> Result<OpenCycle, CycleError> {
    let pdf_b64 = base64(request.document.bytes());
    let chain_b64 = request
        .chain
        .iter()
        .map(|der| base64(der))
        .collect::<Vec<_>>()
        .join(CHAIN_SEPARATOR);
    let extra_params = to_java_properties(&request.config.extra_params());

    let presigned = bridge.presign(PreSignRequest {
        pdf_b64: &pdf_b64,
        algorithm: ALGORITHM,
        certificate_chain_b64: &chain_b64,
        extra_params: &extra_params,
    })?;

    Ok(OpenCycle {
        pdf_b64,
        chain_b64,
        session: presigned.session().to_owned(),
        to_be_signed: presigned.pre_sign().to_vec(),
        seal: presigned.stamp().clone(),
        certificate: request.certificate.clone(),
        already_signed_before: request.document.already_signed(),
    })
}

impl OpenCycle {
    /// Los bytes que el token tiene que firmar, **sin hashear**.
    pub fn to_be_signed(&self) -> &[u8] {
        &self.to_be_signed
    }

    /// Con qué certificado se abrió el ciclo.
    pub fn certificate(&self) -> &CertificateRef {
        &self.certificate
    }

    /// Si el documento ya traía firma, es decir, si esto es una cofirma.
    pub fn is_cosigning(&self) -> bool {
        self.already_signed_before
    }

    /// Una copia del sello para quien tenga que transportarlo hasta la
    /// postfirma. **No se lee**: se lleva y se devuelve.
    pub fn seal_in_transit(&self) -> SessionSeal {
        self.seal.clone()
    }

    /// **Fase 2.** Firma los bytes de `PRE` en el token, y solo en el token.
    ///
    /// Aquí no hay FFI, no hay isolate y no hay Java: es
    /// [`crate::pkcs11::sign`] contra el módulo del sistema con
    /// `CKM_SHA256_RSA_PKCS`, que recibe el bloque sin hashear y hashea él.
    pub fn sign_on_token(&self, pin: &str) -> Result<TokenSignature, CycleError> {
        let signature = pkcs11::sign(&self.certificate, pin, &self.to_be_signed)?;
        Ok(TokenSignature(signature))
    }

    /// **Fase 3.** Comprueba el sello y ensambla el PDF firmado.
    ///
    /// `returned` es el sello tal y como ha vuelto del transporte. Se compara
    /// **antes** de cruzar la frontera: si no coincide, aquí no se llama a
    /// nadie y el error es visible, en vez de un PDF con `Digest Mismatch` que
    /// nadie sabe leer.
    pub fn postsign(
        &self,
        bridge: &NativeBridge,
        signature: &TokenSignature,
        returned: &SessionSeal,
    ) -> Result<Vec<u8>, CycleError> {
        self.seal.verify_unchanged(returned)?;

        Ok(bridge.postsign(PostSignRequest {
            pdf_b64: &self.pdf_b64,
            certificate_chain_b64: &self.chain_b64,
            stamp: returned,
            session: &self.session,
            pkcs1_b64: &signature.to_pkcs1_base64(),
        })?)
    }
}

/// `Debug` mudo sobre el sello y sobre el documento: en un registro de fallos
/// no tiene nada que hacer ni el contenido del PDF ni el interior del sello.
impl std::fmt::Debug for OpenCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenCycle")
            .field("certificate", &self.certificate)
            .field("to_be_signed_bytes", &self.to_be_signed.len())
            .field("cosigning", &self.already_signed_before)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionSeal, TokenSignature, ALGORITHM};
    use std::collections::BTreeSet;

    /// **Grada A**: lo que se comprueba aquí son las invariantes del recorrido,
    /// no la firma. El ciclo entero, contra el token y con `pdfsig` delante,
    /// es la grada C de `tests/native_cycle.rs`.
    ///
    /// La frontera FFI, tal cual está escrita. Es la evidencia sobre la que se
    /// sostiene el ADR-0001: si alguien abriera una puerta para delegar la fase
    /// 2 en Java, tendría que pasar por este fichero.
    const BORDER: &str = include_str!("../ffi.rs");

    /// La mitad de producción de este mismo fichero.
    ///
    /// Se corta por `mod tests` porque, si no, las pruebas de abajo se leerían
    /// a sí mismas: los literales que buscan aparecerían siempre, y las dos
    /// comprobaciones pasarían —o fallarían— por su propio texto.
    fn production_half(source: &str) -> &str {
        source
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .unwrap_or(source)
    }

    /// Los identificadores del código, para poder buscar palabras enteras.
    ///
    /// Buscar la subcadena no vale: `copy_nonoverlapping` lleva un «pin»
    /// dentro, y una prueba que se pone roja por eso no dice nada de la clave
    /// privada.
    fn identifiers(source: &str) -> BTreeSet<&str> {
        source
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|word| !word.is_empty())
            .collect()
    }

    /// Los nombres de los puntos de entrada de la librería nativa que resuelve
    /// `NativeBridge::open_at`, sacados del propio fichero.
    fn entry_points() -> BTreeSet<String> {
        BORDER
            .match_indices("autofirma_")
            .map(|(start, _)| {
                BORDER[start..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect()
            })
            .collect()
    }

    #[test]
    fn java_has_no_entry_point_for_the_signing_phase() {
        // Delegar la fase 2 en Java exige una entrada nueva en la librería, y
        // esta prueba es la que se pondría roja al añadirla. Son tres: las dos
        // fases que sí son de Java, y la que libera las cadenas del ID-11.
        let expected: BTreeSet<String> = [
            "autofirma_free_string",
            "autofirma_pades_postsign",
            "autofirma_pades_presign",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        assert_eq!(
            entry_points(),
            expected,
            "la frontera con Java ha cambiado de puntos de entrada: si uno de \
             ellos firma, la clave privada ha entrado en el isolate (ADR-0001)"
        );
    }

    #[test]
    fn the_pin_has_no_way_across_the_border() {
        // Ni el PIN ni la clave privada aparecen en la frontera. Lo que se
        // busca no es la palabra suelta sino un identificador: un `pin` en una
        // firma de función o en un campo de petición sería la fase 2 cruzando.
        let words = identifiers(BORDER);
        for forbidden in ["pin", "private_key", "AuthPin", "cryptoki"] {
            assert!(
                !words.contains(forbidden),
                "«{forbidden}» aparece en la frontera FFI: la fase 2 se estaría \
                 delegando a Java, contra el ADR-0001"
            );
        }
    }

    #[test]
    fn only_the_pkcs11_module_talks_to_the_token() {
        // La otra mitad de lo mismo, desde el lado del ciclo: quien firma es
        // `pkcs11::sign`, y aquí no hay ni criptografía propia ni una segunda
        // ruta hacia la clave.
        let cycle = production_half(include_str!("cycle.rs"));

        assert!(cycle.contains("pkcs11::sign(&self.certificate, pin"));
        // El puente prefirma y postfirma, y nada más: una tercera llamada
        // sobre él que firmara sería la clave privada cruzando.
        assert!(!identifiers(cycle).contains("sign_on_bridge"));
        assert_eq!(cycle.matches("bridge.").count(), 2);
    }

    #[test]
    fn the_algorithm_matches_the_pkcs11_mechanism() {
        // Los dos nombres son la misma decisión (ID-16): `SHA256withRSA` en
        // Java y `CKM_SHA256_RSA_PKCS` en el token. Cambiar uno solo produce
        // una firma que no valida y que nadie ve fallar.
        let token_side = include_str!("../pkcs11/mod.rs");

        assert_eq!(ALGORITHM, "SHA256withRSA");
        assert!(token_side.contains("Mechanism::Sha256RsaPkcs"));
    }

    #[test]
    fn the_signature_travels_to_the_postsign_in_base64() {
        let signature = TokenSignature(vec![0x30, 0x82, 0x01, 0x00]);

        assert_eq!(signature.raw(), [0x30, 0x82, 0x01, 0x00]);
        assert_eq!(signature.to_pkcs1_base64(), "MIIBAA==");
    }

    #[test]
    fn a_seal_that_came_back_changed_is_refused_before_anything_else() {
        // No hace falta puente: la comparación es lo primero que hace la
        // postfirma, y esta prueba es la que garantiza que sigue siéndolo.
        let issued = SessionSeal::from_bridge("el sello de la prefirma");
        let tampered = SessionSeal::from_bridge("el sello de la prefirma.");

        assert!(issued.verify_unchanged(&tampered).is_err());
        assert!(issued.verify_unchanged(&issued.clone()).is_ok());
    }

    #[test]
    fn the_postsign_compares_the_seal_before_crossing_the_border() {
        // El orden importa tanto como la comparación: comparar después de
        // postfirmar sería enterarse cuando el PDF ya está mal.
        let cycle = production_half(include_str!("cycle.rs"));
        let body = cycle
            .split_once("pub fn postsign(")
            .expect("la postfirma sigue aquí")
            .1;
        let check = body
            .find("verify_unchanged")
            .expect("la postfirma comprueba el sello");
        let crossing = body.find("bridge.postsign").expect("y luego cruza");

        assert!(
            check < crossing,
            "el sello se comprueba después de cruzar: el PDF ya saldría mal"
        );
    }
}
