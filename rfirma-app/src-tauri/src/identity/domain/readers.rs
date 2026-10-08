//! Los lectores de tarjetas y el estado que la ventana enseña de ellos: el de cada uno y el que los resume.

use crate::identity::domain::holder::attribute;

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
            Self::NoReader => 0,
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

/// Si el emisor es una CA del DNIe: `CN=AC DNIE *`, `OU=DNIE`, la Dirección General de la Policía y `C=ES`.
pub fn is_issued_for_a_dnie(issuer: &str) -> bool {
    attribute("CN=", issuer).starts_with("AC DNIE ")
        && attribute("OU=", issuer) == "DNIE"
        && attribute("O=", issuer) == "DIRECCION GENERAL DE LA POLICIA"
        && attribute("C=", issuer) == "ES"
}

#[cfg(test)]
mod tests;
