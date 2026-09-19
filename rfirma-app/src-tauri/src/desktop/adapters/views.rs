//! Los tipos del escritorio que cruzan a la ventana principal (ADR-0011).

use serde::Serialize;

use crate::crossing::crossing;

use crate::desktop::domain::handlers::{UrlHandler, UrlHandlers};

crossing! {
    /// Estado del manejador de enlaces afirma:// en el sistema.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UrlHandlersView {
        /// Si el entorno permite consultar manejadores de protocolo.
        pub available: bool,
        /// Manejadores registrados en el escritorio.
        pub handlers: Vec<UrlHandlerView>,
        /// Manejador asignado por defecto.
        pub current: Option<String>,
        /// Identificador de escritorio de esta aplicación.
        pub ours: String,
    }
}

impl From<UrlHandlers> for UrlHandlersView {
    fn from(handlers: UrlHandlers) -> Self {
        Self {
            available: handlers.available,
            handlers: handlers
                .handlers
                .into_iter()
                .map(UrlHandlerView::from)
                .collect(),
            current: handlers.current,
            ours: handlers.ours,
        }
    }
}

crossing! {
    /// Manejador registrado para el esquema de protocolo.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UrlHandlerView {
        /// Identificador de la aplicación en el escritorio.
        pub id: String,
        /// Nombre visible de la aplicación.
        pub name: String,
    }
}

impl From<UrlHandler> for UrlHandlerView {
    fn from(handler: UrlHandler) -> Self {
        Self {
            id: handler.id,
            name: handler.name,
        }
    }
}

crossing! {
    /// Notificación de nueva versión disponible.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NewVersionView {
        /// Versión publicada.
        pub version: String,
    }
}

use crate::desktop::domain::status::{
    ActionKind, Signal, SignalRow, StatusAction, StoreBrand, StoreDetail, Verdict,
};

crossing! {
    /// Fila de estado de una señal para la ventana.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SignalRowView {
        /// Señal evaluada.
        pub signal: Signal,
        /// Valor en texto plano para la celda.
        pub value: String,
        /// Veredicto calculado en Rust.
        pub verdict: Verdict,
        /// Acción disponible si la hay.
        pub action: Option<StatusActionView>,
        /// Detalle por almacén, para señales que lo despliegan.
        pub detail: Option<Vec<StoreDetailView>>,
        /// Aviso de reiniciar Firefox, tras instalar con el navegador vivo (ADR-0005).
        pub restart_firefox_notice: bool,
    }
}

impl From<SignalRow> for SignalRowView {
    fn from(row: SignalRow) -> Self {
        Self {
            signal: row.signal,
            value: row.value,
            verdict: row.verdict,
            action: row.action.map(StatusActionView::from),
            detail: row
                .detail
                .map(|detail| detail.into_iter().map(StoreDetailView::from).collect()),
            restart_firefox_notice: row.restart_firefox_notice,
        }
    }
}

crossing! {
    /// Un almacén, con su marca y si la señal es de confianza en él.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreDetailView {
        /// Marca del almacén.
        pub brand: StoreBrand,
        /// Si la señal es de confianza en este almacén.
        pub trusted: bool,
    }
}

impl From<StoreDetail> for StoreDetailView {
    fn from(detail: StoreDetail) -> Self {
        Self {
            brand: detail.brand,
            trusted: detail.trusted,
        }
    }
}

crossing! {
    lent from "desktop/domain/status.rs":
    pub enum StoreBrand {
        Firefox,
        Chrome,
        Nssdb,
    }
}

crossing! {
    /// Acción declarada que acompaña al veredicto de una señal.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StatusActionView {
        /// Tipo declarado de acción.
        pub kind: ActionKind,
        /// Destino o identificador de la acción.
        pub target: String,
    }
}

impl From<StatusAction> for StatusActionView {
    fn from(action: StatusAction) -> Self {
        Self {
            kind: action.kind,
            target: action.target,
        }
    }
}

crossing! {
    lent from "desktop/domain/status.rs":
    pub enum Signal {
        Version,
        SiteSignature,
        LocalCaCertificate,
        UserCertificates,
    }
}

crossing! {
    lent from "desktop/domain/status.rs":
    pub enum Verdict {
        Correct,
        Attention,
        Incorrect,
        NotApplicable,
        Checking,
    }
}

crossing! {
    lent from "desktop/domain/status.rs":
    pub enum ActionKind {
        Repair,
        Choice,
        Link,
    }
}
