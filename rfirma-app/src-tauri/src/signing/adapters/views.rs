//! Los tipos de firma local que cruzan a la ventana principal (ADR-0011).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::crossing::crossing;

use crate::signing::adapters::state::VisibleSignatureMemory;
use crate::signing::application::configuration::Preferences;
use crate::signing::application::configuration_memory::Theme;
use crate::signing::domain::{
    Datum, PageSet, PhrasePart, PreviousSignature, PreviousSignaturesReport, SignatureStatus, Tone,
    VisibleBox, VisibleContent,
};

crossing! {
    /// Posición y páginas del recuadro de firma visible.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PlacementView {
        /// Coordenadas del recuadro en espacio de usuario PDF: [x0, y0, x1, y1].
        pub rect: [f64; 4],
        /// Páginas en las que estampar la firma.
        pub pages: PageSet,
    }
}

impl From<VisibleBox> for PlacementView {
    fn from(placed: VisibleBox) -> Self {
        Self {
            rect: placed.rect,
            pages: placed.pages,
        }
    }
}

impl From<PlacementView> for VisibleBox {
    fn from(view: PlacementView) -> Self {
        Self {
            rect: view.rect,
            pages: view.pages,
        }
    }
}

crossing! {
    /// Configuración de la aplicación visible para la ventana (ADR-0011).
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ConfigurationView {
        /// Idioma seleccionado.
        pub language: String,
        /// Nombre de la carpeta de destino.
        pub destination: String,
        /// Si se recuerda la última configuración de firma visible.
        pub remember_visible_signature: bool,
        /// Si se conserva el historial de actividad reciente.
        pub remember_activity: bool,
        /// Si se notifica la disponibilidad de nuevas versiones.
        pub notify_new_version: bool,
        /// Tema visual de la ventana.
        pub theme: Theme,
        /// Si la plataforma permite guardar junto al original.
        #[serde(default)]
        pub offers_the_original_folder: bool,
        /// Si el asistente del primer arranque ya se ha visto.
        pub setup_wizard_seen: bool,
        /// Si el botón de consentir de la ventana de sede espera una cuenta atrás.
        pub consent_countdown: bool,
        /// Si la sede puede elegir sola el único certificado candidato (ADR-0032).
        pub honour_automatic_selection: bool,
    }
}

impl From<Preferences> for ConfigurationView {
    fn from(preferences: Preferences) -> Self {
        Self {
            language: preferences.language,
            destination: preferences.destination,
            remember_visible_signature: preferences.remember_visible_signature,
            remember_activity: preferences.remember_activity,
            notify_new_version: preferences.notify_new_version,
            theme: preferences.theme,
            offers_the_original_folder: preferences.offers_the_original_folder,
            setup_wizard_seen: preferences.setup_wizard_seen,
            consent_countdown: preferences.consent_countdown,
            honour_automatic_selection: preferences.honour_automatic_selection,
        }
    }
}

impl From<ConfigurationView> for Preferences {
    fn from(view: ConfigurationView) -> Self {
        Self {
            language: view.language,
            destination: view.destination,
            remember_visible_signature: view.remember_visible_signature,
            remember_activity: view.remember_activity,
            notify_new_version: view.notify_new_version,
            theme: view.theme,
            offers_the_original_folder: view.offers_the_original_folder,
            setup_wizard_seen: view.setup_wizard_seen,
            consent_countdown: view.consent_countdown,
            honour_automatic_selection: view.honour_automatic_selection,
        }
    }
}

crossing! {
    /// Un dato de la frase de *Personalizada*, de vuelta a la ventana.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum DatumView {
        Signer,
        Issuer,
        SignedAt,
    }
}

impl From<Datum> for DatumView {
    fn from(datum: Datum) -> Self {
        match datum {
            Datum::Signer => Self::Signer,
            Datum::Issuer => Self::Issuer,
            Datum::SignedAt => Self::SignedAt,
        }
    }
}

crossing! {
    /// Un trozo de la frase recordada: texto literal o un dato.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(untagged)]
    pub enum PhrasePartView {
        Text { text: String },
        Datum { datum: DatumView },
    }
}

impl From<&PhrasePart> for PhrasePartView {
    fn from(part: &PhrasePart) -> Self {
        match part {
            PhrasePart::Text(text) => Self::Text { text: text.clone() },
            PhrasePart::Datum(datum) => Self::Datum {
                datum: (*datum).into(),
            },
        }
    }
}

crossing! {
    /// El contenido recordado de la firma visible, por modelo.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(tag = "model", rename_all = "camelCase")]
    pub enum VisibleContentView {
        Complete,
        RubricOnly,
        Custom { phrase: Vec<PhrasePartView> },
    }
}

