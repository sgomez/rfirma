//! Lo que pide la sede, leído de una URL `afirma://` y sin efectos.

pub mod codes;
pub mod filters;
pub mod launch;
pub mod message;
pub mod operation;
pub mod parameters;
pub mod refusal;
pub mod url;
pub mod version;
pub mod visible;

pub use codes::{Parameter, SafCode, WireAnswer, CANCELLED, NOTHING, OUT_OF_MEMORY};
pub use filters::{site_filter, SiteFilter, ACCEPTED_CRITERIA, UNMEASURED_CRITERIA};
pub use launch::{drawn_ports, ChannelCredential, LaunchRequest, PROTOCOL_VERSION};
pub use message::ChannelMessage;
pub use operation::{
    pairs_of, read_operation, SelectCertificate, SignRequest, SignatureRound, SiteOperation,
    ACCEPTED_ALGORITHMS, COSIGN, COUNTERSIGN, PADES, SAVE, SELECT_CERTIFICATE, SIGN, SIGN_AND_SAVE,
};
pub use parameters::{check_local_access_is_not_requested, check_minimum_client_version};
pub use refusal::{Refusal, RefusalSituation};
pub use url::AfirmaUrl;
pub use version::{Version, IMPLEMENTED_AUTOFIRMA_VERSION};
pub use visible::{visible_signature_of, SiteVisibleSignature};
