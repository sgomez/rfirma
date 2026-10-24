//! `DnieToken`, el DNIe leído con rdnie detrás del puerto `Token`: lista sin PIN, da los intentos restantes y firma el resumen, abriendo la tarjeta en cada operación y nunca entre ellas (ADR-0040, ADR-0047).

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rdnie::{HashAlgorithm, PcscTransport, Pin, Session, Transport, INTERMEDIATE_CA_LABEL};

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::ecdsa;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::seen_dnie::{DnieInTheReader, SeenDnie};
use crate::identity::domain::store::{Store, DNIE_STORE_PREFIX};
use crate::identity::ports::Token;

const TOKEN_LABEL: &str = "DNIe";
const PIN_CHECK: &[u8] = b"rfirma: comprobacion del PIN";

/// El almacén del DNIe: de clase tarjeta, sin módulo PKCS#11 detrás.
pub fn dnie_store() -> Store {
    Store::module(format!("{DNIE_STORE_PREFIX}pcsc"))
}

/// Quien abre la tarjeta para cada operación: el lector en producción, el emulador en las pruebas.
pub trait CardOpener: Send + Sync {
    /// Por dónde viajan las órdenes a la tarjeta.
    type Card: Transport;

    /// Una sesión nueva con el DNIe.
    fn open(&self) -> Result<Session<Self::Card>, rdnie::Error>;

    /// El lector donde rdnie reconoce un DNIe, sin abrir su canal seguro; ninguno si no lo sabe.
    fn claimed_reader(&self) -> Option<String> {
        None
    }

    /// Los lectores con una tarjeta dentro, según PC/SC; ninguno si no lo sabe.
    fn readers_with_a_card(&self) -> Option<Vec<String>> {
        None
    }
}

/// El primer lector PC/SC con un DNIe, con la sesión normal de rdnie.
#[derive(Clone, Copy, Debug, Default)]
pub struct PcscCard;

impl CardOpener for PcscCard {
    type Card = PcscTransport;

    fn open(&self) -> Result<Session<PcscTransport>, rdnie::Error> {
        Session::open(PcscTransport::connect_first_dnie()?)
    }

    fn claimed_reader(&self) -> Option<String> {
        let card = PcscTransport::connect_first_dnie().ok()?;
        rdnie_opens(&card.atr().ok()?).then(|| card.reader())
    }

    fn readers_with_a_card(&self) -> Option<Vec<String>> {
        let readers = super::pcsc::survey_readers()?;
        Some(
            readers
                .into_iter()
                .filter(|reader| reader.has_a_card)
                .map(|reader| reader.name)
                .collect(),
        )
    }
}

/// Si el ATR es de un DNIe que abre rdnie: un 3.0 o 4.0 sin quemar; el 1.0 y el 2.0 siguen por PKCS#11.
fn rdnie_opens(atr: &[u8]) -> bool {
    rdnie::is_dnie(atr) && !atr.ends_with(&BURNED_STATUS) && atr.get(OS_VERSION) == Some(&DNIE_3_OS)
}

const BURNED_STATUS: [u8; 2] = [0x65, 0x81];
const OS_VERSION: usize = 15;
const DNIE_3_OS: u8 = 0x04;

/// El adaptador de [`Token`] para el almacén del DNIe.
#[derive(Debug, Default)]
pub struct DnieToken<O> {
    opener: O,
    login: Mutex<Login>,
    turn: Mutex<()>,
}

/// Lo que el adaptador recuerda entre llamadas, sin retener la tarjeta.
#[derive(Debug, Default)]
struct Login {
    shown_attempts: Option<u8>,
    depth: usize,
    cut: Option<TokenError>,
    readings: u64,
    last_reading: Option<Reading>,
}

/// Lo último leído de la tarjeta: los titulares y, con la CA intermedia, las cadenas que aún le deben sus listados.
#[derive(Debug)]
struct Reading {
    number: u64,
    store: Store,
    holders: Vec<TokenCertificate>,
    every: Option<Vec<TokenCertificate>>,
    chains_owed: usize,
}

impl<O: CardOpener> DnieToken<O> {
    /// El DNIe que abre `opener`.
    pub fn new(opener: O) -> Self {
        Self {
            opener,
            login: Mutex::new(Login::default()),
            turn: Mutex::new(()),
        }
    }

