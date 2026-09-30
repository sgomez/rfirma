//! Los tipos del escritorio que cruzan a la ventana principal (ADR-0011).

use serde::{Deserialize, Serialize};

use crate::crossing::crossing;

crossing! {
    /// Notificación de nueva versión disponible.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NewVersionView {
        /// Versión publicada.
        pub version: String,
        /// Si se puede instalar desde la aplicación.
        pub installable: bool,
    }
}

use crate::desktop::domain::installation::Installation;

crossing! {
    /// Lo que pasó al instalar la versión anunciada.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum InstallationView {
        Installed,
        NoUpdate,
        NetworkFailure,
        InvalidSignature,
        NotAvailable,
    }
}

impl From<Installation> for InstallationView {
    fn from(installation: Installation) -> Self {
        match installation {
            Installation::Installed => Self::Installed,
            Installation::NoUpdate => Self::NoUpdate,
            Installation::NetworkFailure => Self::NetworkFailure,
            Installation::InvalidSignature => Self::InvalidSignature,
            Installation::NotAvailable => Self::NotAvailable,
        }
    }
}

use crate::desktop::domain::status::{
    ActionKind, Signal, SignalDetail, SignalRow, SiteSignatureCandidate, StatusAction, StoreBrand,
    StoreCertificates, StoreDetail, Verdict,
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
        /// Detalle desplegable, para las señales que lo tienen.
        pub detail: Option<SignalDetailView>,
        /// Candidatas entre las que elegir, para la señal `Firma en sedes`.
        pub candidates: Option<Vec<SiteSignatureCandidateView>>,
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
            detail: row.detail.map(SignalDetailView::from),
            candidates: row.candidates.map(|candidates| {
                candidates
                    .into_iter()
                    .map(SiteSignatureCandidateView::from)
                    .collect()
            }),
            restart_firefox_notice: row.restart_firefox_notice,
        }
    }
}

crossing! {
    /// Candidata a firmar en sedes, para el desplegable de la señal `Firma en sedes`.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SiteSignatureCandidateView {
        /// Identificador del manejador en el escritorio.
        pub id: String,
        /// Nombre visible, tal y como lo dio el escritorio.
        pub name: String,
        /// Si es la candidata elegida hoy.
        pub selected: bool,
    }
}

impl From<SiteSignatureCandidate> for SiteSignatureCandidateView {
    fn from(candidate: SiteSignatureCandidate) -> Self {
        Self {
            id: candidate.id,
            name: candidate.name,
            selected: candidate.selected,
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
    /// Un sitio y cuántos certificados firmables propios tiene.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreCertificatesView {
        /// Marca del sitio.
        pub brand: StoreBrand,
        /// Cuántos certificados firmables hay en él.
        pub certificates: usize,
    }
}

impl From<StoreCertificates> for StoreCertificatesView {
    fn from(store: StoreCertificates) -> Self {
        Self {
            brand: store.brand,
            certificates: store.certificates,
        }
    }
}

crossing! {
    /// Lo que cuelga de una señal: dónde se confía en la CA, o cuántos certificados hay en cada sitio.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum SignalDetailView {
        Trust { stores: Vec<StoreDetailView> },
        Certificates { stores: Vec<StoreCertificatesView> },
    }
}

impl From<SignalDetail> for SignalDetailView {
    fn from(detail: SignalDetail) -> Self {
        match detail {
            SignalDetail::Trust { stores } => Self::Trust {
                stores: stores.into_iter().map(StoreDetailView::from).collect(),
            },
            SignalDetail::Certificates { stores } => Self::Certificates {
                stores: stores
                    .into_iter()
                    .map(StoreCertificatesView::from)
                    .collect(),
            },
        }
    }
}