impl From<&VisibleContent> for VisibleContentView {
    fn from(content: &VisibleContent) -> Self {
        match content {
            VisibleContent::Complete => Self::Complete,
            VisibleContent::RubricOnly => Self::RubricOnly,
            VisibleContent::Custom(phrase) => Self::Custom {
                phrase: phrase.iter().map(PhrasePartView::from).collect(),
            },
        }
    }
}

crossing! {
    /// Modelo, frase y «Con rúbrica» recordados de la última firma visible configurada (ADR-0010).
    #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RememberedVisibleSignatureView {
        pub content: Option<VisibleContentView>,
        pub with_rubric: bool,
    }
}

impl From<VisibleSignatureMemory> for RememberedVisibleSignatureView {
    fn from(remembered: VisibleSignatureMemory) -> Self {
        Self {
            content: remembered.content.as_ref().map(VisibleContentView::from),
            with_rubric: remembered.rubric,
        }
    }
}

crossing! {
    /// El estado de una firma previa, con el nombre con el que cruza el puente.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum SignatureStatusView {
        Valid,
        CertificateExpired,
        CertificateNotYetValid,
        Broken,
        Unverifiable,
        NotFullyChecked,
    }
}

impl From<SignatureStatus> for SignatureStatusView {
    fn from(status: SignatureStatus) -> Self {
        match status {
            SignatureStatus::Valid => Self::Valid,
            SignatureStatus::CertificateExpired => Self::CertificateExpired,
            SignatureStatus::CertificateNotYetValid => Self::CertificateNotYetValid,
            SignatureStatus::Broken => Self::Broken,
            SignatureStatus::Unverifiable => Self::Unverifiable,
            SignatureStatus::NotFullyChecked => Self::NotFullyChecked,
        }
    }
}

crossing! {
    /// El tono del peor aviso, de menor a mayor gravedad.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum ToneView {
        Information,
        Indeterminate,
        Attention,
    }
}

impl From<Tone> for ToneView {
    fn from(tone: Tone) -> Self {
        match tone {
            Tone::Information => Self::Information,
            Tone::Indeterminate => Self::Indeterminate,
            Tone::Attention => Self::Attention,
        }
    }
}

crossing! {
    /// Titular, fecha, certificado y estado de una de las firmas que ya trae el documento.
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PreviousSignatureView {
        /// El nombre del titular.
        pub name: String,
        /// El NIF del titular.
        pub id_number: String,
        /// La entidad representada, si el certificado la lleva.
        pub organization_identifier: Option<String>,
        /// La autoridad emisora del certificado.
        pub issuer: String,
        /// Número de serie del certificado.
        pub certificate_serial_number: String,
        /// Instante de la firma en ISO-8601, si el puente lo devolvió.
        pub signing_time: Option<String>,
        /// El estado de la firma.
        pub status: SignatureStatusView,
        /// Motivo del original, si el estado no es `Valid`.
        pub reason: Option<String>,
    }
}

impl From<PreviousSignature> for PreviousSignatureView {
    fn from(signature: PreviousSignature) -> Self {
        Self {
            name: signature.name,
            id_number: signature.id_number,
            organization_identifier: signature.organization_identifier,
            issuer: signature.issuer,
            certificate_serial_number: signature.certificate_serial_number,
            signing_time: signature.signing_time,
            status: SignatureStatusView::from(signature.status),
            reason: signature.reason,
        }
    }
}

crossing! {
    /// Las firmas que ya trae el documento, con quién firmó, cuándo, los avisos y su tono.
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PreviousSignaturesReportView {
        /// Una por firma, en orden cronológico.
        pub signatures: Vec<PreviousSignatureView>,
        /// Cuántos avisos deja el informe.
        pub warning_count: usize,
        /// El tono del peor aviso.
        pub tone: ToneView,
    }
}

impl From<PreviousSignaturesReport> for PreviousSignaturesReportView {
    fn from(report: PreviousSignaturesReport) -> Self {
        let warning_count = report.warning_count();
        let tone = ToneView::from(report.tone());
        Self {
            signatures: report
                .into_signatures()
                .into_iter()
                .map(PreviousSignatureView::from)
                .collect(),
            warning_count,
            tone,
        }
    }
}

crossing! {
    lent from "signing/domain/placement.rs":
    pub enum PageSet {
        All,
        Only(BTreeSet<u32>),
    }
}

crossing! {
    lent from "signing/application/configuration_memory.rs":
    pub enum Theme {
        System,
        Light,
        Dark,
    }
}
