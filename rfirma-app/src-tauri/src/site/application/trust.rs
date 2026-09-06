//! Gestión del registro y renovación de la CA local en almacenes NSS (ADR-0005).

use std::path::{Path, PathBuf};

use crate::site::domain::local_ca::{LocalCa, COMMON_NAME};
use crate::site::domain::tls_error::{Situation as TlsSituation, TlsError};
use crate::site::domain::trust::is_trusted_ssl_ca;
use crate::site::domain::trust::{
    self, Moment, NextCa, Notice, PendingNotice, Stage, TrustError, Work,
};
use crate::site::ports::{LocalCaSlots, TrustStores};

/// Resultado del proceso de verificación o instalación de la CA local.
#[derive(Debug)]
pub struct TrustOutcome {
    /// Fase del ciclo de vida de la CA local.
    pub stage: Stage,
    /// Acción realizada sobre los almacenes.
    pub work: Work,
    /// Número de almacenes donde la CA local es de confianza.
    pub trusted: usize,
    /// Almacenes donde no se pudo registrar la CA local y sus errores.
    pub missed: Vec<(PathBuf, TrustError)>,
    /// Aviso pendiente para la persona usuaria.
    pub notice: PendingNotice,
}

impl TrustOutcome {
    /// Indica si la CA local no está presente en ningún almacén revisado.
    pub fn nowhere(&self) -> bool {
        self.looked() && self.trusted == 0
    }

    /// Indica si se llegó a comprobar algún almacén.
    pub fn looked(&self) -> bool {
        !matches!(self.work, Work::Nothing)
    }
}

/// Genera los mensajes descriptivos del resultado de confianza para registro.
pub fn narrate_startup_outcome(mut outcome: TrustOutcome, profiles: &[PathBuf]) -> Vec<String> {
    let mut lines = Vec::new();

    if outcome.nowhere() {
        lines.push(if profiles.is_empty() {
            "rfirma: no se ha encontrado ningún almacén NSS; ninguna sede va \
             a poder abrir el canal local"
                .to_string()
        } else {
            "rfirma: la CA local no ha entrado en ninguno de los almacenes \
             NSS encontrados; ninguna sede va a poder abrir el canal local"
                .to_string()
        });
    }

    match outcome.notice.when_the_errand_ends() {
        Some(Notice::RestartTheBrowser) => {
            lines.push("rfirma: se ha instalado la CA local; reinicia el navegador".to_string());
        }
        None => {}
    }

    if !outcome.missed.is_empty() {
        let detalle = outcome
            .missed
            .iter()
            .map(|(profile, error)| format!("{} ({error})", profile.display()))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "rfirma: la CA local no ha entrado en {} almacén(es) NSS: {detalle}",
            outcome.missed.len()
        ));
    }

    lines
}

/// Registra y renueva la CA local en los almacenes NSS indicados (ADR-0005).
pub fn refresh_local_ca_trust(
    store: &dyn LocalCaSlots,
    profiles: &[PathBuf],
    stores: &dyn TrustStores,
    moment: Moment,
) -> Result<TrustOutcome, TlsError> {
    let saved = store.serving()?;
    let waiting = store.next()?;
    let days_left = saved.as_ref().map(LocalCa::days_left).transpose()?;
    let stage = Stage::of(days_left);
    let work = trust::work_at(
        moment,
        stage,
        if waiting.is_some() {
            NextCa::Waiting
        } else {
            NextCa::None
        },
    );

    let serving = || saved.clone().expect("esa etapa sale de una CA guardada");
    let certificates: Vec<LocalCa> = match work {
        Work::Nothing => {
            return Ok(TrustOutcome {
                stage,
                work,
                trusted: 0,
                missed: Vec::new(),
                notice: PendingNotice::none(),
            })
        }
        Work::InstallTheOneWeHave => vec![serving()],
        Work::MakeOneAndInstallIt => {
            let fresh = LocalCa::generate()?;
            store.write_serving(&fresh)?;
            store.forget_next()?;
            vec![fresh]
        }
        Work::MakeTheNextAndInstallItToo => {
            let next = LocalCa::generate()?;
            store.write_next(&next)?;
            vec![serving(), next]
        }
        Work::InstallBothOfThem => vec![
            serving(),
            waiting.expect("esta rama sale de una siguiente esperando"),
        ],
        Work::PromoteTheNextOne => vec![store
            .promote_next()?
            .expect("esta rama sale de una siguiente esperando")],
    };

    let ders = certificates
        .iter()
        .map(|ca| {
            ca.certificate().to_der().map_err(|error| {
                TlsError::new(
                    TlsSituation::MaterialDamaged,
                    format!("el certificado de la CA local no sale en DER: {error}"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut trusted = 0;
    let mut installed = 0;
    let mut missed = Vec::new();

    for profile in profiles {
        match settle(stores, profile, &ders) {
            Ok(Settled::AlreadyThere) => trusted += 1,
            Ok(Settled::JustInstalled) => {
                trusted += 1;
                installed += 1;
            }
            Err(error) => missed.push((profile.clone(), error)),
        }
    }

    Ok(TrustOutcome {
        stage,
        work,
        trusted,
        missed,
        notice: if installed > 0 {
            PendingNotice::after_installing()
        } else {
            PendingNotice::none()
        },
    })
}

enum Settled {
    AlreadyThere,
    JustInstalled,
}

fn settle(
    stores: &dyn TrustStores,
    profile: &Path,
    ders: &[Vec<u8>],
) -> Result<Settled, TrustError> {
    let mut installed_any = false;
    for der in ders {
        if settle_one(stores, profile, der)? {
            installed_any = true;
        }
    }
    Ok(if installed_any {
        Settled::JustInstalled
    } else {
        Settled::AlreadyThere
    })
}

fn settle_one(stores: &dyn TrustStores, profile: &Path, der: &[u8]) -> Result<bool, TrustError> {
    if stores
        .trust_of(profile, der)?
        .is_some_and(is_trusted_ssl_ca)
    {
        return Ok(false);
    }
    stores.install(profile, der, COMMON_NAME)?;
    if !stores
        .trust_of(profile, der)?
        .is_some_and(is_trusted_ssl_ca)
    {
        return Err(TrustError::new(
            crate::site::domain::trust::Situation::TrustNotWritten,
            format!(
                "la CA local ha entrado en «{}» pero sin los bits de confianza \
                 (¿contraseña maestra en el perfil?)",
                profile.display()
            ),
        ));
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
