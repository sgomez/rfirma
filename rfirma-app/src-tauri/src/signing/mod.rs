//! Las reglas puras de la firma: qué se le pide al puente y qué se le exige de
//! vuelta (#50).
//!
//! Aquí no hay efectos. No se abre un fichero, no se habla con el token, no se
//! dibuja nada y no se llama a la librería nativa: son las reglas que convierten
//! lo que el usuario ha marcado en una configuración de firma, más la invariante
//! que impide que la postfirma invalide la firma en silencio. Por eso se prueba
//! entera en el carril rápido (grada A).
//!
//! Este módulo **no importa [`crate::ffi`]** y no debe volver a hacerlo
//! (ID-82): el ciclo trifásico, que sí cruza la frontera, es un caso de uso y
//! vive en [`crate::app::cycle`]. La frontera sí importa de aquí el sello de
//! sesión, que es infraestructura mirando al dominio y es la dirección correcta
//! (ID-81). Lo vigila `tests/module_directions.rs`.

pub mod admissibility;
pub mod config;
pub mod language;
pub mod layer2_text;
pub mod placement;
pub mod properties;
pub mod session_seal;

pub use admissibility::{AdmissibleDocument, Refusal};
pub use config::{
    PadesRect, Placement, Setting, SignatureConfig, ALLOW_UNREGISTERED_KEY, SUB_FILTER,
};
pub use language::Language;
pub use layer2_text::{compose_layer2_text, mask_id_number, VisibleTextFields};
pub use placement::{
    MediaBox, OutOfDocument, OutOfPage, Page, PageSet, Rotation, UserSpaceRect, ViewerRect,
};
pub use properties::to_java_properties;
pub use session_seal::{SealMismatch, SessionSeal};
