//! Lo que pide la sede, leído de una URL `afirma://` y nada más (ID-244).
//!
//! Aquí no hay efectos: no se abre un socket, no se llama al puente y no se
//! toca el disco. Se lee una cadena y se dice qué pide —puertos sorteados,
//! versión de protocolo, credencial de canal y versión mínima exigida— o por
//! qué se rechaza. Por eso se prueba entero como función pura, igual que
//! [`crate::signing::placement`] (TD-53).
//!
//! **El parseo es de rFirma, no del puente** (ID-244): las reglas de versión y
//! de credencial se aplican sobre la URL antes de que exista canal, y la lista
//! blanca de filtros se comprueba antes de llamar a Java. Del puente se toma el
//! motor de filtros, no el parseo.
//!
//! El contrato que se reproduce está medido en
//! `docs/research/contrato-protocolo-afirma.md`, sobre el tag `v1.9.2` de
//! `clienteafirma`. Cuatro decisiones se apartan del original **a propósito**,
//! y las cuatro endurecen:
//!
//! 1. Un `idsession` mal formado se **rechaza** (`SAF_03`). El original lo pone
//!    a `null`, y un `null` desactiva la comprobación entera del canal: abre un
//!    canal sin cerradura (ID-249).
//! 2. **El protocolo 3 no existe**: su camino es puerto fijo y sin credencial, y
//!    rFirma no abre nunca un canal sin credencial (ID-247). Sólo `v=4` pasa.
//! 3. `mcv` se compara contra la versión de AutoFirma que rFirma **declara
//!    implementar** ([`version::IMPLEMENTED_AUTOFIRMA_VERSION`]), que es un
//!    número distinto de la versión de rFirma (ID-250).
//! 4. Un criterio de `filters=` **fuera de la lista blanca** se rechaza
//!    (`SAF_03`) en vez de ignorarse. El original lo descarta en silencio y
//!    sirve el listado entero, que es más ancho de lo que la sede pidió
//!    ([`filters`], ID-256).
//!
//! Y una que **no** se aparta aunque tiente: la comparación de `mcv` no es
//! semver, y se reproduce tal cual (ID-251, [`version`]).

pub mod codes;
pub mod filters;
pub mod launch;
pub mod message;
pub mod parameters;
pub mod refusal;
pub mod url;
pub mod version;

pub use codes::{Parameter, SafCode, WireAnswer, CANCELLED, NOTHING, OUT_OF_MEMORY};
pub use filters::{site_filter, SiteFilter, ACCEPTED_CRITERIA, UNMEASURED_CRITERIA};
pub use launch::{drawn_ports, ChannelCredential, LaunchRequest, PROTOCOL_VERSION};
pub use message::ChannelMessage;
pub use parameters::{check_local_access_is_not_requested, check_minimum_client_version};
pub use refusal::Refusal;
pub use url::AfirmaUrl;
pub use version::{Version, IMPLEMENTED_AUTOFIRMA_VERSION};
