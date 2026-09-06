//! Tipos de salida que cruzan hacia la ventana principal, reexportados por contexto (ADR-0011).

pub mod desktop;
pub mod documents;
pub mod identity;
pub mod signing;

pub use super::failure::Failure;
pub use desktop::{NewVersionView, UrlHandlerView, UrlHandlersView};
pub use documents::{
    DestinationView, DroppedDocumentView, OpenedDocumentView, RecentDocumentView,
    SignedDocumentView,
};
pub use identity::{store_name, CertificateView, SecretView};
pub use signing::{ConfigurationView, PlacementView, StatusView};
