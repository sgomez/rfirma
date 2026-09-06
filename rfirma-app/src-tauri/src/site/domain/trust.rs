//! Registro y gestión de confianza de la CA local en almacenes NSS (ADR-0005).

pub use super::trust_error::{Situation, TrustError};

/// Días de solape previos a la caducidad para instalar la CA siguiente (ADR-0005).
pub const OVERLAP_DAYS: i64 = 120;

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
}

/// Determina la acción a realizar según el momento, etapa y existencia de CA siguiente (ADR-0005).
pub fn work_at(moment: Moment, stage: Stage, next: NextCa) -> Work {
    match (moment, stage, next) {
        (Moment::MidErrand, _, _) => Work::Nothing,
        (Moment::Startup, Stage::Serving, _) => Work::InstallTheOneWeHave,
        (Moment::Startup, Stage::Absent, _) => Work::MakeOneAndInstallIt,
        (Moment::Startup, Stage::Overlapping, NextCa::None) => Work::MakeTheNextAndInstallItToo,
        (Moment::Startup, Stage::Overlapping, NextCa::Waiting) => Work::InstallBothOfThem,
        (Moment::Startup, Stage::Expired, NextCa::Waiting) => Work::PromoteTheNextOne,
        (Moment::Startup, Stage::Expired, NextCa::None) => Work::MakeOneAndInstallIt,
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

#[cfg(test)]
mod tests;
