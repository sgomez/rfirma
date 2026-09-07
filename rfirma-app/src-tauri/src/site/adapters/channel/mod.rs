//! Conexión `wss://` local con la sede para la recepción de operaciones (ADR-0005).

pub mod bind;
pub mod conversation;
pub mod reply;
pub mod server;

pub use bind::{bind_first_free, THE_PORT_OF_THE_THIRD_PROTOCOL};
pub use conversation::{answer, Answer, ECHO_OK};
pub use reply::ReplyHandle;
pub use server::{open, serve, SiteOperations};
