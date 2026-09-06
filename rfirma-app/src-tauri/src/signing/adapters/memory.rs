//! Las dos memorias entre sesiones, la configuración viva y los dos interruptores (ADR-0010).

use std::sync::Mutex;

use crate::desktop::adapters::paths::Paths;
use crate::desktop::domain::version_check::VersionCheck;
use crate::lock;
use crate::signing::adapters::state::State;
use crate::signing::adapters::store::{JsonFile, Loaded};
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::domain::memory_error::MemoryError;

/// Las dos memorias y sus dos soportes (ADR-0010).
#[derive(Debug)]
pub struct Memory {
    configuration: JsonFile<Configuration>,
    state: JsonFile<State>,
    live: Mutex<Configuration>,
}

impl Memory {
    /// La memoria que vive en las rutas dadas.
    pub fn at(paths: &Paths) -> Self {
        let configuration = JsonFile::at(paths.config_file());
        let live = configuration
            .load()
            .map(Loaded::into_value)
            .unwrap_or_default();
        Self {
            configuration,
            state: JsonFile::at(paths.state_file()),
            live: Mutex::new(live),
        }
    }

    /// Una instantánea de la configuración viva.
    pub fn configuration(&self) -> Configuration {
        lock(&self.live).clone()
    }

    /// El soporte de la configuración.
    pub fn configuration_file(&self) -> &JsonFile<Configuration> {
        &self.configuration
    }

    /// El soporte del estado.
    pub fn state_file(&self) -> &JsonFile<State> {
        &self.state
    }

    /// La configuración guardada, o la de por omisión.
    pub fn stored_configuration(&self) -> Result<Loaded<Configuration>, MemoryError> {
        self.configuration.load()
    }

    /// El estado guardado, o el vacío.
    pub fn state(&self) -> Result<Loaded<State>, MemoryError> {
        self.state.load()
    }

    /// Guarda la configuración y borra el estado si la actividad queda desactivada (ADR-0010).
    pub fn remember_configuration(&self, configuration: &Configuration) -> Result<(), MemoryError> {
        self.configuration.save(configuration)?;
        if !configuration.remember_activity {
            self.erase_activity_but_keep_the_exempt()?;
        }
        *lock(&self.live) = configuration.clone();
        Ok(())
    }

    /// Guarda el estado según lo que permitan los dos interruptores (ADR-0010).
    pub fn remember_state(
        &self,
        configuration: &Configuration,
        state: &State,
    ) -> Result<(), MemoryError> {
        if !configuration.remember_activity {
            return self.erase_activity_but_keep_the_exempt();
        }
        if configuration.remember_visible_signature {
            return self.state.save(state);
        }
        let mut without_the_box = state.clone();
        without_the_box.visible_signature = None;
        without_the_box.recents.forget_placements();
        self.state.save(&without_the_box)
    }

    /// Olvida lo acumulado conservando los datos exentos (ADR-0010).
    pub fn forget_activity(&self) -> Result<(), MemoryError> {
        self.erase_activity_but_keep_the_exempt()
    }

    /// Guarda el registro de comprobación de versión sin depender de interruptores de actividad.
    pub fn remember_version_check(&self, check: VersionCheck) -> Result<(), MemoryError> {
        let mut state = self.state.load()?.into_value();
        state.version_check = Some(check);
        self.state.save(&state)
    }

    fn erase_activity_but_keep_the_exempt(&self) -> Result<(), MemoryError> {
        let mut kept = self
            .state
            .load()
            .map(Loaded::into_value)
            .unwrap_or_default();
        kept.forget_everything();
        self.state.erase()?;
        if kept.is_empty() {
            return Ok(());
        }
        self.state.save(&kept)
    }
}

#[cfg(test)]
mod tests;
