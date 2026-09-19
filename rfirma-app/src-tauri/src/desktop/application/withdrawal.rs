//! Qué retirada hay que reintentar y cómo fusionar el resultado con lo que ya constaba: la
//! decisión pura detrás de que `Reintentar` solo toque lo que falló.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::desktop::domain::status::StoreBrand;
use crate::desktop::domain::withdrawal::{StoreWithdrawal, Withdrawal, WithdrawalReport};

/// Si el manejador hay que reintentarlo: no, si `previous` ya lo dejó sin fallo.
pub fn handler_needs_retry(previous: Option<&WithdrawalReport>) -> bool {
    previous.is_none_or(|report| report.handler.failed())
}

/// Los perfiles de `profiles` que hay que volver a tocar: todos sin intento previo, o solo los
/// que en `previous` fallaron.
pub fn profiles_to_retry(
    profiles: &[(PathBuf, StoreBrand)],
    previous: Option<&WithdrawalReport>,
) -> Vec<PathBuf> {
    profiles
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            previous.is_none_or(|report| {
                report
                    .stores
                    .get(*index)
                    .is_none_or(|store| store.outcome.failed())
            })
        })
        .map(|(_, (profile, _))| profile.clone())
        .collect()
}

/// Combina lo que se acaba de retirar (`retried`) con lo que ya constaba en `previous` para los
/// almacenes que no se han vuelto a tocar.
pub fn merged_report(
    handler: Withdrawal,
    profiles: &[(PathBuf, StoreBrand)],
    mut retried: HashMap<PathBuf, Withdrawal>,
    previous: Option<&WithdrawalReport>,
) -> WithdrawalReport {
    let stores = profiles
        .iter()
        .enumerate()
        .map(|(index, (profile, brand))| {
            let outcome = retried.remove(profile).unwrap_or_else(|| {
                previous
                    .and_then(|report| report.stores.get(index))
                    .map(|store| store.outcome.clone())
                    .unwrap_or(Withdrawal::WasNotThere)
            });
            StoreWithdrawal {
                brand: *brand,
                outcome,
            }
        })
        .collect();
    WithdrawalReport { handler, stores }
}

#[cfg(test)]
mod tests;
