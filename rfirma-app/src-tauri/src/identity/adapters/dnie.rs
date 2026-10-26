//! `DnieToken`, el DNIe leído con rdnie detrás del puerto `Token`: lista sin PIN, da los intentos restantes y firma el resumen sobre una sesión que dura lo que la tarjeta sigue en el lector (ADR-0040, ADR-0047).

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rdnie::{
    Atr, DigestAlgorithm, PcscTransport, Pin, Session, Transport, INTERMEDIATE_CA_LABEL,
    SIGNATURE_LABEL,
};

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::ecdsa;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::{Store, DNIE_STORE_PREFIX};
use crate::identity::ports::Token;

const TOKEN_LABEL: &str = "DNIe";

/// El almacén del DNIe: de clase tarjeta, sin módulo PKCS#11 detrás.
pub fn dnie_store() -> Store {
    Store::module(format!("{DNIE_STORE_PREFIX}pcsc"))
}

/// Quien abre la tarjeta al empezar cada inserción: el lector en producción, el emulador en las pruebas.
pub trait CardOpener: Send + Sync {
    /// Por dónde viajan las órdenes a la tarjeta.
    type Card: Transport + Send;

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

    /// Una sesión nueva con el DNIe y el lector donde se ha abierto, si se sabe.
    fn open_in_its_reader(&self) -> Result<(Session<Self::Card>, Option<String>), rdnie::Error> {
        Ok((self.open()?, None))
    }

