//! Evaluación y medición de las señales del panel de estado.

use std::time::SystemTime;

use crate::desktop::application::version::{ask_and_remember, fresh_answer, ReleaseFeed, Version};
use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::destination::{RELEASES, REPOSITORY};
use crate::desktop::domain::status::{ActionKind, Signal, SignalRow, StatusAction, Verdict};
use crate::desktop::ports::VersionMemory;

/// Destino de actualización que corresponde al canal de distribución.
pub fn update_destination_for(channel: Channel) -> &'static str {
    match channel {
        Channel::Flatpak => REPOSITORY,
        Channel::Native => RELEASES,
    }
}

/// Evalúa el estado de la señal de versión a partir de las lecturas.
pub fn evaluate_version_signal(
    running: Version,
    announced: Option<Version>,
    checking: bool,
    channel: Channel,
) -> SignalRow {
    if checking {
        return SignalRow {
            signal: Signal::Version,
            value: running.to_string(),
            verdict: Verdict::Checking,
            action: None,
        };
    }

    if let Some(latest) = announced.filter(|&v| v > running) {
        SignalRow {
            signal: Signal::Version,
            value: format!("{running} → {latest}"),
            verdict: Verdict::Attention,
            action: Some(StatusAction {
                kind: ActionKind::Link,
                target: update_destination_for(channel).to_string(),
            }),
        }
    } else {
        SignalRow {
            signal: Signal::Version,
            value: running.to_string(),
            verdict: Verdict::Correct,
            action: None,
        }
    }
}

/// Mide la señal de versión consultando la memoria o la red según proceda.
pub fn check_version_signal(
    running: Version,
    memory: &dyn VersionMemory,
    feed: ReleaseFeed<'_>,
    channel: Channel,
    recheck: bool,
    now: SystemTime,
) -> SignalRow {
    if recheck {
        let announced = ask_and_remember(memory, feed, now);
        evaluate_version_signal(running, announced, false, channel)
    } else {
        match fresh_answer(memory, now) {
            Some(cached) => evaluate_version_signal(running, Some(cached), false, channel),
            None => evaluate_version_signal(running, None, true, channel),
        }
    }
}

#[cfg(test)]
mod tests;
