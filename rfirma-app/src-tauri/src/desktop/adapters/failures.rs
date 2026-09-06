//! La única traducción de las situaciones del escritorio a lo que ve la ventana (ADR-0009).

use crate::commands::Failure;
use crate::desktop::domain::error::{DesktopError, Situation};

/// Clave del catálogo de cada situación del escritorio; ninguna llega a la sede.
pub fn situation_name(situation: Situation) -> &'static str {
    match situation {
        Situation::NotAvailableInsideTheSandbox => "handlerNotAvailable",
        Situation::TheListIsNotReadable => "handlerListUnreadable",
        Situation::TheListIsNotWritable => "handlerListUnwritable",
    }
}

impl From<DesktopError> for Failure {
    fn from(error: DesktopError) -> Self {
        Self::new(situation_name(error.situation()), error.detail().to_owned())
    }
}

#[cfg(test)]
mod tests;
