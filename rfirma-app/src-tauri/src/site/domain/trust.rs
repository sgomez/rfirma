//! Las reglas puras de la confianza en la CA local: en qué etapa de su vida está, qué trabajo toca en cada momento y qué aviso queda pendiente (ADR-0005).

pub use super::trust_error::{Situation, TrustError};

use super::local_ca::{is_an_rfirma_ca, ChannelMark};

/// Días de solape previos a la caducidad para instalar la CA siguiente (ADR-0005).
pub const OVERLAP_DAYS: i64 = 120;

/// Días mínimos de validez de la CA local para atender un trámite de sede (ADR-0005).
pub const SITE_MINIMUM_DAYS: i64 = 7;

/// Indica si la CA local bloquea el trámite de sede: falta, o le quedan menos de siete días.
pub fn blocks_the_site(days_left: Option<i64>) -> bool {
    match days_left {
        None => true,
        Some(days) => days < SITE_MINIMUM_DAYS,
    }
}

/// Estado del ciclo de vida de la CA local guardada (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// No hay CA local guardada.
    Absent,
    /// CA local vigente con validez suficiente.
    Serving,
    /// CA local en periodo de solape previo a caducar.
    Overlapping,
    /// CA local caducada.
    Expired,
}

impl Stage {
    /// Determina la etapa a partir de los días restantes de validez.
    pub fn of(days_left: Option<i64>) -> Self {
        match days_left {
            None => Stage::Absent,
            Some(days) if days <= 0 => Stage::Expired,
            Some(days) if days < OVERLAP_DAYS => Stage::Overlapping,
            Some(_) => Stage::Serving,
        }
    }
}

/// Momento en el que se evalúa el estado de confianza (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// Arranque de la aplicación.
    Startup,
    /// Trámite de sede en curso.
    MidErrand,
    /// Instalación desde la ventana de sede, con un canal firmado por la CA vigente.
    ChannelServing,
}

/// Estado de existencia de la CA local siguiente en el almacén.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextCa {
    /// No hay CA siguiente fabricada.
    None,
    /// Hay una CA siguiente esperando relevo.
    Waiting,
}

/// Acción a ejecutar sobre los almacenes NSS (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    /// No realizar ninguna acción.
    Nothing,
    /// Registrar la CA existente en los perfiles donde falte.
    InstallTheOneWeHave,
    /// Fabricar una CA local e instalarla.
    MakeOneAndInstallIt,
    /// Fabricar la CA siguiente e instalarla manteniendo la vigente.
    MakeTheNextAndInstallItToo,
    /// Registrar tanto la CA vigente como la siguiente.
    InstallBothOfThem,
    /// Promover la CA siguiente a vigente sin fabricar material nuevo.
    PromoteTheNextOne,
    /// Fabricar una CA con la marca del canal que sustituye a la vigente sin marca y retirar esta.
    ReplaceTheUnmarkedOne,
}

/// Determina la acción a realizar según el momento, etapa y existencia de CA siguiente (ADR-0005).
pub fn work_at(moment: Moment, stage: Stage, next: NextCa) -> Work {
    match (moment, stage, next) {
        (Moment::MidErrand, _, _) => Work::Nothing,
        (_, Stage::Serving, _) => Work::InstallTheOneWeHave,
        (_, Stage::Absent, _) => Work::MakeOneAndInstallIt,
        (_, Stage::Overlapping, NextCa::None) => Work::MakeTheNextAndInstallItToo,
        (_, Stage::Overlapping, NextCa::Waiting) => Work::InstallBothOfThem,
        (_, Stage::Expired, NextCa::Waiting) => Work::PromoteTheNextOne,
        (_, Stage::Expired, NextCa::None) => Work::MakeOneAndInstallIt,
    }
}

/// El trabajo cuando la CA vigente no lleva marca de canal: si toca escribir, se sustituye (ADR-0005).
pub fn replacing_the_unmarked(work: Work) -> Work {
    match work {
        Work::Nothing => Work::Nothing,
        _ => Work::ReplaceTheUnmarkedOne,
    }
}

/// Lo que se le dice a la persona cuando se ha tocado un almacén NSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    /// Indicación de reiniciar el navegador.
    RestartTheBrowser,
}

/// Aviso diferido que espera a la finalización del trámite (ADR-0005).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingNotice(Option<Notice>);

impl PendingNotice {
    /// Construye un estado sin aviso pendiente.
    pub fn none() -> Self {
        Self(None)
    }

    /// Registra el aviso tras la instalación de certificados.
    pub fn after_installing() -> Self {
        Self(Some(Notice::RestartTheBrowser))
    }

    /// Consulta el aviso durante el trámite.
    pub fn mid_errand(&self) -> Option<Notice> {
        None
    }

    /// Extrae el aviso pendiente al finalizar el trámite.
    pub fn when_the_errand_ends(&mut self) -> Option<Notice> {
        self.0.take()
    }

    /// Comprueba si hay un aviso pendiente.
    pub fn is_pending(&self) -> bool {
        self.0.is_some()
    }
}

const CERTDB_VALID_CA: u32 = 0x0008;
const CERTDB_TRUSTED_CA: u32 = 0x0010;

/// Bits que identifican una CA de confianza para TLS en NSS.
pub const TRUSTED_SSL_CA: u32 = CERTDB_VALID_CA | CERTDB_TRUSTED_CA;

/// Comprueba si los bits corresponden a una CA de confianza para TLS.
pub fn is_trusted_ssl_ca(flags: u32) -> bool {
    flags & TRUSTED_SSL_CA == TRUSTED_SSL_CA
}

/// Las CA de rFirma de un almacén que instalar retira: las de su canal o sin marca que no se conservan; en Windows, ninguna (ADR-0005).
pub fn orphaned_local_cas(
    found: Vec<Vec<u8>>,
    kept: &[Vec<u8>],
    mark: ChannelMark,
) -> Vec<Vec<u8>> {
    if mark == ChannelMark::Windows {
        return Vec::new();
    }
    found
        .into_iter()
        .filter(|der| is_an_rfirma_ca(der) && !kept.contains(der))
        .filter(|der| ChannelMark::of_certificate(der).is_none_or(|theirs| theirs == mark))
        .collect()
}

#[cfg(test)]
mod tests;