    /// Un número que cambia cada vez que se mete o se saca una tarjeta de `reader`, o de cualquiera si no se sabe cuál; ninguno si no lo sabe.
    fn insertion(&self, _reader: Option<&str>) -> Option<u64> {
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

    fn open_in_its_reader(&self) -> Result<(Session<PcscTransport>, Option<String>), rdnie::Error> {
        let card = PcscTransport::connect_first_dnie()?;
        let reader = card.reader();
        Ok((Session::open(card)?, Some(reader)))
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

    fn insertion(&self, reader: Option<&str>) -> Option<u64> {
        super::pcsc::card_events(reader)
    }
}

/// Si el ATR es de un DNIe que abre rdnie: un 3.0 o 4.0 sin quemar; el 1.0 y el 2.0 siguen por PKCS#11.
fn rdnie_opens(atr: &Atr) -> bool {
    atr.check().is_ok()
}

fn take_the_card_turn() -> MutexGuard<'static, ()> {
    CARD_TURN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// El adaptador de [`Token`] para el almacén del DNIe.
pub struct DnieToken<O: CardOpener> {
    opener: O,
    inserted: Mutex<Option<Inserted<O::Card>>>,
    login: Mutex<Login>,
}

/// La sesión de la tarjeta que sigue en el lector, con el lector y la inserción en que se abrió.
struct Inserted<T: Transport> {
    session: Session<T>,
    reader: Option<String>,
    insertion: Option<u64>,
}

/// Una sola tarjeta física: el turno es de todo el proceso, también para el informe de depuración.
static CARD_TURN: Mutex<()> = Mutex::new(());

/// Lo que el adaptador recuerda del ciclo de firma entre llamadas.
#[derive(Debug, Default)]
struct Login {
    shown_attempts: Option<u8>,
    depth: usize,
    cut: Option<TokenError>,
}

impl<O: CardOpener> DnieToken<O> {
    /// El DNIe que abre `opener`.
    pub fn new(opener: O) -> Self {
        Self {
            opener,
            inserted: Mutex::new(None),
            login: Mutex::new(Login::default()),
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

    fn login(&self) -> MutexGuard<'_, Login> {
        self.login
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn inserted(&self) -> MutexGuard<'_, Option<Inserted<O::Card>>> {
        self.inserted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Un listado sobre la sesión de esta inserción: tras un reset ajeno, o en la sesión de una tarjeta ya sacada, se repite una vez.
    fn listing<R>(
        &self,
        mut operation: impl FnMut(&mut Session<O::Card>) -> Result<R, rdnie::Error>,
    ) -> Result<Option<R>, TokenError> {
        let _turn = take_the_card_turn();
        let mut inserted = self.inserted();
        let mut retried = false;
        loop {
            let reused = inserted.is_some();
            let failure = match opened(&mut inserted, &self.opener) {
                Ok(session) => match operation(session) {
                    Ok(done) => return Ok(Some(done)),
                    Err(failure) => failure,
                },
                Err(error) if is_no_readable_dnie(&error) => return Ok(None),
                Err(error) => error,
            };
            let spent = forgotten_if_spent(&mut inserted, &failure);
            let worth_another_go = matches!(failure, rdnie::Error::CardReset) || (spent && reused);
            if retried || !worth_another_go {
                return Err(token_error(&failure));
            }
            retried = true;
        }
    }

    /// Un paso de la firma sobre la sesión de esta inserción, que nunca se repite (ADR-0047).
    fn signing<R>(
        &self,
        operation: impl FnOnce(&mut Session<O::Card>) -> Result<R, rdnie::Error>,
    ) -> Result<R, TokenError> {
        let _turn = take_the_card_turn();
        one_step(&mut self.inserted(), &self.opener, operation)
    }

    fn attempts_left(&self) -> Result<u8, TokenError> {
        self.signing(retries_left_or_blocked)
    }

    /// Un paso con el PIN; tras mostrar `shown`, en la sesión de la tarjeta que haya vuelto al lector.
    fn with_the_pin<R>(
        &self,
        shown: Option<u8>,
        operation: impl FnOnce(&mut Session<O::Card>) -> Result<R, rdnie::Error>,
    ) -> Result<R, TokenError> {
        let _turn = take_the_card_turn();
        let mut inserted = self.inserted();
        if shown.is_some() {
            forgotten_if_reinserted(&mut inserted, &self.opener);
        }
        one_step(&mut inserted, &self.opener, operation)
    }

    fn verified(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        shown: Option<u8>,
    ) -> Result<(), TokenError> {
        let pin = typed_pin(secret)?;
        self.with_the_pin(shown, |session| {
            if session.private_key(reference.label())?.is_none() {
                return Ok(Err(key_not_found(reference)));
            }
            under_the_retries_lock(session, shown, |session| {
                session.verify_pin(Pin::new(pin.to_owned()))
            })
        })?
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
        let pin = typed_pin(secret)?;
        self.with_the_pin(shown, |session| {
            let Some(key) = session.private_key(reference.label())? else {
                return Ok(Err(key_not_found(reference)));
            };
            under_the_retries_lock(session, shown, |session| {
                session.sign(&digest, hash, &key, Pin::new(pin.to_owned()))
            })
        })?
    }

    fn signing_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self
            .listing(|session| session.certificate(SIGNATURE_LABEL))?
            .flatten()
            .map(|der| in_the_store(store, SIGNATURE_LABEL, der))
            .into_iter()
            .collect())
    }

    fn holders(&self) -> Result<usize, TokenError> {
        Ok(self
            .listing(|session| session.labels().map(|found| found.len()))?
            .unwrap_or(0))
    }

    fn every_certificate_of(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self
            .listing(every_der)?
            .unwrap_or_default()
            .into_iter()
            .map(|(label, der)| in_the_store(store, &label, der))
            .collect())
    }
}

/// La sesión de esta inserción, abierta si aún no la hay; la de una tarjeta que el lector ya ha visto salir falla como `CardRemoved`.
fn opened<'a, O: CardOpener>(
    inserted: &'a mut Option<Inserted<O::Card>>,
    opener: &O,
) -> Result<&'a mut Session<O::Card>, rdnie::Error> {
    let held = match inserted.take() {
        Some(held) if held.insertion == opener.insertion(held.reader.as_deref()) => held,
        Some(_) => return Err(rdnie::Error::CardRemoved),
        None => {
            let (session, reader) = opener.open_in_its_reader()?;
            let insertion = opener.insertion(reader.as_deref());
            Inserted {
                session,
                reader,
                insertion,
            }
        }
    };
    Ok(&mut inserted.insert(held).session)
}

