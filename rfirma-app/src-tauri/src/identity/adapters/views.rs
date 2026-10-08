//! Los tipos de identidad que cruzan a la ventana: `CertificateView`, `StatusView`, `SecretView` y las noticias de los lectores (ADR-0011).

use serde::Serialize;

use crate::crossing::crossing;

use crate::identity::application::readers::ReaderNews;
use crate::identity::domain::certificate::{CertificateStatus, ListedCertificate};
use crate::identity::domain::readers::{ReaderStatus, ReadyCard};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::StoreClass;

crossing! {
    /// Estado de un certificado tal como cruza a la ventana.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum StatusView {
        #[serde(rename_all = "camelCase")]
        Valid {
            not_after: u64,
        },
        #[serde(rename_all = "camelCase")]
        Expired {
            not_after: u64,
        },
        #[serde(rename_all = "camelCase")]
        NotYetValid {
            not_before: u64,
        },
        Revoked {
            reason: String,
        },
        Unreadable {
            detail: String,
        },
    }
}

impl From<CertificateStatus> for StatusView {
    fn from(status: CertificateStatus) -> Self {
        match status {
            CertificateStatus::Valid { not_after } => Self::Valid { not_after },
            CertificateStatus::Expired { not_after } => Self::Expired { not_after },
            CertificateStatus::NotYetValid { not_before } => Self::NotYetValid { not_before },
            CertificateStatus::Revoked { reason } => Self::Revoked { reason },
            CertificateStatus::Unreadable { detail } => Self::Unreadable { detail },
        }
    }
}

crossing! {
    /// Forma de solicitar el secreto al almacén de claves.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum SecretView {
        NotNeeded,
        TypedOnScreen,
        TypedOnTheReaderKeypad,
    }
}

impl From<StoreSecret> for SecretView {
    fn from(secret: StoreSecret) -> Self {
        match secret {
            StoreSecret::NotNeeded => Self::NotNeeded,
            StoreSecret::TypedOnScreen => Self::TypedOnScreen,
            StoreSecret::TypedOnTheReaderKeypad => Self::TypedOnTheReaderKeypad,
        }
    }
}

/// Nombre en inglés de una clase de almacén para su traducción en la ventana.
pub fn store_name(class: StoreClass) -> &'static str {
    match class {
        StoreClass::Card => "card",
        StoreClass::Firefox => "firefox",
        StoreClass::Chrome => "chrome",
        StoreClass::Nssdb => "nssdb",
        StoreClass::Installed => "installed",
        StoreClass::Windows => "windows",
    }
}

crossing! {
    /// Certificado para mostrar en la lista y volver a seleccionarlo.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CertificateView {
        /// Asa opaca asignada al listar.
        pub id: String,
        pub label: String,
        pub holder_name: String,
        /// El `CN` tal y como lo estampa la firma visible.
        pub stamped_signer: String,
        /// Nombre de pila, vacío si el certificado no lo trae.
        pub given_name: String,
        /// Primer apellido, vacío si el certificado no lo trae.
        pub surname: String,
        pub id_number: String,
        /// La entidad representada (`organizationIdentifier`), o nada si el certificado no la lleva.
        pub organization_identifier: Option<String>,
        /// El nombre de la entidad representada, o nada si el certificado no es de representante.
        pub entity_name: Option<String>,
        pub issuer: String,
        /// Número de serie del certificado, en base diez.
        pub certificate_serial_number: String,
        /// Las clases de almacén donde está, por orden de preferencia.
        pub stores: Vec<String>,
        pub status: StatusView,
        /// Si alguna de sus copias fue la usada en la última firma.
        pub remembered: bool,
    }
}

impl From<ListedCertificate> for CertificateView {
    fn from(certificate: ListedCertificate) -> Self {
        Self {
            id: certificate.id,
            label: certificate.label,
            holder_name: certificate.holder_name,
            stamped_signer: certificate.stamped_signer,
            given_name: certificate.given_name,
            surname: certificate.surname,
            id_number: certificate.id_number,
            organization_identifier: certificate.organization_identifier,
            entity_name: certificate.entity_name,
            issuer: certificate.issuer,
            certificate_serial_number: certificate.certificate_serial_number,
            stores: certificate
                .stores
                .into_iter()
                .map(|class| match class {
                    StoreClass::Card if certificate.from_a_dnie => "dnie".to_owned(),
                    _ => store_name(class).to_owned(),
                })
                .collect(),
            status: certificate.status.into(),
            remembered: certificate.remembered,
        }
    }
}

crossing! {
    /// El estado que resume a los lectores de tarjetas, tal como cruza a la ventana.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum ReaderStatusView {
        Unavailable,
        NoReader,
        NoCard,
        Reading,
        DnieReady,
        CardReady,
        Unreadable,
    }
}

impl From<ReaderStatus> for ReaderStatusView {
    fn from(status: ReaderStatus) -> Self {
        match status {
            ReaderStatus::Unavailable => Self::Unavailable,
            ReaderStatus::NoReader => Self::NoReader,
            ReaderStatus::NoCard => Self::NoCard,
            ReaderStatus::Reading => Self::Reading,
            ReaderStatus::Ready(ReadyCard::Dnie) => Self::DnieReady,
            ReaderStatus::Ready(ReadyCard::Other) => Self::CardReady,
            ReaderStatus::Unreadable => Self::Unreadable,
        }
    }
}

crossing! {
    /// Lo que la ventana recibe al cambiar un lector o una tarjeta: el estado y, si ha cambiado, la lista (ADR-0048).
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ReaderNewsView {
        pub reader: ReaderStatusView,
        /// La lista de siempre, con las tarjetas de ahora; nada, si no ha cambiado.
        pub certificates: Option<Vec<CertificateView>>,
    }
}

impl From<ReaderNews> for ReaderNewsView {
    fn from(news: ReaderNews) -> Self {
        Self {
            reader: news.reader.into(),
            certificates: news
                .certificates
                .map(|rows| rows.into_iter().map(CertificateView::from).collect()),
        }
    }
}

#[cfg(test)]
mod tests;
