//! Reglas de firma puras sin efectos secundarios.

pub mod admissibility;
pub mod bridge;
pub mod config;
pub mod isolate_gone;
pub mod language;
pub mod layer2_text;
pub mod placement;
pub mod properties;
pub mod session_seal;

pub use admissibility::{AdmissibleDocument, Refusal};
pub use bridge::{CompletedCycle, Format, SealedPreSignature, TokenSignature};
pub use config::{
    ChosenFields, PadesRect, Placement, Setting, SignatureConfig, SigningChoice,
    ALLOW_UNREGISTERED_KEY, SUB_FILTER,
};
pub use language::Language;
pub use layer2_text::{compose_layer2_text, mask_id_number, VisibleTextFields};
pub use placement::{
    BoxSize, MediaBox, OutOfDocument, OutOfPage, Page, PageSet, PlacementError, Rotation, Spot,
    UserSpaceRect, ViewerRect, VisibleBox,
};
pub use properties::{merged_with, to_java_properties};
pub use session_seal::{SealMismatch, SessionSeal};
