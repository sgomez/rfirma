//! Registro y gestión de confianza de la CA local en almacenes NSS (ADR-0005).

pub mod error;
pub mod nss;

use std::path::Path;

pub use error::{Situation, TrustError};
pub use nss::NssTrustStores;

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

/// Puerto de interacción con los almacenes NSS (ADR-0005).
pub trait TrustStores {
    /// Instala el certificado en el almacén de perfil indicado con permisos de confianza TLS.
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        nickname: &str,
    ) -> Result<(), TrustError>;

    /// Obtiene los bits de confianza TLS configurados para el certificado en el almacén.
    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_without_a_local_ca_is_the_first_boot_and_not_a_failure() {
        assert_eq!(Stage::of(None), Stage::Absent);
    }

    #[test]
    fn a_local_ca_with_years_left_is_simply_serving() {
        assert_eq!(Stage::of(Some(700)), Stage::Serving);
    }

    #[test]
    fn the_next_local_ca_goes_in_months_before_the_current_one_expires() {
        assert_eq!(Stage::of(Some(OVERLAP_DAYS)), Stage::Serving);
        assert_eq!(Stage::of(Some(OVERLAP_DAYS - 1)), Stage::Overlapping);
        assert_eq!(Stage::of(Some(1)), Stage::Overlapping);
    }

    #[test]
    fn a_local_ca_that_ran_out_is_expired_and_not_overlapping() {
        assert_eq!(Stage::of(Some(0)), Stage::Expired);
        assert_eq!(Stage::of(Some(-40)), Stage::Expired);
    }

    #[test]
    fn nothing_is_ever_repaired_in_the_middle_of_an_errand() {
        for stage in [
            Stage::Absent,
            Stage::Serving,
            Stage::Overlapping,
            Stage::Expired,
        ] {
            for next in [NextCa::None, NextCa::Waiting] {
                assert_eq!(work_at(Moment::MidErrand, stage, next), Work::Nothing);
            }
        }
    }

    #[test]
    fn the_first_boot_makes_a_local_ca_and_so_does_an_expired_one_with_no_successor() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Absent, NextCa::None),
            Work::MakeOneAndInstallIt
        );
        assert_eq!(
            work_at(Moment::Startup, Stage::Expired, NextCa::None),
            Work::MakeOneAndInstallIt
        );
    }

    #[test]
    fn a_local_ca_that_still_serves_is_installed_but_never_remade() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Serving, NextCa::None),
            Work::InstallTheOneWeHave
        );
    }

    #[test]
    fn the_overlap_installs_the_next_one_without_asking_for_the_current_one_back() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Overlapping, NextCa::None),
            Work::MakeTheNextAndInstallItToo
        );
    }

    #[test]
    fn the_next_local_ca_is_made_once_and_then_only_installed() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Overlapping, NextCa::Waiting),
            Work::InstallBothOfThem
        );
    }

    #[test]
    fn an_expired_local_ca_with_a_successor_waiting_hands_over_instead_of_starting_again() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Expired, NextCa::Waiting),
            Work::PromoteTheNextOne
        );
    }

    #[test]
    fn the_notice_never_shows_up_in_the_middle_of_an_errand() {
        let pending = PendingNotice::after_installing();

        assert_eq!(pending.mid_errand(), None);
        assert!(pending.is_pending());
    }

    #[test]
    fn the_notice_comes_out_once_when_the_errand_ends() {
        let mut pending = PendingNotice::after_installing();

        assert_eq!(
            pending.when_the_errand_ends(),
            Some(Notice::RestartTheBrowser)
        );
        assert_eq!(pending.when_the_errand_ends(), None);
    }

    #[test]
    fn nothing_installed_means_nothing_to_say() {
        let mut pending = PendingNotice::none();

        assert!(!pending.is_pending());
        assert_eq!(pending.when_the_errand_ends(), None);
    }
}
