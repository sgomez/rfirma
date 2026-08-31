//! La rúbrica: de lo que el usuario aporta a lo único que el puente acepta
//! (#52).
//!
//! El usuario trae un PNG o un JPEG cualquiera y aquí sale un **JPEG opaco y
//! sin perfil ICC**, guardado en el almacén de la aplicación. No es una
//! comodidad: la librería nativa es de un solo fichero porque se excluyó del
//! puente el módulo que normalizaba imágenes en Java, y con él se fue el
//! subárbol de `javax.imageio` (ID-08, ADR-0012). El precio es que la
//! normalización pasa a Rust; el beneficio es doble, la lista de formatos deja
//! de estar congelada en tiempo de construcción y `libawt.so` no está en el
//! directorio —que es lo que convierte un JPEG con perfil ICC de aborto del
//! proceso en error recuperable.
//!
//! El puente **exige** recibirla ya así, y por eso su mensaje «la rúbrica no
//! está codificada en JPEG» es el comportamiento **correcto**, no un fallo que
//! haya que ablandar: significa que una imagen llegó a la firma sin pasar por
//! [`normalize`], y ese camino no debe existir.
//!
//! Dos cosas que parecen bugs y no lo son están explicadas donde se hacen:
//! el fondo blanco de un PNG con alfa, en
//! [`normalize::flatten_onto_white`](normalize), y la ausencia de perfil ICC,
//! en [`normalize`].

pub mod error;
pub mod normalize;
pub mod store;

pub use error::{RubricError, Situation};
pub use normalize::{
    accepted_formats, normalize, AcceptedFormat, NormalizedRubric, JPEG_QUALITY, MAX_INPUT_BYTES,
    MAX_SIDE_PX,
};
pub use store::RubricStore;
