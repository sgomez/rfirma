//! Las reglas puras de la firma: qué se le pide al puente y qué se le exige de
//! vuelta (#50).
//!
//! Aquí no hay efectos. No se abre un fichero, no se habla con el token, no se
//! dibuja nada y no se llama a la librería nativa: son las reglas que convierten
//! lo que el usuario ha marcado en una configuración de firma, más la invariante
//! que impide que la postfirma invalide la firma en silencio. Por eso se prueba
//! entera en el carril rápido (grada A).

pub mod admissibility;
pub mod config;
pub mod cycle;
pub mod language;
pub mod layer2_text;
pub mod placement;
pub mod properties;
pub mod session_seal;

pub use admissibility::{AdmissibleDocument, Refusal};
pub use config::{Setting, SignatureBox, SignatureConfig, SUB_FILTER};
pub use cycle::{presign, CycleError, OpenCycle, SigningRequest, TokenSignature, ALGORITHM};
pub use language::Language;
pub use layer2_text::{compose_layer2_text, mask_id_number, VisibleTextFields};
pub use placement::{MediaBox, OutOfPage, Page, Rotation, UserSpaceRect, ViewerRect};
pub use properties::to_java_properties;
pub use session_seal::{SealMismatch, SessionSeal};
