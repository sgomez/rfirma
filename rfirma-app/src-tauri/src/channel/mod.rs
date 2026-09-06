//! **El canal**: la conexión `wss://` que la sede abre contra el servidor
//! local, y lo que hace falta para sostenerla (ADR-0005, ID-212…ID-219).
//!
//! Cuatro piezas y ninguna decisión de trámite:
//!
//! | Módulo | Qué hace |
//! |---|---|
//! | [`bind`] | ata uno de los puertos que sorteó la sede, y nunca el 63117 |
//! | [`server`] | levanta el WebSocket sobre TLS y acepta conexiones |
//! | [`conversation`] | qué se contesta a cada mensaje, sin socket delante |
//! | [`reply`] | el asa por la que se contesta la operación que quedó pendiente |
//!
//! **No existe escuchador en claro**: no hay ruta que sirva `ws://` (ID-212).
//!
//! Este módulo es **infraestructura**: no sabe por qué se abre el canal ni qué
//! se hace con lo que llega. Quien lo decide es el caso de uso
//! ([`crate::app::site`]), que lo usa a través de un puerto de transporte
//! (ID-214) —la única costura nueva del hito (TD-51)—, y por eso ninguna prueba
//! de `app/` abre un socket (TD-52).

pub mod bind;
pub mod conversation;
pub mod error;
pub mod reply;
pub mod server;

pub use bind::{bind_first_free, THE_PORT_OF_THE_THIRD_PROTOCOL};
pub use conversation::{answer, Answer, ChannelDuty, ECHO_OK};
pub use error::{ChannelError, Situation};
pub use reply::ReplyHandle;
pub use server::{open, serve, OpenChannel, Shutdown, SiteOperations};
