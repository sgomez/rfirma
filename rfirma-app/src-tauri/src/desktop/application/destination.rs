//! Caso de uso: apertura de un destino externo en el navegador (ID-369).

use crate::desktop::domain::destination::resolve_destination;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DestinationError {
    UnknownTarget(String),
    Failed(String),
}

impl std::fmt::Display for DestinationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTarget(target) => write!(f, "destino desconocido: {target}"),
            Self::Failed(detail) => write!(f, "fallo al abrir destino: {detail}"),
        }
    }
}

impl std::error::Error for DestinationError {}

pub fn open_destination<O>(target: &str, opener: O) -> Result<(), DestinationError>
where
    O: FnOnce(&str) -> Result<(), String>,
{
    let url = resolve_destination(target)
        .ok_or_else(|| DestinationError::UnknownTarget(target.to_string()))?;
    opener(url).map_err(DestinationError::Failed)
}

#[cfg(test)]
mod tests;
