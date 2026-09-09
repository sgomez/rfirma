//! Lo que pide la sede, leído de una URL `afirma://` y sin efectos.
//!
//! Donde rFirma se aparta del original a propósito:
//!
//! - **El lote local solo existe en JSON**. El original admite
//!   `localBatchProcess=true` con el XML heredado y lo manda de todos modos a
//!   los dos servlets (`ProtocolInvocationLauncherBatch.signBatch`, 1.9.2);
//!   aquí ese lote es un `SAF_03` que nombra `dat`.
//! - **`format=auto` sobre una factura elige FacturaE**, no XAdES: es lo que
//!   hace el original (`PreProcessorFactory.getSignFormat`, 1.9.2), y firmarla
//!   como un XML cualquiera dejaría una factura que su propia política invalida.
//!   Quien lo decide es `detection.rs`, con la misma comprobación de raíz y
//!   tres hijos que `AOFacturaESigner.isValidDataFile`.
//! - **La XAdES explícita no se reproduce**. El original avisa de que
//!   `mode=explicit` está obsoleto y hashea el dato con SHA1 antes de firmar
//!   (`ProtocolInvocationLauncherSign.java:390-405`); aquí `mode=explicit`
//!   con XAdES sale con `SAF_06`.
//! - **El Base64 del servidor intermedio rechaza un carácter fuera del
//!   alfabeto**. El original nunca lanza su «Bad Base64 input character»:
//!   ningún valor de su tabla queda por debajo del umbral que lo dispara
//!   (`Base64.decode`, 1.9.2), así que ignora cualquier byte extraño y ante
//!   una página de error del servlet devolvería basura. Aquí se ignoran los
//!   espacios en blanco y lo que siga al relleno, como el original, pero
//!   cualquier otro carácter sale con `SAF_15`.
//! - **XMLDSig no se atiende**. El original lo firma monofásico con la clave
//!   privada dentro de Java (`AOXMLDSigSigner`, `afirma-crypto-xmlsignature`)
//!   y su `PreProcessorFactory` no tiene preprocesador trifásico para él
//!   (1.9.2); atenderlo exigiría la clave dentro de Java, que prohíbe el
//!   ADR-0001, así que `format=XMLDSig*` sale con `SAF_06`.

pub mod algorithm;
pub mod cipher;
pub mod codes;
pub mod data_source;
pub mod detection;
pub mod filters;
pub mod format;
pub mod framing;
pub mod launch;
pub mod message;
pub mod operation;
pub mod parameters;
pub mod refusal;
pub mod relay_parameters;
pub mod url;
pub mod version;
pub mod visible;

pub use algorithm::AskedAlgorithm;
pub use cipher::{cipher as encrypt, decipher as decrypt, CipherKey};
pub use codes::{Parameter, SafCode, WireAnswer, CANCELLED, NOTHING, OUT_OF_MEMORY};
pub use data_source::{download_url, DataSource};
pub use detection::{shape_of, DetectedShape};
pub use filters::{site_filter, SiteFilter, ACCEPTED_CRITERIA, UNMEASURED_CRITERIA};
pub use format::{format_of, RequestedFormat, XadesEnvelope};
pub use framing::{
    credential_matches, http_response, read_request, split_response, FragmentBuffer, FramedRequest,
    NotOfTheFraming, MORE_DATA_NEED, RESPONSE_MAX_SIZE,
};
pub use launch::{
    asks_for_active_wait, drawn_ports, location_for_a_refusal, ChannelCredential, LaunchRequest,
    NegotiatedCredential, RelayChannelInfo, RelayRequest, PROTOCOL_VERSION,
    THE_PORT_OF_THE_THIRD_PROTOCOL, THIRD_PROTOCOL_VERSION,
};
pub use message::ChannelMessage;
pub use operation::{
    pairs_of, read_operation, refuse_a_countersignature_outside_cades_and_xades,
    refuse_a_multisignature_of_an_invoice, refuse_explicit_xades, BatchRequest, CounterTarget,
    LoadRequest, PendingSignRequest, SaveRequest, SelectCertificate, SignAndSaveRequest,
    SignRequest, SignatureRound, SiteOperation, AUTO, BATCH, COSIGN, COUNTERSIGN, LOAD, SAVE,
    SELECT_CERTIFICATE, SIGN, SIGN_AND_SAVE,
};
pub use parameters::{
    check_local_access_is_not_requested, check_minimum_client_version, sticky_certificate,
    StickyCertificate,
};
pub use refusal::{Refusal, RefusalSituation};
pub use relay_parameters::operation_of_the_parameters_xml;
pub use url::AfirmaUrl;
pub use version::{Version, IMPLEMENTED_AUTOFIRMA_VERSION};
pub use visible::{forget_the_box, visible_signature_of, SiteVisibleSignature};