    /// El lector donde rdnie reconoce este DNIe, sin abrir su canal seguro.
    pub fn claimed_reader(&self) -> Option<String> {
        self.opener.claimed_reader()
    }

    /// Los lectores con una tarjeta dentro; ninguno si no se sabe.
    pub fn readers_with_a_card(&self) -> Option<Vec<String>> {
        self.opener.readers_with_a_card()
    }

    /// Cuántas veces se ha leído la tarjeta: lo que apunta un listado al empezar.
    pub fn readings(&self) -> u64 {
        self.login().readings
    }

    /// Los titulares para un listado que empezó tras `arrived` lecturas: de la que acabó después, si la hay.
    pub fn list_since(
        &self,
        store: &Store,
        arrived: u64,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        let _turn = self.card_turn();
        if let Some(holders) = self.reading_since(store, arrived) {
            return Ok(holders);
        }
        self.read_holders(store)
    }

    fn login(&self) -> MutexGuard<'_, Login> {
        self.login
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Abrir una sesión resetea la tarjeta: con el turno, ninguna corta a otra a medias.
    fn card_turn(&self) -> MutexGuard<'_, ()> {
        self.turn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn session(&self) -> Result<Session<O::Card>, TokenError> {
        self.opener.open().map_err(|error| token_error(&error))
    }

    fn attempts_left(&self) -> Result<u8, TokenError> {
        match self.session()?.pin_retries() {
            Err(rdnie::Error::PinBlocked) => Ok(0),
            read => read.map_err(|error| token_error(&error)),
        }
    }

    /// rdnie no expone un VERIFY suelto: el PIN se comprueba con una firma que nunca sale de aquí (ADR-0047).
    fn verified_by_a_discarded_signature(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        shown: u8,
    ) -> Result<(), TokenError> {
        self.signed(
            reference,
            secret,
            SignatureAlgorithm::Sha256Rsa,
            PIN_CHECK,
            Some(shown),
        )
        .map(|_| ())
    }

    fn signed(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
        shown: Option<u8>,
    ) -> Result<Vec<u8>, TokenError> {
        let hash = hash_of(algorithm)?;
        let digest = ecdsa::digest(algorithm, data)?;
        let pin = pin_of(secret)?;
        let mut session = self.session()?;
        let key = session
            .certificates()
            .map_err(|error| token_error(&error))?
            .into_iter()
            .find(|certificate| certificate.label == reference.label())
            .ok_or_else(|| {
                TokenError::new(
                    Situation::CertificateNotFound,
                    format!("el DNIe no tiene la clave de {}", reference.label()),
                )
            })?
            .key;
        let left = session.pin_retries().map_err(|error| token_error(&error))?;
        unchanged_since_shown(left, shown)?;
        session
            .sign(&digest, hash, &key, pin)
            .map_err(|error| token_error(&error))
    }

    /// La lectura que acabó después de llegar el listado `arrived`, que vale tanto como una nueva.
    fn reading_since(&self, store: &Store, arrived: u64) -> Option<Vec<TokenCertificate>> {
        let mut login = self.login();
        let reading = login.last_reading.as_mut()?;
        if reading.number <= arrived || &reading.store != store {
            return None;
        }
        reading.chains_owed += 1;
        Some(reading.holders.clone())
    }

    /// Titulares y CA intermedia en una sola sesión; todo lo leído se queda para la cadena de cada listado que lo use.
    fn read_holders(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        let Some(mut session) = self.open_if_readable()? else {
            return Ok(Vec::new());
        };
        let holders = holders_of(store, &mut session)?;
        let intermediate = session
            .certificate(INTERMEDIATE_CA_LABEL)
            .ok()
            .flatten()
            .map(|der| in_the_store(store, INTERMEDIATE_CA_LABEL, der));
        let mut login = self.login();
        login.readings += 1;
        login.last_reading = Some(Reading {
            number: login.readings,
            store: store.clone(),
            holders: holders.clone(),
            every: intermediate.map(|intermediate| {
                let mut every = holders.clone();
                every.push(intermediate);
                every
            }),
            chains_owed: 1,
        });
        Ok(holders)
    }

