//! Evaluación y medición de las señales del panel de estado.

use std::time::SystemTime;

use crate::desktop::application::version::{ask_and_remember, fresh_answer, ReleaseFeed, Version};
use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::destination::{CERTIFICATE_ISSUANCE, RELEASES, REPOSITORY};
use crate::desktop::domain::handlers::UrlHandlers;
use crate::desktop::domain::status::{
    ActionKind, Signal, SignalRow, StatusAction, StoreDetail, Verdict,
};
use crate::desktop::ports::VersionMemory;

/// Nombre visible de rFirma como candidata a firmar en sedes.
const OUR_NAME: &str = "rFirma";

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
            detail: None,
            restart_firefox_notice: false,
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
            detail: None,
            restart_firefox_notice: false,
        }
    } else {
        SignalRow {
            signal: Signal::Version,
            value: running.to_string(),
            verdict: Verdict::Correct,
            action: None,
            detail: None,
            restart_firefox_notice: false,
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

/// Evalúa el estado de la señal de qué programa abre las sedes, con las candidatas instaladas.
pub fn evaluate_site_signature_signal(handlers: UrlHandlers) -> SignalRow {
    if !handlers.available {
        return SignalRow {
            signal: Signal::SiteSignature,
            value: String::new(),
            verdict: Verdict::NotApplicable,
            action: None,
            detail: None,
            restart_firefox_notice: false,
        };
    }

    let Some(current) = &handlers.current else {
        return SignalRow {
            signal: Signal::SiteSignature,
            value: String::new(),
            verdict: Verdict::Attention,
            action: None,
            detail: None,
            restart_firefox_notice: false,
        };
    };

    if *current == handlers.ours {
        return SignalRow {
            signal: Signal::SiteSignature,
            value: OUR_NAME.to_string(),
            verdict: Verdict::Correct,
            action: None,
            detail: None,
            restart_firefox_notice: false,
        };
    }

    let name = handlers
        .handlers
        .iter()
        .find(|handler| handler.id == *current)
        .map(|handler| handler.name.clone())
        .unwrap_or_else(|| current.clone());

    SignalRow {
        signal: Signal::SiteSignature,
        value: name,
        verdict: Verdict::Attention,
        action: None,
        detail: None,
        restart_firefox_notice: false,
    }
}

/// Evalúa el estado de la señal de certificados propios a partir de los almacenes con certificados.
pub fn evaluate_user_certificates_signal(stores_with_certificates: usize) -> SignalRow {
    if stores_with_certificates == 0 {
        SignalRow {
            signal: Signal::UserCertificates,
            value: "0".to_string(),
            verdict: Verdict::Attention,
            action: Some(StatusAction {
                kind: ActionKind::Link,
                target: CERTIFICATE_ISSUANCE.to_string(),
            }),
            detail: None,
            restart_firefox_notice: false,
        }
    } else {
        SignalRow {
            signal: Signal::UserCertificates,
            value: stores_with_certificates.to_string(),
            verdict: Verdict::Correct,
            action: None,
            detail: None,
            restart_firefox_notice: false,
        }
    }
}

/// Fila de la señal del certificado de rFirma mientras se mide fuera del hilo de la interfaz.
pub fn checking_local_ca_certificate_signal() -> SignalRow {
    SignalRow {
        signal: Signal::LocalCaCertificate,
        value: String::new(),
        verdict: Verdict::Checking,
        action: None,
        detail: None,
        restart_firefox_notice: false,
    }
}

/// Identificador de la reparación de la señal del certificado de rFirma, con `Instalar`.
pub const INSTALL_LOCAL_CA_CERTIFICATE: &str = "installLocalCaCertificate";

/// Evalúa el estado de la señal del certificado de rFirma a partir del detalle por almacén y de
/// si Firefox estaba vivo cuando se instaló.
pub fn evaluate_local_ca_certificate_signal(
    detail: Vec<StoreDetail>,
    restart_firefox_notice: bool,
) -> SignalRow {
    let total = detail.len();
    let trusted = detail.iter().filter(|store| store.trusted).count();
    let verdict = if trusted == total && total > 0 {
        Verdict::Correct
    } else if trusted == 0 {
        Verdict::Incorrect
    } else {
        Verdict::Attention
    };
    let action = (verdict != Verdict::Correct).then(|| StatusAction {
        kind: ActionKind::Repair,
        target: INSTALL_LOCAL_CA_CERTIFICATE.to_string(),
    });

    SignalRow {
        signal: Signal::LocalCaCertificate,
        value: format!("{trusted}/{total}"),
        verdict,
        action,
        detail: Some(detail),
        restart_firefox_notice,
    }
}

/// Si conviene avisar de reiniciar Firefox: alguno de sus perfiles pasó de no confiar a confiar
/// en la CA local mientras Firefox seguía abierto.
pub fn firefox_restart_notice(
    firefox_was_running: bool,
    trusted_before: &[bool],
    trusted_after: &[bool],
) -> bool {
    firefox_was_running
        && trusted_after
            .iter()
            .zip(trusted_before)
            .any(|(after, before)| *after && !*before)
}

#[cfg(test)]
mod tests;