fn one_step<O: CardOpener, R>(
    inserted: &mut Option<Inserted<O::Card>>,
    opener: &O,
    operation: impl FnOnce(&mut Session<O::Card>) -> Result<R, rdnie::Error>,
) -> Result<R, TokenError> {
    let failure = match opened(inserted, opener) {
        Ok(session) => match operation(session) {
            Ok(done) => return Ok(done),
            Err(failure) => failure,
        },
        Err(error) => error,
    };
    forgotten_if_spent(inserted, &failure);
    Err(token_error(&failure))
}

/// Olvida la sesión de una tarjeta que el lector ha visto salir, para abrir la que haya ahora.
fn forgotten_if_reinserted<O: CardOpener>(inserted: &mut Option<Inserted<O::Card>>, opener: &O) {
    if inserted
        .as_ref()
        .is_some_and(|held| held.insertion != opener.insertion(held.reader.as_deref()))
    {
        *inserted = None;
    }
}

/// Olvida la sesión si `failure` ha dejado su transporte inservible, y dice si lo ha hecho.
fn forgotten_if_spent<T: Transport>(
    inserted: &mut Option<Inserted<T>>,
    failure: &rdnie::Error,
) -> bool {
    let spent = leaves_the_transport_spent(failure);
    if spent {
        *inserted = None;
    }
    spent
}

fn key_not_found(reference: &CertificateRef) -> TokenError {
    TokenError::new(
        Situation::CertificateNotFound,
        format!("el DNIe no tiene la clave de {}", reference.label()),
    )
}

/// Tras estos errores el transporte ya no sirve y la sesión de la inserción se descarta.
fn leaves_the_transport_spent(error: &rdnie::Error) -> bool {
    matches!(
        error,
        rdnie::Error::CardRemoved
            | rdnie::Error::NoCard
            | rdnie::Error::NoReader
            | rdnie::Error::Transport(_)
    )
}

fn every_der<T: Transport>(
    session: &mut Session<T>,
) -> Result<Vec<(String, Vec<u8>)>, rdnie::Error> {
    let mut labels = session.labels()?;
    labels.push(INTERMEDIATE_CA_LABEL.to_owned());
    let mut found = Vec::new();
    for label in labels {
        if let Some(der) = session.certificate(&label)? {
            found.push((label, der));
        }
    }
    Ok(found)
}

/// La versión de rdnie que fija `Cargo.lock`.
pub const RDNIE_VERSION: &str = env!("RFIRMA_RDNIE_VERSION");

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

fn hash_of(algorithm: SignatureAlgorithm) -> Result<DigestAlgorithm, TokenError> {
    match algorithm {
        SignatureAlgorithm::Sha256Rsa => Ok(DigestAlgorithm::Sha256),
        SignatureAlgorithm::Sha384Rsa => Ok(DigestAlgorithm::Sha384),
        SignatureAlgorithm::Sha512Rsa => Ok(DigestAlgorithm::Sha512),
        other => Err(TokenError::new(
            Situation::MechanismNotOffered,
            format!("el DNIe por rdnie no firma con {}", other.name()),
        )),
    }
}

impl<O: CardOpener> Token for DnieToken<O> {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.signing_certificate(store)
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.every_certificate_of(store)
    }

    fn holders_without_reading(&self, _store: &Store) -> Option<Result<usize, TokenError>> {
        Some(self.holders())
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
        let shown = self.login().shown_attempts.take();
        self.verified(reference, secret, shown)
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
        let signed = self.signed(reference, secret, algorithm, data, shown);
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

mod retries;
mod seen;
use retries::{
    has_a_length_the_dnie_admits, retries_left_or_blocked, typed_pin, under_the_retries_lock,
};
pub use seen::seen_dnie;

#[cfg(test)]
mod tests;
