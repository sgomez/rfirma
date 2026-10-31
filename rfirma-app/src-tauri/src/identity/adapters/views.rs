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

crossing! {
    /// De qué almacén es una copia del certificado: una clase, o el navegador y el canal de un almacén NSS.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum StoreMarkView {
        Card,
        Dnie,
        Installed,
        Nssdb,
        Windows,
        Macos,
        /// El almacén NSS que comparten los navegadores Chromium nativos.
        SharedChromium,
        /// El almacén NSS de un navegador, con el canal por el que llegó si no es nativo.
        Browser {
            name: String,
            channel: Option<String>,
        },
    }
}

impl StoreMarkView {
    fn of(class: StoreClass, from_a_dnie: bool) -> Self {
        match class {
            StoreClass::Card if from_a_dnie => Self::Dnie,
            StoreClass::Card => Self::Card,
            StoreClass::Installed => Self::Installed,
            StoreClass::Nssdb => Self::Nssdb,
            StoreClass::Windows => Self::Windows,
            StoreClass::Macos => Self::Macos,
            StoreClass::Browser(owner) => match owner.browser_name() {
                Some(name) => Self::Browser {
                    name: name.to_owned(),
                    channel: owner.channel_name().map(str::to_owned),
                },
                None => Self::SharedChromium,
            },
        }
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
        pub stores: Vec<StoreMarkView>,
        pub status: StatusView,
        /// Si alguna de sus copias fue la usada en la última firma.
        pub remembered: bool,
        /// Si rFirma lo instaló desde un `.p12`: Preferencias solo enseña y quita estos.
        pub installed: bool,
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
                .map(|class| StoreMarkView::of(class, certificate.from_a_dnie))
                .collect(),
            status: certificate.status.into(),
            remembered: certificate.remembered,
            installed: certificate.installed,
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
        LegacyDnie,
        BurnedDnie,
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
            ReaderStatus::LegacyDnie => Self::LegacyDnie,
            ReaderStatus::BurnedDnie => Self::BurnedDnie,
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
