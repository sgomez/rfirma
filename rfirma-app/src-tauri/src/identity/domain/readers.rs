//! Los lectores de tarjetas y el estado que la ventana enseña de ellos: el de cada uno y el que los resume.

use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::is_a_card_key_provider;

/// Un lector tal como lo ve PC/SC: su nombre y si tiene una tarjeta dentro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reader {
    /// El nombre del lector.
    pub name: String,
    /// Si hay una tarjeta en el lector.
    pub has_a_card: bool,
}

/// Qué tarjeta ha quedado lista para firmar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyCard {
    /// Un DNIe, reconocido por el emisor de sus certificados.
    Dnie,
    /// Cualquier otra tarjeta.
    Other,
}

/// El estado de un lector, o el que resume a varios.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderStatus {
    /// Esta versión no vigila lectores: no se sabe si los hay.
    Unavailable,
    /// No hay ningún lector, o PC/SC no responde.
    NoReader,
    /// Hay lector, pero sin tarjeta.
    NoCard,
    /// Se están listando los almacenes de tarjeta.
    Reading,
    /// La tarjeta enseña sus certificados.
    Ready(ReadyCard),
    /// Hay tarjeta, pero ningún almacén de tarjeta la enseña.
    Unreadable,
}

/// Dónde está el listado de los almacenes de tarjeta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Listing {
    /// Todavía no ha terminado.
    InProgress,
    /// Ha terminado: la tarjeta que ha enseñado certificados, o ninguna.
    Done(Option<ReadyCard>),
}

impl ReaderStatus {
    /// El más avanzado: leyendo, lista, ilegible, sin tarjeta; sin ninguno, sin lector.
    pub fn most_advanced(statuses: impl IntoIterator<Item = Self>) -> Self {
        statuses
            .into_iter()
            .max_by_key(|status| status.advance())
            .unwrap_or(Self::NoReader)
    }

    fn advance(self) -> u8 {
        match self {
            Self::Unavailable | Self::NoReader => 0,
            Self::NoCard => 1,
            Self::Unreadable => 2,
            Self::Ready(ReadyCard::Other) => 3,
            Self::Ready(ReadyCard::Dnie) => 4,
            Self::Reading => 5,
        }
    }
}

/// El estado de los lectores según lo que tienen dentro y dónde está el listado.
pub fn status_of(readers: &[Reader], listing: Listing) -> ReaderStatus {
    ReaderStatus::most_advanced(readers.iter().map(|reader| {
        if !reader.has_a_card {
            return ReaderStatus::NoCard;
        }
        match listing {
            Listing::InProgress => ReaderStatus::Reading,
            Listing::Done(Some(card)) => ReaderStatus::Ready(card),
            Listing::Done(None) => ReaderStatus::Unreadable,
        }
    }))
}

/// La tarjeta que enseñan los certificados leídos de los almacenes de tarjeta; sin ninguno, ninguna.
pub fn ready_card_among(on_the_cards: &[TokenCertificate]) -> Option<ReadyCard> {
    if on_the_cards.is_empty() {
        return None;
    }
    let a_dnie = on_the_cards.iter().any(TokenCertificate::is_from_a_dnie);
    Some(if a_dnie {
        ReadyCard::Dnie
    } else {
        ReadyCard::Other
    })
}

/// Sin las copias que Windows guarda de una tarjeta que ya no está: las que su proveedor de tarjeta no enseña ahora (ADR-0048).
pub fn with_the_cards_present(
    found: Vec<TokenCertificate>,
    on_the_cards: &[Vec<u8>],
) -> Vec<TokenCertificate> {
    found
        .into_iter()
        .filter(|certificate| {
            let copied_from_a_card = certificate
                .reference()
                .key_provider()
                .is_some_and(is_a_card_key_provider);
            !copied_from_a_card || on_the_cards.iter().any(|der| der == certificate.der())
        })
        .collect()
}

#[cfg(test)]
mod tests;
