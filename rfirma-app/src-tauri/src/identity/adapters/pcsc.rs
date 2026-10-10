//! El vigilante PC/SC de Linux y Windows: espera a `SCardGetStatusChange` sobre todos los lectores y las notificaciones PnP (ADR-0048).

use std::ffi::CString;
use std::time::Duration;

use pcsc::{Context, ReaderState, Scope, State};

use crate::identity::domain::readers::Reader;
use crate::identity::ports::ReaderWatch;

const RETRY_WITHOUT_PCSC: Duration = Duration::from_secs(5);

type ReaderStates = Vec<(CString, State)>;

/// De dónde salen los estados de los lectores: espera a que cambien respecto a los conocidos.
pub trait StatusSource: Send {
    /// Los estados tras el cambio; `None`, si PC/SC no responde.
    fn wait_for_a_change(&mut self, known: &[(CString, State)]) -> Option<ReaderStates>;
}

/// Vigila los lectores por PC/SC; sin `pcscd`, cuenta cero lectores y vuelve a intentarlo.
pub type PcscReaderWatch = StatusWatch<PcscSource>;

/// Convierte los cambios de estado de una fuente en la vista de lectores del puerto.
pub struct StatusWatch<S> {
    source: S,
    retry: Duration,
    known: ReaderStates,
    last: Option<Vec<Reader>>,
}

impl<S: StatusSource + Default> Default for StatusWatch<S> {
    fn default() -> Self {
        Self::new(S::default(), RETRY_WITHOUT_PCSC)
    }
}

impl<S: StatusSource> StatusWatch<S> {
    fn new(source: S, retry: Duration) -> Self {
        Self {
            source,
            retry,
            known: Vec::new(),
            last: None,
        }
    }

    fn look(&mut self) -> (Vec<Reader>, bool) {
        let Some(states) = self.source.wait_for_a_change(&self.known) else {
            self.known.clear();
            return (Vec::new(), false);
        };
        self.known = states;
        (readers_in(&self.known), true)
    }
}

impl<S: StatusSource> ReaderWatch for StatusWatch<S> {
    fn next_change(&mut self) -> Option<Vec<Reader>> {
        loop {
            let (readers, available) = self.look();
            if self.last.as_ref() != Some(&readers) {
                self.last = Some(readers.clone());
                return Some(readers);
            }
            if !available {
                std::thread::sleep(self.retry);
            }
        }
    }
}

/// La fuente real: `libpcsclite` en Linux, `winscard.dll` en Windows.
#[derive(Default)]
pub struct PcscSource {
    context: Option<Context>,
}

impl StatusSource for PcscSource {
    fn wait_for_a_change(&mut self, known: &[(CString, State)]) -> Option<ReaderStates> {
        let context = self.context()?;
        let names = reader_names(context)?;
        let observed = status_change(context, &names, known);
        if observed.is_none() {
            self.context = None;
        }
        observed
    }
}

impl PcscSource {
    fn context(&mut self) -> Option<&Context> {
        if self.context.is_none() {
            self.context = Context::establish(Scope::System).ok();
        }
        self.context.as_ref()
    }
}

fn reader_names(context: &Context) -> Option<Vec<CString>> {
    match context.list_readers_owned() {
        Ok(names) => Some(names),
        Err(pcsc::Error::NoReadersAvailable) => Some(Vec::new()),
        Err(_) => None,
    }
}

fn status_change(
    context: &Context,
    names: &[CString],
    known: &[(CString, State)],
) -> Option<ReaderStates> {
    let mut states: Vec<ReaderState> = std::iter::once(pcsc::PNP_NOTIFICATION().to_owned())
        .chain(names.iter().cloned())
        .map(|name| {
            let state = state_known_for(known, &name);
            ReaderState::new(name, state)
        })
        .collect();
    context.get_status_change(None, &mut states).ok()?;
    Some(
        states
            .iter()
            .map(|state| (state.name().to_owned(), state.event_state()))
            .collect(),
    )
}

fn state_known_for(known: &[(CString, State)], name: &CString) -> State {
    known
        .iter()
        .find(|(known_name, _)| known_name == name)
        .map_or(State::UNAWARE, |(_, state)| *state)
}

fn readers_in(states: &[(CString, State)]) -> Vec<Reader> {
    states
        .iter()
        .filter(|(name, state)| {
            name.as_c_str() != pcsc::PNP_NOTIFICATION()
                && !state.intersects(State::IGNORE | State::UNKNOWN)
        })
        .map(|(name, state)| Reader {
            name: name.to_string_lossy().into_owned(),
            has_a_card: state.contains(State::PRESENT),
        })
        .collect()
}

/// Una consulta puntual a PC/SC; `None`, si no responde.
pub fn survey_readers() -> Option<Vec<Reader>> {
    let context = Context::establish(Scope::System).ok()?;
    let names = reader_names(&context)?;
    let mut states: Vec<ReaderState> = names
        .into_iter()
        .map(|name| ReaderState::new(name, State::UNAWARE))
        .collect();
    if !states.is_empty() {
        context
            .get_status_change(Duration::ZERO, &mut states)
            .ok()?;
    }
    let observed: ReaderStates = states
        .iter()
        .map(|state| (state.name().to_owned(), state.event_state()))
        .collect();
    Some(readers_in(&observed))
}

#[cfg(test)]
mod tests;