crossing! {
    lent from "desktop/domain/status.rs":
    pub enum StoreBrand {
        Firefox,
        Chrome,
        Nssdb,
        Card,
        Installed,
        Windows,
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

use crate::desktop::domain::withdrawal::{StoreWithdrawal, Withdrawal, WithdrawalReport};

crossing! {
    /// Qué pasó al retirar algo propio de rFirma de un sitio del sistema.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum WithdrawalView {
        Withdrawn,
        WasNotThere,
        Failed(String),
    }
}

impl From<Withdrawal> for WithdrawalView {
    fn from(withdrawal: Withdrawal) -> Self {
        match withdrawal {
            Withdrawal::Withdrawn => Self::Withdrawn,
            Withdrawal::WasNotThere => Self::WasNotThere,
            Withdrawal::Failed(reason) => Self::Failed(reason),
        }
    }
}

impl From<WithdrawalView> for Withdrawal {
    fn from(view: WithdrawalView) -> Self {
        match view {
            WithdrawalView::Withdrawn => Self::Withdrawn,
            WithdrawalView::WasNotThere => Self::WasNotThere,
            WithdrawalView::Failed(reason) => Self::Failed(reason),
        }
    }
}

crossing! {
    /// Un almacén NSS con el resultado de retirar de él la CA local de rFirma.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StoreWithdrawalView {
        /// Marca del almacén.
        pub brand: StoreBrand,
        /// Resultado de la retirada en este almacén.
        pub outcome: WithdrawalView,
    }
}

impl From<StoreWithdrawal> for StoreWithdrawalView {
    fn from(store: StoreWithdrawal) -> Self {
        Self {
            brand: store.brand,
            outcome: store.outcome.into(),
        }
    }
}

impl From<StoreWithdrawalView> for StoreWithdrawal {
    fn from(view: StoreWithdrawalView) -> Self {
        Self {
            brand: view.brand,
            outcome: view.outcome.into(),
        }
    }
}

crossing! {
    /// Resultado de retirar rFirma: el manejador de sedes y la CA local de cada almacén.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WithdrawalReportView {
        /// Resultado de quitar rFirma como manejador de `afirma://`.
        pub handler: WithdrawalView,
        /// Resultado por almacén de retirar la CA local.
        pub stores: Vec<StoreWithdrawalView>,
    }
}

impl From<WithdrawalReport> for WithdrawalReportView {
    fn from(report: WithdrawalReport) -> Self {
        Self {
            handler: report.handler.into(),
            stores: report.stores.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<WithdrawalReportView> for WithdrawalReport {
    fn from(view: WithdrawalReportView) -> Self {
        Self {
            handler: view.handler.into(),
            stores: view.stores.into_iter().map(Into::into).collect(),
        }
    }
}

crossing! {
    /// El control de la barra de título nativa que se ha pulsado.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(tag = "action", rename_all = "camelCase")]
    pub enum TitlebarActionView {
        Open,
        Status,
        Preferences,
        Feedback,
        About,
        ClearRecents,
        Recent { path: String },
    }
}

crossing! {
    /// Las etiquetas de la barra de título nativa, ya traducidas por la ventana.
    #[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TitlebarLabelsView {
        /// Texto del botón de abrir.
        pub open: String,
        /// Tooltip del botón de abrir, con su atajo.
        pub open_tooltip: String,
        /// Tooltip y nombre accesible del botón de aviso.
        pub warning: String,
        /// Tooltip y nombre accesible del botón del menú.
        pub menu: String,
        /// Entrada del estado de rFirma.
        pub status: String,
        /// Entrada de Preferencias.
        pub preferences: String,
        /// Entrada de comentarios y ayuda.
        pub feedback: String,
        /// Entrada de «Acerca de».
        pub about: String,
        /// Rótulo de la sección de recientes.
        pub recents: String,
        /// Entrada que vacía la lista de recientes.
        pub clear_recents: String,
        /// Sufijo de un reciente que ya no está en disco.
        pub not_found: String,
    }
}

crossing! {
    /// Un reciente de la barra de título nativa, con lo que pinta su entrada.
    #[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TitlebarRecentView {
        /// Ruta que la ventana recibe al elegirlo.
        pub path: String,
        /// Nombre del documento.
        pub name: String,
        /// Carpeta que lo contiene.
        pub folder: String,
        /// Si el documento está firmado.
        pub signed: bool,
        /// Si el fichero se sigue encontrando.
        pub found: bool,
    }
}

crossing! {
    /// El estado completo de la barra de título nativa que manda la ventana.
    #[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TitlebarStateView {
        /// Si se ve el botón de abrir.
        pub open_visible: bool,
        /// Si se ve el botón de aviso.
        pub warning_visible: bool,
        /// Las etiquetas ya traducidas.
        pub labels: TitlebarLabelsView,
        /// Los recientes, del más nuevo al más viejo.
        pub recents: Vec<TitlebarRecentView>,
    }
}