    fn chain_owed(&self, store: &Store) -> Option<Vec<TokenCertificate>> {
        let mut login = self.login();
        let reading = login.last_reading.as_mut()?;
        if reading.chains_owed == 0 || &reading.store != store {
            return None;
        }
        reading.chains_owed -= 1;
        reading.every.clone()
    }

    fn read_every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if let Some(every) = self.chain_owed(store) {
            return Ok(every);
        }
        let Some(mut session) = self.open_if_readable()? else {
            return Ok(Vec::new());
        };
        let mut found = holders_of(store, &mut session)?;
        let intermediate = session
            .certificate(INTERMEDIATE_CA_LABEL)
            .map_err(|error| token_error(&error))?;
        found.extend(intermediate.map(|der| in_the_store(store, INTERMEDIATE_CA_LABEL, der)));
        Ok(found)
    }

    fn open_if_readable(&self) -> Result<Option<Session<O::Card>>, TokenError> {
        match self.opener.open() {
            Ok(session) => Ok(Some(session)),
            Err(error) if is_no_readable_dnie(&error) => Ok(None),
            Err(error) => Err(token_error(&error)),
        }
    }
}

fn holders_of<T: Transport>(
    store: &Store,
    session: &mut Session<T>,
) -> Result<Vec<TokenCertificate>, TokenError> {
    Ok(session
        .certificates()
        .map_err(|error| token_error(&error))?
        .into_iter()
        .map(|certificate| in_the_store(store, &certificate.label, certificate.der))
        .collect())
}

/// La versión de rdnie que fija `Cargo.lock`.
pub const RDNIE_VERSION: &str = env!("RFIRMA_RDNIE_VERSION");

const DNIE_4_CHIP: [u8; 2] = [0x4C, 0x34];
const CHIP_TYPE: std::ops::Range<usize> = 13..15;

/// Lo que hay en el lector de `opener`: el DNIe que se ve, ninguno, o el error de rdnie al abrirlo.
pub fn seen_dnie<O: CardOpener>(opener: &O) -> DnieInTheReader {
    match described(opener) {
        Ok(seen) => DnieInTheReader::Seen(seen),
        Err(rdnie::Error::NoReader | rdnie::Error::NoCard | rdnie::Error::NotDnie) => {
            DnieInTheReader::Absent
        }
        Err(error) => DnieInTheReader::Unopened(error.to_string()),
    }
}

fn described<O: CardOpener>(opener: &O) -> Result<SeenDnie, rdnie::Error> {
    let session = opener.open()?;
    let key_set = match session.key_generation() {
        rdnie::KeyGeneration::First => "primero",
        _ => "segundo",
    };
    let atr = session.into_transport().reset()?;
    let version = if atr.get(CHIP_TYPE) == Some(&DNIE_4_CHIP[..]) {
        "4.0"
    } else {
        "3.0"
    };
    Ok(SeenDnie {
        atr: atr.iter().map(|byte| format!("{byte:02X}")).collect(),
        version: version.to_owned(),
        key_set: key_set.to_owned(),
    })
}

fn in_the_store(store: &Store, label: &str, der: Vec<u8>) -> TokenCertificate {
    TokenCertificate::new(CertificateRef::new(store, TOKEN_LABEL, label, None), der)
}

fn is_no_readable_dnie(error: &rdnie::Error) -> bool {
    matches!(
        error,
        rdnie::Error::NoReader
            | rdnie::Error::NoCard
            | rdnie::Error::NotDnie
            | rdnie::Error::LegacyDnie
            | rdnie::Error::BurnedDnie
    )
}

fn token_error(error: &rdnie::Error) -> TokenError {
    let situation = match error {
        rdnie::Error::WrongPin { retries_left } => {
            return TokenError::new(Situation::IncorrectPin, error.to_string())
                .with_attempts_left(u32::from(*retries_left));
        }
        rdnie::Error::InvalidPinFormat => Situation::IncorrectPin,
        rdnie::Error::PinBlocked => Situation::PinLocked,
        rdnie::Error::CardRemoved
        | rdnie::Error::CardReset
        | rdnie::Error::NoCard
        | rdnie::Error::NoReader
        | rdnie::Error::Transport(_) => Situation::TokenAbsent,
        _ => Situation::Unknown,
    };
    TokenError::new(situation, error.to_string())
}

