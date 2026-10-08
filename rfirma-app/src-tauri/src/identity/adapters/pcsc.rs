//! El vigilante PC/SC de Linux: espera a `SCardGetStatusChange` sobre todos los lectores y las notificaciones PnP (ADR-0048).

use std::ffi::CString;
use std::time::Duration;

use pcsc::{Context, ReaderState, Scope, State};

use crate::identity::domain::readers::Reader;
use crate::identity::ports::ReaderWatch;

const RETRY_WITHOUT_PCSC: Duration = Duration::from_secs(5);

/// Vigila los lectores por PC/SC; sin `pcscd`, cuenta cero lectores y vuelve a intentarlo.
#[derive(Default)]
pub struct PcscReaderWatch {
    context: Option<Context>,
    known: Vec<(CString, State)>,
    last: Option<Vec<Reader>>,
}

impl ReaderWatch for PcscReaderWatch {
    fn next_change(&mut self) -> Option<Vec<Reader>> {
        loop {
            let readers = match self.observe() {
                Some(readers) => readers,
                None if self.last.as_deref() == Some(&[]) => {
                    std::thread::sleep(RETRY_WITHOUT_PCSC);
                    continue;
                }
                None => Vec::new(),
            };
            if self.last.as_ref() != Some(&readers) {
                self.last = Some(readers.clone());
                return Some(readers);
            }
        }
    }
}

impl PcscReaderWatch {
    fn observe(&mut self) -> Option<Vec<Reader>> {
        let observed = self.wait_for_a_change();
        if observed.is_none() {
            self.context = None;
            self.known.clear();
        }
        observed
    }

    fn wait_for_a_change(&mut self) -> Option<Vec<Reader>> {
        if self.context.is_none() {
            self.context = Some(Context::establish(Scope::System).ok()?);
        }
        let context = self.context.as_ref()?;
        let names = match context.list_readers_owned() {
            Ok(names) => names,
            Err(pcsc::Error::NoReadersAvailable) => Vec::new(),
            Err(_) => return None,
        };
        let mut states = self.states_for(&names);
        context.get_status_change(None, &mut states).ok()?;
        self.known = states
            .iter()
            .map(|state| (state.name().to_owned(), state.event_state()))
            .collect();
        Some(readers_in(&self.known))
    }

    fn states_for(&self, names: &[CString]) -> Vec<ReaderState> {
        std::iter::once(pcsc::PNP_NOTIFICATION().to_owned())
            .chain(names.iter().cloned())
            .map(|name| {
                let known = self
                    .known
                    .iter()
                    .find(|(known_name, _)| *known_name == name)
                    .map_or(State::UNAWARE, |(_, state)| *state);
                ReaderState::new(name, known)
            })
            .collect()
    }
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

#[cfg(test)]
mod tests;
