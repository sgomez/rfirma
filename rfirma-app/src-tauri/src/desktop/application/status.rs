//! Evaluación y medición de las señales del panel de estado.

use std::time::SystemTime;

use crate::desktop::application::version::{
    ask_and_remember, remembered_answer, ReleaseFeed, Version,
};
use crate::desktop::domain::channel::Channel;
use crate::desktop::domain::destination::{CERTIFICATE_ISSUANCE, RELEASES, REPOSITORY, WINDOWS};
use crate::desktop::domain::handlers::{UrlHandlers, FLATPAK_DESKTOP_FILE};
use crate::desktop::domain::status::{
    ActionKind, Signal, SignalDetail, SignalRow, SiteSignatureCandidate, StatusAction,
    StoreCertificates, StoreDetail, Verdict,
};
use crate::desktop::ports::VersionMemory;

/// Nombre visible de rFirma como candidata a firmar en sedes.
const OUR_NAME: &str = "rFirma";

/// Destino de actualización que corresponde al canal de distribución.
pub fn update_destination_for(channel: Channel) -> &'static str {
    match channel {
        Channel::Flatpak => REPOSITORY,
        Channel::Native => RELEASES,
        Channel::Windows => WINDOWS,
    }
}

/// Evalúa el estado de la señal de versión a partir de las lecturas.
pub fn evaluate_version_signal(
    running: Version,
    announced: Option<Version>,
    channel: Channel,
) -> SignalRow {
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
            candidates: None,
            restart_firefox_notice: false,
        }
    } else {
        SignalRow {
            signal: Signal::Version,
            value: running.to_string(),
            verdict: Verdict::Correct,
            action: None,
            detail: None,
            candidates: None,
            restart_firefox_notice: false,
        }
    }
}

/// Mide la señal de versión preguntando a la red, cayendo a la última conocida sin respuesta.
pub fn measure_version_signal(
    running: Version,
    memory: &dyn VersionMemory,
    feed: ReleaseFeed<'_>,
    channel: Channel,
    now: SystemTime,
) -> SignalRow {
    let announced =
        ask_and_remember(memory, feed, channel, now).or_else(|| remembered_answer(memory));
    evaluate_version_signal(running, announced, channel)
}

/// Fila de la señal de versión mientras se mide fuera de la lectura de estado (ADR-0015).
pub fn checking_version_signal() -> SignalRow {
    SignalRow {
        signal: Signal::Version,
        value: String::new(),
        verdict: Verdict::Checking,
        action: None,
        detail: None,
        candidates: None,
        restart_firefox_notice: false,
    }
}

/// Candidatas a firmar en sedes: solo hay dónde elegir con dos o más registradas.
fn site_signature_candidates(handlers: &UrlHandlers) -> Option<Vec<SiteSignatureCandidate>> {
    if handlers.handlers.len() < 2 {
        return None;
    }
    Some(
        handlers
            .handlers
            .iter()
            .map(|handler| SiteSignatureCandidate {
                id: handler.id.clone(),
                name: handler.name.clone(),
                selected: handlers.current.as_deref() == Some(handler.id.as_str()),
            })
            .collect(),
    )
}

/// `Usar rFirma`: solo donde de verdad hay algo que arreglar, que es cuando rFirma no es quien
/// firma en sedes hoy.
fn site_signature_action(handlers: &UrlHandlers) -> Option<StatusAction> {
    let is_ours = handlers.current.as_deref() == Some(handlers.ours.as_str());
    (!is_ours).then(|| StatusAction {
        kind: ActionKind::Choice,
        target: handlers.ours.clone(),
    })
}

/// Cómo diagnosticar quién abre las sedes: solo en el flatpak, donde lo dicen las órdenes `xdg-mime` de fuera del sandbox.
fn site_signature_diagnosis(channel: Channel) -> Option<SignalDetail> {
    (channel == Channel::Flatpak).then(|| SignalDetail::HandlerDiagnosis {
        desktop_file: FLATPAK_DESKTOP_FILE.to_owned(),
    })
}

/// Evalúa el estado de la señal de qué programa abre las sedes, con las candidatas instaladas.
pub fn evaluate_site_signature_signal(handlers: UrlHandlers, channel: Channel) -> SignalRow {
    if !handlers.available {
        return SignalRow {
            signal: Signal::SiteSignature,
            value: String::new(),
            verdict: Verdict::NotApplicable,
            action: None,
            detail: site_signature_diagnosis(channel),
            candidates: None,
            restart_firefox_notice: false,
        };
    }

    let candidates = site_signature_candidates(&handlers);
    let action = site_signature_action(&handlers);

    let Some(current) = &handlers.current else {
        return SignalRow {
            signal: Signal::SiteSignature,
            value: String::new(),
            verdict: Verdict::Attention,
            action,
            detail: None,
            candidates,
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
            candidates,
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
        action,
        detail: None,
        candidates,
        restart_firefox_notice: false,
    }
}

/// Evalúa la señal de los certificados propios: cuántos hay en total y en qué sitio está cada cuántos.
pub fn evaluate_user_certificates_signal(stores: Vec<StoreCertificates>) -> SignalRow {
    let found: usize = stores.iter().map(|store| store.certificates).sum();
    let action = (found == 0).then(|| StatusAction {
        kind: ActionKind::Link,
        target: CERTIFICATE_ISSUANCE.to_string(),
    });

    SignalRow {
        signal: Signal::UserCertificates,
        value: found.to_string(),
        verdict: if found == 0 {
            Verdict::Attention
        } else {
            Verdict::Correct
        },
        action,
        detail: (found > 0).then_some(SignalDetail::Certificates { stores }),
        candidates: None,
        restart_firefox_notice: false,
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
        candidates: None,
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
        detail: Some(SignalDetail::Trust { stores: detail }),
        candidates: None,
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