fn pin_of(secret: &ProtectedSecret) -> Result<Pin, TokenError> {
    secret
        .expose_secret()
        .map(|pin| Pin::new(pin.to_owned()))
        .map_err(|_| TokenError::new(Situation::IncorrectPin, "el PIN no es texto"))
}

fn hash_of(algorithm: SignatureAlgorithm) -> Result<HashAlgorithm, TokenError> {
    match algorithm {
        SignatureAlgorithm::Sha256Rsa => Ok(HashAlgorithm::Sha256),
        SignatureAlgorithm::Sha384Rsa => Ok(HashAlgorithm::Sha384),
        SignatureAlgorithm::Sha512Rsa => Ok(HashAlgorithm::Sha512),
        other => Err(TokenError::new(
            Situation::MechanismNotOffered,
            format!("el DNIe por rdnie no firma con {}", other.name()),
        )),
    }
}

fn unchanged_since_shown(left: u8, shown: Option<u8>) -> Result<(), TokenError> {
    match shown {
        Some(shown) if shown != left => Err(TokenError::new(
            Situation::Unknown,
            format!("los intentos han pasado de {shown} a {left} tras leerlos: el PIN no se envia"),
        )),
        _ => Ok(()),
    }
}

fn has_a_length_the_dnie_admits(secret: &ProtectedSecret) -> bool {
    secret
        .expose_secret()
        .is_ok_and(|pin| (8..=16).contains(&pin.encode_utf16().count()))
}

impl<O: CardOpener> Token for DnieToken<O> {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.list_since(store, self.readings())
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        let _turn = self.card_turn();
        self.read_every_certificate(store)
    }

    fn list_authenticated(
        &self,
        store: &Store,
        _pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        self.list(store)
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::TypedOnScreen)
    }

    fn pin_warning(&self, _reference: &CertificateRef) -> Result<PinWarning, TokenError> {
        let _turn = self.card_turn();
        let left = self.attempts_left()?;
        self.login().shown_attempts = Some(left);
        Ok(PinWarning::AttemptsLeft(u32::from(left)))
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        hash_of(algorithm).map(|_| ())
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        if let Some(cut) = self.login().cut.clone() {
            return Err(cut);
        }
        if !has_a_length_the_dnie_admits(secret) {
            return Err(token_error(&rdnie::Error::InvalidPinFormat));
        }
        let _turn = self.card_turn();
        let left = self.attempts_left()?;
        let shown = self.login().shown_attempts.take();
        unchanged_since_shown(left, shown)?;
        PinWarning::AttemptsLeft(u32::from(left)).unless_locked()?;
        self.verified_by_a_discarded_signature(reference, secret, left)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        let shown = {
            let mut login = self.login();
            if let Some(cut) = login.cut.clone() {
                return Err(cut);
            }
            login.shown_attempts.take()
        };
        let signed = {
            let _turn = self.card_turn();
            self.signed(reference, secret, algorithm, data, shown)
        };
        if let Err(failure) = &signed {
            let mut login = self.login();
            if login.depth > 0 {
                login.cut = Some(failure.clone());
            }
        }
        signed
    }

    fn hold_one_login(&self, _reference: &CertificateRef) {
        self.login().depth += 1;
    }

    fn release_the_login(&self, _reference: &CertificateRef) {
        let mut login = self.login();
        login.depth = login.depth.saturating_sub(1);
        if login.depth == 0 {
            login.cut = None;
        }
    }

    fn import_pkcs12(
        &self,
        _directory: &Path,
        _pkcs12: &[u8],
        _password: &str,
        _pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        Err(TokenError::new(
            Situation::Unknown,
            "un .p12 no se instala en el DNIe",
        ))
    }

    fn remove_certificate(
        &self,
        _directory: &Path,
        _reference: &CertificateRef,
        _pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        Err(TokenError::new(
            Situation::Unknown,
            "del DNIe no se quita ningun certificado",
        ))
    }
}

#[cfg(test)]
mod tests;
