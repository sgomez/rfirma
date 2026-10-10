//! Cuándo se instala la CA local en los almacenes de confianza, cómo se solapa con la siguiente y cómo se retira de todos ellos (ADR-0005).

use std::path::{Path, PathBuf};

use crate::desktop::domain::channel::Channel;
use crate::site::domain::local_ca::{ChannelMark, LocalCa, COMMON_NAME};
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

/// La marca que lleva la CA local de cada canal de distribución.
pub fn mark_of(channel: Channel) -> ChannelMark {
    match channel {
        Channel::Native => ChannelMark::Native,
        Channel::Flatpak => ChannelMark::Flatpak,
        Channel::Windows => ChannelMark::Windows,
    }
}

/// Registra y renueva la CA local en los almacenes NSS indicados (ADR-0005).
pub fn refresh_local_ca_trust(
    store: &dyn LocalCaSlots,
    profiles: &[PathBuf],
    stores: &dyn TrustStores,
    moment: Moment,
    mark: ChannelMark,
) -> Result<TrustOutcome, TlsError> {
    let saved = store.serving()?;
    let waiting = store.next()?;
    let days_left = saved.as_ref().map(LocalCa::days_left).transpose()?;
    let stage = Stage::of(days_left);
    let work = work_for(moment, stage, saved.as_ref(), waiting.is_some(), mark);
    let retired = if work == Work::ReplaceTheUnmarkedOne {
        [saved.as_ref(), waiting.as_ref()]
            .into_iter()
            .flatten()
            .map(der_of)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

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
        Work::MakeOneAndInstallIt | Work::ReplaceTheUnmarkedOne => {
            let fresh = LocalCa::generate(mark)?;
            store.write_serving(&fresh)?;
            store.forget_next()?;
            vec![fresh]
        }
        Work::MakeTheNextAndInstallItToo => {
            let next = LocalCa::generate(mark)?;
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
        .map(der_of)
        .collect::<Result<Vec<_>, _>>()?;
    let kept = [store.serving()?, store.next()?]
        .iter()
        .flatten()
        .map(der_of)
        .collect::<Result<Vec<_>, _>>()?;

    let tally = install_everywhere(
        stores,
        profiles,
        &Installation {
            ders: &ders,
            retired: &retired,
            kept: &kept,
            mark,
        },
    );

    Ok(TrustOutcome {
        stage,
        work,
        trusted: tally.trusted,
        missed: tally.missed,
        notice: if tally.installed > 0 {
            PendingNotice::after_installing()
        } else {
            PendingNotice::none()
        },
    })
}

struct Installation<'a> {
    ders: &'a [Vec<u8>],
    retired: &'a [Vec<u8>],
    kept: &'a [Vec<u8>],
    mark: ChannelMark,
}

#[derive(Default)]
struct Tally {
    trusted: usize,
    installed: usize,
    missed: Vec<(PathBuf, TrustError)>,
}

fn install_everywhere(
    stores: &dyn TrustStores,
    profiles: &[PathBuf],
    installation: &Installation<'_>,
) -> Tally {
    let mut tally = Tally::default();
    for profile in profiles {
        match settle(stores, profile, installation.ders).and_then(|settled| {
            retire(stores, profile, installation.retired)?;
            Ok(settled)
        }) {
            Ok(Settled::AlreadyThere) => tally.trusted += 1,
            Ok(Settled::JustInstalled) => {
                tally.trusted += 1;
                tally.installed += 1;
            }
            Err(error) => {
                tally.missed.push((profile.clone(), error));
                continue;
            }
        }
        if let Err(error) = sweep_orphans(stores, profile, installation.kept, installation.mark) {
            tally.missed.push((profile.clone(), error));
        }
    }
    tally
}

fn sweep_orphans(
    stores: &dyn TrustStores,
    profile: &Path,
    kept: &[Vec<u8>],
    mark: ChannelMark,
) -> Result<(), TrustError> {
    let found = stores.local_cas(profile)?;
    retire(
        stores,
        profile,
        &trust::orphaned_local_cas(found, kept, mark),
    )
}

fn work_for(
    moment: Moment,
    stage: Stage,
    saved: Option<&LocalCa>,
    waiting: bool,
    mark: ChannelMark,
) -> Work {
    let next = if waiting {
        NextCa::Waiting
    } else {
        NextCa::None
    };
    let work = trust::work_at(moment, stage, next);
    let unmarked = saved.is_some_and(|ca| ca.mark().is_none());
    if unmarked && mark.replaces_an_unmarked_local_ca() {
        trust::replacing_the_unmarked(work)
    } else {
        work
    }
}

enum Settled {
    AlreadyThere,
    JustInstalled,
}

fn der_of(ca: &LocalCa) -> Result<Vec<u8>, TlsError> {
    ca.certificate().to_der().map_err(|error| {
        TlsError::new(
            TlsSituation::MaterialDamaged,
            format!("el certificado de la CA local no sale en DER: {error}"),
        )
    })
}

fn retire(stores: &dyn TrustStores, profile: &Path, ders: &[Vec<u8>]) -> Result<(), TrustError> {
    for der in ders {
        stores.withdraw(profile, der)?;
    }
    Ok(())
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

/// Si la CA local vigente es de confianza en un perfil, medido sin escribir (ADR-0005).
pub struct ProfileTrust {
    /// Perfil NSS medido.
    pub profile: PathBuf,
    /// Si la CA local vigente tiene los bits de confianza TLS en ese perfil.
    pub trusted: bool,
}

/// Mide, sin escribir, si la CA local vigente es de confianza en cada perfil.
pub fn measure_local_ca_trust(
    store: &dyn LocalCaSlots,
    profiles: &[PathBuf],
    stores: &dyn TrustStores,
) -> Result<Vec<ProfileTrust>, TlsError> {
    let Some(serving) = store.serving()? else {
        return Ok(profiles
            .iter()
            .map(|profile| ProfileTrust {
                profile: profile.clone(),
                trusted: false,
            })
            .collect());
    };
    let der = serving.certificate().to_der().map_err(|error| {
        TlsError::new(
            TlsSituation::MaterialDamaged,
            format!("el certificado de la CA local no sale en DER: {error}"),
        )
    })?;
    Ok(profiles
        .iter()
        .map(|profile| ProfileTrust {
            profile: profile.clone(),
            trusted: stores
                .trust_of(profile, &der)
                .ok()
                .flatten()
                .is_some_and(is_trusted_ssl_ca),
        })
        .collect())
}

/// Qué pasó al retirar la CA local de un almacén NSS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreWithdrawal {
    /// La CA local estaba y se ha retirado.
    Withdrawn,
    /// La CA local no estaba en este almacén.
    WasNotThere,
    /// No se ha podido retirar.
    Failed(TrustError),
}

impl StoreWithdrawal {
    /// Si la CA local estaba en este almacén y se ha retirado.
    pub fn withdrawn(&self) -> bool {
        matches!(self, Self::Withdrawn)
    }

    /// El motivo, ya traducido a texto, por el que no se ha podido retirar.
    pub fn failure(&self) -> Option<String> {
        match self {
            Self::Failed(error) => Some(error.to_string()),
            Self::Withdrawn | Self::WasNotThere => None,
        }
    }
}

/// Resultado de retirar la CA local de cada almacén NSS.
#[derive(Debug)]
pub struct WithdrawOutcome {
    /// Resultado por perfil.
    pub results: Vec<(PathBuf, StoreWithdrawal)>,
}

/// Retira la CA local —vigente y la del solape— de los almacenes NSS indicados, por huella.
/// Las ranuras solo se vacían después, y solo si ningún almacén ha fallado.
pub fn withdraw_everywhere(
    store: &dyn LocalCaSlots,
    profiles: &[PathBuf],
    stores: &dyn TrustStores,
) -> Result<WithdrawOutcome, TlsError> {
    let ders = [store.serving()?, store.next()?]
        .into_iter()
        .flatten()
        .map(|ca| {
            ca.certificate().to_der().map_err(|error| {
                TlsError::new(
                    TlsSituation::MaterialDamaged,
                    format!("el certificado de la CA local no sale en DER: {error}"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if ders.is_empty() {
        return Ok(WithdrawOutcome {
            results: Vec::new(),
        });
    }

    let results: Vec<(PathBuf, StoreWithdrawal)> = profiles
        .iter()
        .map(|profile| (profile.clone(), withdraw_one(stores, profile, &ders)))
        .collect();

    if !results
        .iter()
        .any(|(_, outcome)| matches!(outcome, StoreWithdrawal::Failed(_)))
    {
        store.forget_next()?;
        store.forget_serving()?;
    }

    Ok(WithdrawOutcome { results })
}

fn withdraw_one(stores: &dyn TrustStores, profile: &Path, ders: &[Vec<u8>]) -> StoreWithdrawal {
    let mut was_there = false;
    for der in ders {
        match stores.trust_of(profile, der) {
            Ok(Some(_)) => was_there = true,
            Ok(None) => {}
            Err(error) => return StoreWithdrawal::Failed(error),
        }
        if let Err(error) = stores.withdraw(profile, der) {
            return StoreWithdrawal::Failed(error);
        }
    }
    if was_there {
        StoreWithdrawal::Withdrawn
    } else {
        StoreWithdrawal::WasNotThere
    }
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
