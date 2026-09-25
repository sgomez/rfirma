//! El consentimiento que la compilación de conformidad da sola cuando no queda nada que decidir; no existe en un binario publicado.

use crate::identity::domain::certificate::ListedCertificate;
use crate::identity::domain::secret::StoreSecret;
use crate::site::domain::protocol::SiteVisibleSignature;
use crate::site::ports::{FilterEngine, PolicyEngine};

use std::ffi::OsStr;

use super::desk::{ErrandDesk, Neighbours};
use super::outcome::ErrandStep;
use super::state::LiveErrand;
use super::Consented;

/// Si el valor del interruptor lo enciende: solo `1`.
pub fn switched_on(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

/// Lo que se consiente sin nadie delante, con el asa del único certificado candidato.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unattended {
    /// `selectcert`: la sede recibe la identidad.
    Identify(String),
    /// Una firma que termina con su postfirma.
    Sign(String),
    /// Un lote, remoto o local, que la orden de firma cierra entero.
    SignTheBatch(String),
}

impl Unattended {
    /// El asa del certificado con el que se consiente.
    pub fn certificate(&self) -> &str {
        match self {
            Self::Identify(id) | Self::Sign(id) | Self::SignTheBatch(id) => id,
        }
    }
}

/// El consentimiento sin nadie delante, solo si el paso no deja nada que decidir; `secret_of` dice cómo pide el secreto el almacén del candidato.
pub fn nothing_to_decide(
    step: &ErrandStep,
    secret_of: impl FnOnce(&str) -> Option<StoreSecret>,
) -> Option<Unattended> {
    let asked = match step {
        ErrandStep::AskingForConsent { certificates, .. } => {
            return the_only(certificates).map(Unattended::Identify);
        }
        ErrandStep::AskingToSign(consent) => {
            let asks_something =
                matches!(consent.visible, SiteVisibleSignature::MarkedByThePerson(_))
                    || consent.unregistered_signatures
                    || consent.saving.is_some();
            if asks_something {
                return None;
            }
            Unattended::Sign(the_only(&consent.certificates)?)
        }
        ErrandStep::AskingToSignTheBatch(consent) => {
            Unattended::SignTheBatch(the_only(&consent.certificates)?)
        }
        ErrandStep::AskingToSignTheLocalBatch(consent) => {
            Unattended::SignTheBatch(the_only(&consent.certificates)?)
        }
        _ => return None,
    };
    (secret_of(asked.certificate()) == Some(StoreSecret::NotNeeded)).then_some(asked)
}

/// El mismo juicio, con el secreto que dicen los almacenes de la mesa.
pub fn nothing_to_decide_on<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    step: &ErrandStep,
) -> Option<Unattended> {
    nothing_to_decide(step, |handle| {
        let found = desk.neighbours.listed().ok()?;
        let candidate = desk.neighbours.usable(&found, handle).ok()?;
        desk.neighbours.secret_of(candidate).ok()
    })
}

/// El recorrido del clic con el único candidato: consentir, firmar sin secreto y, si es una firma, entregarla.
pub fn consent_unattended<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
    unattended: &Unattended,
    signed_without_a_secret: impl FnOnce() -> bool,
) {
    let Ok(Consented::SigningWith(_)) = super::consent(desk, unattended.certificate(), live) else {
        return;
    };
    if signed_without_a_secret() && matches!(unattended, Unattended::Sign(_)) {
        let _ = super::finish(desk, live);
    }
}

fn the_only(certificates: &[ListedCertificate]) -> Option<String> {
    match certificates {
        [only] => Some(only.id.clone()),
        _ => None,
    }
}
