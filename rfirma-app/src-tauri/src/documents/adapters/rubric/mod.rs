//! Normalización de la rúbrica manuscrita y almacén entre sesiones (ADR-0012).

pub mod normalize;
pub mod store;

pub use crate::documents::domain::rubric::{NormalizedRubric, RubricError, Situation};
pub use normalize::{
    accepted_formats, normalize, AcceptedFormat, JPEG_QUALITY, MAX_DECODED_BYTES, MAX_INPUT_BYTES,
    MAX_SIDE_PX,
};
pub use store::RubricStore;
